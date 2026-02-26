#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::parser::ast::{
    Annotation, Block, ClassDecl, Expr, FieldDecl, MethodDecl, SourceFile, Span, Stmt, StructDecl,
    TopLevelItem, TypeExpr,
};

/// Canonical semantic types used during type checking.
///
/// Purpose:
/// - decouple parser syntax (`TypeExpr`) from semantic reasoning (`Type`)
/// - normalize equivalent representations into one internal model
///
/// Why this is needed:
/// - type inference, compatibility checks, and serializability rules operate on
///   semantic categories, not parser surface syntax
/// - later phases (IR/codegen) need stable, resolved type shapes
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Null,
    Any,
    Named(String),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Result(Box<Type>, Box<Type>),
    Option(Box<Type>),
    Function(Vec<Type>, Box<Type>),
    Unknown,
}

/// All compile-time type errors emitted by the type checker.
///
/// Purpose:
/// - provide structured, phase-specific diagnostics
///
/// Why this is needed:
/// - keeps type-checking failures explicit and machine-readable
/// - maps directly to error-reporting phase output
#[derive(Debug, Error, Clone, PartialEq)]
pub enum TypeError {
    #[error("missing return type annotation on method '{method}'")]
    MissingReturnType { method: String, span: Span },

    #[error("untyped DTO field '{field}' in '{class}'")]
    UntypedDtoField {
        class: String,
        field: String,
        span: Span,
    },

    #[error("non-serializable return type '{ty}'")]
    NonSerializableReturn { ty: String, span: Span },

    #[error("type mismatch: expected '{expected}', found '{found}'")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("incompatible types '{lhs}' and '{rhs}'")]
    IncompatibleTypes {
        lhs: String,
        rhs: String,
        span: Span,
    },
}

/// Type-checker state for one source-file analysis pass.
///
/// Purpose:
/// - cache declarations needed for lookup/inference
/// - collect errors without failing fast
///
/// Why this is needed:
/// - we must report as many useful type issues as possible in one run
/// - controller/DTO checks require cross-item lookups
#[derive(Debug, Default)]
pub struct TypeChecker {
    classes: HashMap<String, ClassDecl>,
    structs: HashMap<String, StructDecl>,
    controllers: HashSet<String>,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    /// Creates a fresh checker with empty caches and no accumulated errors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs full type validation over a parsed source file.
    ///
    /// Purpose:
    /// - perform method-level checks, controller contract checks, and DTO checks
    ///
    /// Why this is needed:
    /// - central entrypoint used by CLI/compiler pipeline for phase 4
    /// - returns all discovered errors together instead of stopping early
    pub fn check_source_file(&mut self, ast: &SourceFile) -> Result<(), Vec<TypeError>> {
        self.preload_declarations(ast);

        for item in &ast.items {
            if let TopLevelItem::Class(class) = item {
                self.check_class(class);
            }
        }

        self.check_dto_fields_used_by_controllers();

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Preloads class/struct declarations and identifies controller classes.
    ///
    /// Purpose:
    /// - build lookup tables before method/body analysis
    ///
    /// Why this is needed:
    /// - inference and serializability checks need O(1) access to declared types
    fn preload_declarations(&mut self, ast: &SourceFile) {
        self.classes.clear();
        self.structs.clear();
        self.controllers.clear();

        for item in &ast.items {
            match item {
                TopLevelItem::Class(class) => {
                    if self.has_annotation(&class.annotations, "controller") {
                        self.controllers.insert(class.name.clone());
                    }
                    self.classes.insert(class.name.clone(), class.clone());
                }
                TopLevelItem::Struct(struct_decl) => {
                    self.structs
                        .insert(struct_decl.name.clone(), struct_decl.clone());
                }
                _ => {}
            }
        }
    }

    /// Validates all methods in a class with class-level context.
    ///
    /// Why this is needed:
    /// - method validation depends on whether the class is a controller
    fn check_class(&mut self, class: &ClassDecl) {
        let is_controller = self.controllers.contains(&class.name);
        for method in &class.methods {
            self.check_method(class, method, is_controller);
        }
    }

    /// Validates one method signature and body.
    ///
    /// Purpose:
    /// - enforce declared return rules
    /// - type-check statements against method contract
    ///
    /// Why this is needed:
    /// - method boundaries are the core unit for return/parameter correctness
    fn check_method(&mut self, class: &ClassDecl, method: &MethodDecl, is_controller: bool) {
        if Self::is_missing_return_type(&method.return_type) {
            self.errors.push(TypeError::MissingReturnType {
                method: method.name.clone(),
                span: method.span.clone(),
            });
            return;
        }

        let declared_return = Self::from_type_expr(&method.return_type);

        if is_controller {
            self.check_controller_handler_signature(method, &declared_return);
        }

        let mut env = self.build_method_env(class, method);
        self.check_block(&method.body, &declared_return, &mut env);
    }

    /// Builds the local type environment for method-body checking.
    ///
    /// Why this is needed:
    /// - identifiers must resolve to known types during expression inference
    /// - method scope includes both fields and parameters
    fn build_method_env(&self, class: &ClassDecl, method: &MethodDecl) -> HashMap<String, Type> {
        let mut env = HashMap::new();

        for field in &class.fields {
            env.insert(field.name.clone(), Self::from_type_expr(&field.ty));
        }

        for param in &method.params {
            env.insert(param.name.clone(), Self::from_type_expr(&param.ty));
        }

        env
    }

    /// Enforces controller handler return contract: `Result<T, HttpError>`.
    ///
    /// Why this is needed:
    /// - ArwaLang guarantees framework handler semantics at compile time
    /// - response payload type must be serializable for runtime JSON handling
    fn check_controller_handler_signature(&mut self, method: &MethodDecl, return_ty: &Type) {
        let Type::Result(ok, err) = return_ty else {
            self.errors.push(TypeError::TypeMismatch {
                expected: "Result<T, HttpError>".to_string(),
                found: return_ty.to_string(),
                span: method.span.clone(),
            });
            return;
        };

        if **err != Type::Named("HttpError".to_string()) {
            self.errors.push(TypeError::TypeMismatch {
                expected: "Result<T, HttpError>".to_string(),
                found: return_ty.to_string(),
                span: method.span.clone(),
            });
        }

        if !self.is_serializable(ok) {
            self.errors.push(TypeError::NonSerializableReturn {
                ty: ok.to_string(),
                span: method.span.clone(),
            });
        }
    }

    /// Checks all statements inside a block against expected method return type.
    fn check_block(
        &mut self,
        block: &Block,
        expected_return: &Type,
        env: &mut HashMap<String, Type>,
    ) {
        for stmt in &block.stmts {
            self.check_stmt(stmt, expected_return, env);
        }
    }

    /// Type-checks one statement and updates local environment when needed.
    ///
    /// Why this is needed:
    /// - `let` introduces new bindings
    /// - `return` must match declared method return
    /// - control-flow branches must remain type-safe
    fn check_stmt(&mut self, stmt: &Stmt, expected_return: &Type, env: &mut HashMap<String, Type>) {
        match stmt {
            Stmt::Let { name, ty, value } => {
                let inferred = self.infer_expr(value, env);
                if let Some(annotated) = ty {
                    let declared = Self::from_type_expr(annotated);
                    if !self.is_compatible(&declared, &inferred) {
                        self.errors.push(TypeError::TypeMismatch {
                            expected: declared.to_string(),
                            found: inferred.to_string(),
                            span: Self::stmt_span(stmt),
                        });
                    }
                    env.insert(name.clone(), declared);
                } else {
                    env.insert(name.clone(), inferred);
                }
            }
            Stmt::Return(expr) => {
                let actual = if let Some(expr) = expr {
                    self.infer_expr(expr, env)
                } else {
                    Type::Null
                };

                if !self.is_compatible(expected_return, &actual) {
                    self.errors.push(TypeError::TypeMismatch {
                        expected: expected_return.to_string(),
                        found: actual.to_string(),
                        span: Self::stmt_span(stmt),
                    });
                }
            }
            Stmt::Expr(expr) => {
                let _ = self.infer_expr(expr, env);
            }
            Stmt::If { cond, then, else_ } => {
                let cond_ty = self.infer_expr(cond, env);
                if !self.is_compatible(&Type::Bool, &cond_ty) {
                    self.errors.push(TypeError::TypeMismatch {
                        expected: Type::Bool.to_string(),
                        found: cond_ty.to_string(),
                        span: Self::stmt_span(stmt),
                    });
                }

                let mut then_env = env.clone();
                self.check_block(then, expected_return, &mut then_env);

                if let Some(else_block) = else_ {
                    let mut else_env = env.clone();
                    self.check_block(else_block, expected_return, &mut else_env);
                }
            }
        }
    }

    /// Infers expression type from local scope and known declarations.
    ///
    /// Why this is needed:
    /// - statement validation relies on inferred RHS/return/condition types
    /// - later phases can reuse inferred semantics if we persist them
    pub fn infer_expr(&self, expr: &Expr, env: &HashMap<String, Type>) -> Type {
        match expr {
            Expr::IntLit(_) => Type::Int,
            Expr::FloatLit(_) => Type::Float,
            Expr::StringLit(_) => Type::String,
            Expr::BoolLit(_) => Type::Bool,
            Expr::Null => Type::Null,
            Expr::Ident(name) => env.get(name).cloned().unwrap_or(Type::Unknown),
            Expr::Call { callee, args } => self.infer_call_expr(callee, args, env),
            Expr::FieldAccess { obj, field } => self.infer_field_access(obj, field, env),
            Expr::UnaryOp { op, expr } => {
                let inner = self.infer_expr(expr, env);
                match op {
                    crate::parser::ast::UnaryOp::Neg => {
                        if inner == Type::Int || inner == Type::Float {
                            inner
                        } else {
                            Type::Unknown
                        }
                    }
                    crate::parser::ast::UnaryOp::Not => Type::Bool,
                }
            }
            Expr::BinOp { op, lhs, rhs } => {
                let left = self.infer_expr(lhs, env);
                let right = self.infer_expr(rhs, env);
                match op {
                    crate::parser::ast::BinOp::Add
                    | crate::parser::ast::BinOp::Sub
                    | crate::parser::ast::BinOp::Mul
                    | crate::parser::ast::BinOp::Div => {
                        if left == Type::Float || right == Type::Float {
                            Type::Float
                        } else if left == Type::Int && right == Type::Int {
                            Type::Int
                        } else {
                            Type::Unknown
                        }
                    }
                    crate::parser::ast::BinOp::EqEq
                    | crate::parser::ast::BinOp::BangEq
                    | crate::parser::ast::BinOp::Lt
                    | crate::parser::ast::BinOp::Gt
                    | crate::parser::ast::BinOp::LtEq
                    | crate::parser::ast::BinOp::GtEq
                    | crate::parser::ast::BinOp::And
                    | crate::parser::ast::BinOp::Or => Type::Bool,
                }
            }
        }
    }

    /// Infers call expression type and validates argument compatibility.
    ///
    /// Why this is needed:
    /// - method invocations are a major source of type mismatches
    fn infer_call_expr(&self, callee: &Expr, args: &[Expr], env: &HashMap<String, Type>) -> Type {
        if let Expr::FieldAccess { obj, field } = callee {
            let obj_ty = self.infer_expr(obj, env);
            if let Type::Named(class_name) = obj_ty {
                if let Some(class) = self.classes.get(&class_name) {
                    if let Some(method) = class.methods.iter().find(|m| m.name == *field) {
                        for (idx, arg_expr) in args.iter().enumerate() {
                            if let Some(param) = method.params.get(idx) {
                                let expected = Self::from_type_expr(&param.ty);
                                let actual = self.infer_expr(arg_expr, env);
                                if !self.is_compatible(&expected, &actual) {
                                    return Type::Unknown;
                                }
                            }
                        }
                        return Self::from_type_expr(&method.return_type);
                    }
                }
            }
        }

        Type::Any
    }

    /// Resolves and infers field access type (`obj.field`).
    ///
    /// Why this is needed:
    /// - object property access must map to declared field types
    fn infer_field_access(&self, obj: &Expr, field: &str, env: &HashMap<String, Type>) -> Type {
        let obj_ty = self.infer_expr(obj, env);
        let Type::Named(name) = obj_ty else {
            return Type::Unknown;
        };

        if let Some(class) = self.classes.get(&name) {
            if let Some(found) = class.fields.iter().find(|f| f.name == field) {
                return Self::from_type_expr(&found.ty);
            }
        }

        if let Some(struct_decl) = self.structs.get(&name) {
            if let Some(found) = struct_decl.fields.iter().find(|f| f.name == field) {
                return Self::from_type_expr(&found.ty);
            }
        }

        Type::Unknown
    }

    /// Collects DTO types used by controllers and validates their field typing.
    ///
    /// Why this is needed:
    /// - DTO shape validity is a compile-time guarantee for request/response payloads
    fn check_dto_fields_used_by_controllers(&mut self) {
        let mut dto_names = HashSet::new();

        for class_name in &self.controllers {
            let Some(class) = self.classes.get(class_name) else {
                continue;
            };

            for method in &class.methods {
                for param in &method.params {
                    if self.has_annotation(&param.annotations, "body") {
                        if let Some(name) = Self::extract_named_type(&param.ty) {
                            dto_names.insert(name);
                        }
                    }
                }

                let return_ty = Self::from_type_expr(&method.return_type);
                if let Type::Result(ok, _) = return_ty {
                    if let Some(name) = Self::extract_named_type_from_type(&ok) {
                        dto_names.insert(name);
                    }
                }
            }
        }

        for name in dto_names {
            if let Some(class) = self.classes.get(&name).cloned() {
                self.check_typed_fields(&name, &class.fields);
                continue;
            }

            if let Some(struct_decl) = self.structs.get(&name).cloned() {
                self.check_typed_fields(&name, &struct_decl.fields);
            }
        }
    }

    /// Ensures each DTO field has explicit type information.
    ///
    /// Why this is needed:
    /// - untyped DTO fields break serialization and schema expectations
    fn check_typed_fields(&mut self, owner: &str, fields: &[FieldDecl]) {
        for field in fields {
            if Self::is_missing_return_type(&field.ty) {
                self.errors.push(TypeError::UntypedDtoField {
                    class: owner.to_string(),
                    field: field.name.clone(),
                    span: field.span.clone(),
                });
            }
        }
    }

    /// Public wrapper for serializability checks.
    ///
    /// Why this is needed:
    /// - isolates recursion state handling from callers
    fn is_serializable(&self, ty: &Type) -> bool {
        self.is_serializable_inner(ty, &mut HashSet::new())
    }

    /// Recursive serializability checker with cycle protection.
    ///
    /// Why this is needed:
    /// - nested DTO graphs require deep validation
    /// - seen-set avoids infinite recursion on self-referential types
    fn is_serializable_inner(&self, ty: &Type, seen: &mut HashSet<String>) -> bool {
        match ty {
            Type::Int | Type::Float | Type::Bool | Type::String => true,
            Type::List(inner) => self.is_serializable_inner(inner, seen),
            Type::Map(key, value) => {
                **key == Type::String && self.is_serializable_inner(value, seen)
            }
            Type::Option(inner) => self.is_serializable_inner(inner, seen),
            Type::Named(name) => {
                if seen.contains(name) {
                    return true;
                }

                seen.insert(name.clone());

                if let Some(class) = self.classes.get(name) {
                    return class
                        .fields
                        .iter()
                        .all(|f| self.is_serializable_inner(&Self::from_type_expr(&f.ty), seen));
                }

                if let Some(struct_decl) = self.structs.get(name) {
                    return struct_decl
                        .fields
                        .iter()
                        .all(|f| self.is_serializable_inner(&Self::from_type_expr(&f.ty), seen));
                }

                false
            }
            Type::Any | Type::Null | Type::Result(_, _) | Type::Function(_, _) | Type::Unknown => {
                false
            }
        }
    }

    /// Determines whether `actual` can be used where `expected` is required.
    ///
    /// Why this is needed:
    /// - foundational rule used by assignment, return, and argument checks
    fn is_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if expected == &Type::Any || actual == &Type::Any {
            return true;
        }

        match (expected, actual) {
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String)
            | (Type::Null, Type::Null)
            | (Type::Unknown, Type::Unknown) => true,
            (Type::Named(a), Type::Named(b)) => a == b,
            (Type::List(a), Type::List(b)) => self.is_compatible(a, b),
            (Type::Map(ka, va), Type::Map(kb, vb)) => {
                self.is_compatible(ka, kb) && self.is_compatible(va, vb)
            }
            (Type::Result(ok_a, err_a), Type::Result(ok_b, err_b)) => {
                self.is_compatible(ok_a, ok_b) && self.is_compatible(err_a, err_b)
            }
            (Type::Option(a), Type::Option(b)) => self.is_compatible(a, b),
            _ => false,
        }
    }

    /// Converts parser type syntax into semantic `Type`.
    ///
    /// Why this is needed:
    /// - checker logic should operate on normalized type forms only
    fn from_type_expr(ty: &TypeExpr) -> Type {
        match ty {
            TypeExpr::Named(name) => match name.as_str() {
                "Int" => Type::Int,
                "Float" => Type::Float,
                "Bool" => Type::Bool,
                "String" => Type::String,
                "Any" => Type::Any,
                "" => Type::Unknown,
                _ => Type::Named(name.clone()),
            },
            TypeExpr::Generic { name, params } if name == "List" && params.len() == 1 => {
                Type::List(Box::new(Self::from_type_expr(&params[0])))
            }
            TypeExpr::Generic { name, params } if name == "Map" && params.len() == 2 => Type::Map(
                Box::new(Self::from_type_expr(&params[0])),
                Box::new(Self::from_type_expr(&params[1])),
            ),
            TypeExpr::Generic { name, .. } => Type::Named(name.clone()),
            TypeExpr::Result { ok, err } => Type::Result(
                Box::new(Self::from_type_expr(ok)),
                Box::new(Self::from_type_expr(err)),
            ),
            TypeExpr::Option(inner) => Type::Option(Box::new(Self::from_type_expr(inner))),
        }
    }

    /// Utility to test whether an annotation list contains a specific marker.
    fn has_annotation(&self, annotations: &[Annotation], expected: &str) -> bool {
        annotations.iter().any(|ann| ann.name == expected)
    }

    /// Temporary signal for "missing return type" in current AST model.
    ///
    /// Why this is needed:
    /// - AST currently always stores `return_type`; empty name acts as sentinel
    fn is_missing_return_type(ty: &TypeExpr) -> bool {
        matches!(ty, TypeExpr::Named(name) if name.is_empty())
    }

    /// Extracts first meaningful named type from a type expression.
    ///
    /// Why this is needed:
    /// - DTO detection needs named payload types from wrapped forms
    fn extract_named_type(ty: &TypeExpr) -> Option<String> {
        match ty {
            TypeExpr::Named(name) if !name.is_empty() => Some(name.clone()),
            TypeExpr::Generic { params, .. } => params.first().and_then(Self::extract_named_type),
            TypeExpr::Option(inner) => Self::extract_named_type(inner),
            TypeExpr::Result { ok, .. } => Self::extract_named_type(ok),
            _ => None,
        }
    }

    /// Extracts named type from semantic type forms used in return checks.
    fn extract_named_type_from_type(ty: &Type) -> Option<String> {
        match ty {
            Type::Named(name) => Some(name.clone()),
            Type::List(inner) | Type::Option(inner) => Self::extract_named_type_from_type(inner),
            _ => None,
        }
    }

    /// Placeholder statement span until AST statements carry explicit spans.
    ///
    /// Why this is needed:
    /// - keeps diagnostics shape consistent even before statement-span support lands
    fn stmt_span(stmt: &Stmt) -> Span {
        match stmt {
            Stmt::Let { .. } | Stmt::Return(_) | Stmt::Expr(_) | Stmt::If { .. } => Span {
                file: std::path::PathBuf::from("<stmt>"),
                line_start: 0,
                col_start: 0,
                line_end: 0,
                col_end: 0,
            },
        }
    }
}

impl std::fmt::Display for Type {
    /// Formats semantic types into human-readable diagnostic strings.
    ///
    /// Why this is needed:
    /// - type mismatch errors must clearly show expected vs found shapes
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Null => write!(f, "Null"),
            Type::Any => write!(f, "Any"),
            Type::Named(v) => write!(f, "{v}"),
            Type::List(inner) => write!(f, "List<{inner}>"),
            Type::Map(k, v) => write!(f, "Map<{k}, {v}>"),
            Type::Result(ok, err) => write!(f, "Result<{ok}, {err}>"),
            Type::Option(inner) => write!(f, "Option<{inner}>"),
            Type::Function(args, ret) => {
                let args = args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({args}) -> {ret}")
            }
            Type::Unknown => write!(f, "<unknown>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::parser::ast::{
        Annotation, Block, ClassDecl, Expr, MethodDecl, Param, SourceFile, Span, Stmt, StructDecl,
        TopLevelItem, TypeExpr,
    };

    use super::{Type, TypeChecker, TypeError};

    fn span() -> Span {
        Span {
            file: PathBuf::from("test.rw"),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 1,
        }
    }

    fn controller_annotation() -> Annotation {
        Annotation {
            name: "controller".to_string(),
            args: vec![],
            span: span(),
        }
    }

    fn simple_method(name: &str, return_type: TypeExpr, body: Vec<Stmt>) -> MethodDecl {
        MethodDecl {
            annotations: vec![],
            name: name.to_string(),
            params: vec![],
            return_type,
            body: Block { stmts: body },
            span: span(),
        }
    }

    #[test]
    fn infers_expression_types() {
        let checker = TypeChecker::new();
        let mut env = HashMap::new();
        env.insert("x".to_string(), Type::Int);

        assert_eq!(checker.infer_expr(&Expr::IntLit(1), &env), Type::Int);
        assert_eq!(checker.infer_expr(&Expr::BoolLit(true), &env), Type::Bool);
        assert_eq!(
            checker.infer_expr(&Expr::Ident("x".to_string()), &env),
            Type::Int
        );
    }

    #[test]
    fn reports_missing_return_type() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(ClassDecl {
                annotations: vec![],
                name: "A".to_string(),
                implements: vec![],
                constructor: None,
                methods: vec![simple_method(
                    "m",
                    TypeExpr::Named("".to_string()),
                    vec![Stmt::Return(Some(Expr::IntLit(1)))],
                )],
                fields: vec![],
                span: span(),
            })],
        };

        let mut checker = TypeChecker::new();
        let errors = checker
            .check_source_file(&ast)
            .expect_err("should fail on missing return type");

        assert!(errors
            .iter()
            .any(|e| matches!(e, TypeError::MissingReturnType { method, .. } if method == "m")));
    }

    #[test]
    fn validates_controller_handler_result_signature() {
        let controller = ClassDecl {
            annotations: vec![controller_annotation()],
            name: "UserController".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![simple_method(
                "get",
                TypeExpr::Named("Int".to_string()),
                vec![Stmt::Return(Some(Expr::IntLit(1)))],
            )],
            fields: vec![],
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller)],
        };

        let mut checker = TypeChecker::new();
        let errors = checker
            .check_source_file(&ast)
            .expect_err("controller method should require Result return type");

        assert!(errors
            .iter()
            .any(|e| matches!(e, TypeError::TypeMismatch { .. })));
    }

    #[test]
    fn validates_non_serializable_controller_return() {
        let controller = ClassDecl {
            annotations: vec![controller_annotation()],
            name: "UserController".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![simple_method(
                "get",
                TypeExpr::Result {
                    ok: Box::new(TypeExpr::Named("UnknownDto".to_string())),
                    err: Box::new(TypeExpr::Named("HttpError".to_string())),
                },
                vec![Stmt::Return(Some(Expr::Ident("x".to_string())))],
            )],
            fields: vec![],
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller)],
        };

        let mut checker = TypeChecker::new();
        let errors = checker
            .check_source_file(&ast)
            .expect_err("unknown dto should be non-serializable");

        assert!(errors
            .iter()
            .any(|e| matches!(e, TypeError::NonSerializableReturn { .. })));
    }

    #[test]
    fn validates_typed_dto_fields_for_body_params() {
        let dto = StructDecl {
            annotations: vec![],
            name: "UserDto".to_string(),
            fields: vec![crate::parser::ast::FieldDecl {
                name: "name".to_string(),
                ty: TypeExpr::Named("".to_string()),
                optional: false,
                span: span(),
            }],
            span: span(),
        };

        let controller = ClassDecl {
            annotations: vec![controller_annotation()],
            name: "UserController".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![MethodDecl {
                annotations: vec![],
                name: "create".to_string(),
                params: vec![Param {
                    annotations: vec![Annotation {
                        name: "body".to_string(),
                        args: vec![],
                        span: span(),
                    }],
                    name: "dto".to_string(),
                    ty: TypeExpr::Named("UserDto".to_string()),
                    is_private: false,
                    span: span(),
                }],
                return_type: TypeExpr::Result {
                    ok: Box::new(TypeExpr::Named("UserDto".to_string())),
                    err: Box::new(TypeExpr::Named("HttpError".to_string())),
                },
                body: Block {
                    stmts: vec![Stmt::Return(Some(Expr::Ident("dto".to_string())))],
                },
                span: span(),
            }],
            fields: vec![],
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Struct(dto), TopLevelItem::Class(controller)],
        };

        let mut checker = TypeChecker::new();
        let errors = checker
            .check_source_file(&ast)
            .expect_err("dto field should be required to be typed");

        assert!(errors
            .iter()
            .any(|e| matches!(e, TypeError::UntypedDtoField { class, .. } if class == "UserDto")));
    }

    #[test]
    fn reports_return_type_mismatch() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(ClassDecl {
                annotations: vec![],
                name: "A".to_string(),
                implements: vec![],
                constructor: None,
                methods: vec![simple_method(
                    "m",
                    TypeExpr::Named("Int".to_string()),
                    vec![Stmt::Return(Some(Expr::StringLit("bad".to_string())))],
                )],
                fields: vec![],
                span: span(),
            })],
        };

        let mut checker = TypeChecker::new();
        let errors = checker
            .check_source_file(&ast)
            .expect_err("should report mismatch");

        assert!(errors
            .iter()
            .any(|e| matches!(e, TypeError::TypeMismatch { expected, found, .. } if expected == "Int" && found == "String")));
    }

    #[test]
    fn checks_generic_compatibility_in_let() {
        let method = MethodDecl {
            annotations: vec![],
            name: "m".to_string(),
            params: vec![],
            return_type: TypeExpr::Named("Int".to_string()),
            body: Block {
                stmts: vec![
                    Stmt::Let {
                        name: "xs".to_string(),
                        ty: Some(TypeExpr::Generic {
                            name: "List".to_string(),
                            params: vec![TypeExpr::Named("Int".to_string())],
                        }),
                        value: Expr::IntLit(1),
                    },
                    Stmt::Return(Some(Expr::IntLit(0))),
                ],
            },
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(ClassDecl {
                annotations: vec![],
                name: "A".to_string(),
                implements: vec![],
                constructor: None,
                methods: vec![method],
                fields: vec![],
                span: span(),
            })],
        };

        let mut checker = TypeChecker::new();
        let errors = checker
            .check_source_file(&ast)
            .expect_err("generic mismatch should be reported");

        assert!(errors
            .iter()
            .any(|e| matches!(e, TypeError::TypeMismatch { expected, .. } if expected.starts_with("List<"))));
    }
}
