#![allow(dead_code)]

use thiserror::Error;

use crate::di::graph::DiGraphBuilder;
use crate::lifecycle::pipeline::PipelineBuilder;
use crate::parser::ast::{Expr, SourceFile, Stmt, TopLevelItem, TypeExpr};
use crate::routes::registry::RouteTableBuilder;

/// Compiler IR type model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrType {
    Void,
    Int,
    Float,
    Bool,
    String,
    Any,
    Named(String),
    List(Box<IrType>),
    Map(Box<IrType>, Box<IrType>),
    Result { ok: Box<IrType>, err: Box<IrType> },
    Option(Box<IrType>),
}

/// IR constant and symbolic value model.
#[derive(Debug, Clone, PartialEq)]
pub enum IrValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Null,
    Local(String),
    FunctionRef(String),
    Temporary(String),
}

/// IR instruction set used by early lowering.
#[derive(Debug, Clone, PartialEq)]
pub enum IrInstruction {
    Store {
        dst: String,
        value: IrValue,
    },
    Call {
        dst: Option<String>,
        callee: String,
        args: Vec<IrValue>,
    },
    Return(Option<IrValue>),
    Nop,
}

/// One IR basic block.
#[derive(Debug, Clone, PartialEq)]
pub struct IrBlock {
    pub label: String,
    pub instructions: Vec<IrInstruction>,
}

/// One IR function.
#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub blocks: Vec<IrBlock>,
}

/// One lowered nominal data shape for class/struct declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct IrStruct {
    pub name: String,
    pub fields: Vec<(String, IrType)>,
}

/// Static DI registry IR entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiRegistryEntry {
    pub token: String,
    pub factory_fn: String,
}

/// Static route-table IR entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRegistryEntry {
    pub method: String,
    pub path: String,
    pub handler_fn: String,
}

/// Static lifecycle pipeline IR entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineRegistryEntry {
    pub handler_fn: String,
    pub guards: Vec<String>,
    pub pipes: Vec<String>,
    pub interceptors: Vec<String>,
}

/// Full IR module payload.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IrModule {
    pub name: String,
    pub structs: Vec<IrStruct>,
    pub functions: Vec<IrFunction>,
    pub di_registry: Vec<DiRegistryEntry>,
    pub route_table: Vec<RouteRegistryEntry>,
    pub pipelines: Vec<PipelineRegistryEntry>,
}

/// IR-lowering errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IrError {
    #[error("missing module declaration for IR generation")]
    MissingModule,

    #[error("prerequisite phase failed while lowering IR: {phase}")]
    PrerequisiteFailed { phase: String },
}

/// AST-to-IR lowering entrypoint.
#[derive(Debug, Default)]
pub struct IrGenerator;

impl IrGenerator {
    /// Creates a new IR generator.
    pub fn new() -> Self {
        Self
    }

    /// Generates IR module from validated AST.
    pub fn generate_ir(&self, ast: &SourceFile) -> Result<IrModule, IrError> {
        let mut di_builder = DiGraphBuilder::new();
        let di_graph = di_builder
            .build(ast)
            .map_err(|_| IrError::PrerequisiteFailed {
                phase: "DI graph".to_string(),
            })?;

        let route_table =
            RouteTableBuilder::new()
                .build(ast)
                .map_err(|_| IrError::PrerequisiteFailed {
                    phase: "route table".to_string(),
                })?;

        let pipelines = PipelineBuilder::new()
            .build(&route_table, ast)
            .map_err(|_| IrError::PrerequisiteFailed {
                phase: "lifecycle pipeline".to_string(),
            })?;

        let module_name = ast
            .items
            .iter()
            .find_map(|item| match item {
                TopLevelItem::Module(module) => Some(module.name.clone()),
                _ => None,
            })
            .ok_or(IrError::MissingModule)?;

        let mut ir = IrModule {
            name: module_name,
            ..IrModule::default()
        };

        for item in &ast.items {
            let TopLevelItem::Class(class) = item else {
                continue;
            };

            ir.structs.push(IrStruct {
                name: class.name.clone(),
                fields: class
                    .fields
                    .iter()
                    .map(|field| (field.name.clone(), lower_type(&field.ty)))
                    .collect(),
            });

            for method in &class.methods {
                let mut block = IrBlock {
                    label: "entry".to_string(),
                    instructions: Vec::new(),
                };

                for stmt in &method.body.stmts {
                    lower_stmt(stmt, &mut block.instructions);
                }

                if !ends_with_return(&block.instructions) {
                    block.instructions.push(IrInstruction::Return(None));
                }

                ir.functions.push(IrFunction {
                    name: format!("{}.{}", class.name, method.name),
                    params: method
                        .params
                        .iter()
                        .map(|param| (param.name.clone(), lower_type(&param.ty)))
                        .collect(),
                    return_type: lower_type(&method.return_type),
                    blocks: vec![block],
                });
            }
        }

        ir.di_registry = di_graph
            .providers
            .iter()
            .map(|provider| DiRegistryEntry {
                token: provider.token.clone(),
                factory_fn: format!("factory::{}", provider.implementation),
            })
            .collect();
        ir.di_registry.sort_by(|a, b| a.token.cmp(&b.token));

        ir.route_table = route_table
            .get_routes()
            .iter()
            .map(|route| RouteRegistryEntry {
                method: route.method.to_string(),
                path: route.path.clone(),
                handler_fn: route.handler.clone(),
            })
            .collect();
        ir.route_table.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.method.cmp(&b.method))
                .then_with(|| a.handler_fn.cmp(&b.handler_fn))
        });

        ir.pipelines = pipelines
            .iter()
            .map(|(route, pipeline)| PipelineRegistryEntry {
                handler_fn: route.handler.clone(),
                guards: pipeline.guards.iter().map(|c| c.name.clone()).collect(),
                pipes: pipeline.pipes.iter().map(|c| c.name.clone()).collect(),
                interceptors: pipeline
                    .interceptors
                    .iter()
                    .map(|c| c.name.clone())
                    .collect(),
            })
            .collect();
        ir.pipelines.sort_by(|a, b| a.handler_fn.cmp(&b.handler_fn));

        Ok(ir)
    }
}

fn ends_with_return(instructions: &[IrInstruction]) -> bool {
    matches!(instructions.last(), Some(IrInstruction::Return(_)))
}

fn lower_stmt(stmt: &Stmt, out: &mut Vec<IrInstruction>) {
    match stmt {
        Stmt::Let { name, value, .. } => {
            out.push(IrInstruction::Store {
                dst: name.clone(),
                value: lower_expr(value),
            });
        }
        Stmt::Return(expr) => {
            out.push(IrInstruction::Return(expr.as_ref().map(lower_expr)));
        }
        Stmt::Expr(expr) => {
            if let Expr::Call { callee, args } = expr {
                out.push(IrInstruction::Call {
                    dst: None,
                    callee: callee_name(callee),
                    args: args.iter().map(lower_expr).collect(),
                });
            } else {
                out.push(IrInstruction::Nop);
            }
        }
        Stmt::If { .. } => {
            out.push(IrInstruction::Nop);
        }
    }
}

fn callee_name(callee: &Expr) -> String {
    match callee {
        Expr::Ident(name) => name.clone(),
        Expr::FieldAccess { obj, field } => format!("{}.{}", callee_name(obj), field),
        _ => "<callable>".to_string(),
    }
}

fn lower_expr(expr: &Expr) -> IrValue {
    match expr {
        Expr::IntLit(v) => IrValue::Int(*v),
        Expr::FloatLit(v) => IrValue::Float(*v),
        Expr::StringLit(v) => IrValue::String(v.clone()),
        Expr::BoolLit(v) => IrValue::Bool(*v),
        Expr::Null => IrValue::Null,
        Expr::Ident(name) => IrValue::Local(name.clone()),
        Expr::FieldAccess { obj, field } => {
            IrValue::Temporary(format!("{}.{}", callee_name(obj), field))
        }
        Expr::Call { callee, .. } => IrValue::Temporary(format!("call:{}", callee_name(callee))),
        Expr::BinOp { .. } => IrValue::Temporary("binop".to_string()),
        Expr::UnaryOp { .. } => IrValue::Temporary("unary".to_string()),
    }
}

fn lower_type(ty: &TypeExpr) -> IrType {
    match ty {
        TypeExpr::Named(name) => match name.as_str() {
            "Int" => IrType::Int,
            "Float" => IrType::Float,
            "Bool" => IrType::Bool,
            "String" => IrType::String,
            "Any" => IrType::Any,
            other => IrType::Named(other.to_string()),
        },
        TypeExpr::Generic { name, params } => {
            if name == "List" && params.len() == 1 {
                IrType::List(Box::new(lower_type(&params[0])))
            } else if name == "Map" && params.len() == 2 {
                IrType::Map(
                    Box::new(lower_type(&params[0])),
                    Box::new(lower_type(&params[1])),
                )
            } else {
                IrType::Named(name.clone())
            }
        }
        TypeExpr::Result { ok, err } => IrType::Result {
            ok: Box::new(lower_type(ok)),
            err: Box::new(lower_type(err)),
        },
        TypeExpr::Option(inner) => IrType::Option(Box::new(lower_type(inner))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::ast::{
        Annotation, AnnotationArg, Block, ClassDecl, Expr, MethodDecl, ModuleDecl, Param,
        SourceFile, Span, Stmt, TopLevelItem, TypeExpr,
    };

    use super::{IrGenerator, IrInstruction, IrType, IrValue};

    fn span() -> Span {
        Span {
            file: PathBuf::from("test.rw"),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 1,
        }
    }

    fn module(name: &str) -> ModuleDecl {
        ModuleDecl {
            name: name.to_string(),
            imports: vec![],
            providers: vec![],
            controllers: vec![],
            exports: vec![],
            span: span(),
        }
    }

    fn method_returning_int(name: &str, value: i64) -> MethodDecl {
        MethodDecl {
            annotations: vec![],
            name: name.to_string(),
            params: vec![],
            return_type: TypeExpr::Named("Int".to_string()),
            body: Block {
                stmts: vec![Stmt::Return(Some(crate::parser::ast::Expr::IntLit(value)))],
            },
            span: span(),
        }
    }

    fn class(name: &str, methods: Vec<MethodDecl>) -> ClassDecl {
        ClassDecl {
            annotations: vec![],
            name: name.to_string(),
            implements: vec![],
            constructor: None,
            methods,
            fields: vec![],
            span: span(),
        }
    }

    fn injectable_class(name: &str, methods: Vec<MethodDecl>) -> ClassDecl {
        let mut c = class(name, methods);
        c.annotations.push(Annotation {
            name: "injectable".to_string(),
            args: vec![],
            span: span(),
        });
        c
    }

    fn controller_class(name: &str, prefix: &str, methods: Vec<MethodDecl>) -> ClassDecl {
        let mut c = class(name, methods);
        c.annotations.push(Annotation {
            name: "controller".to_string(),
            args: vec![AnnotationArg::Positional(Expr::StringLit(
                prefix.to_string(),
            ))],
            span: span(),
        });
        c
    }

    fn route_method(name: &str, ann: &str, path: &str) -> MethodDecl {
        MethodDecl {
            annotations: vec![Annotation {
                name: ann.to_string(),
                args: vec![AnnotationArg::Positional(Expr::StringLit(path.to_string()))],
                span: span(),
            }],
            name: name.to_string(),
            params: vec![],
            return_type: TypeExpr::Named("Int".to_string()),
            body: Block {
                stmts: vec![Stmt::Return(Some(Expr::IntLit(1)))],
            },
            span: span(),
        }
    }

    #[test]
    fn generates_functions_from_class_methods() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Module(module("App")),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![method_returning_int("count", 1)],
                )),
            ],
        };

        let ir = IrGenerator::new().generate_ir(&ast).expect("must lower");
        assert_eq!(ir.name, "App");
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "UserController.count");
        assert_eq!(ir.functions[0].return_type, IrType::Int);
    }

    #[test]
    fn lowers_return_literal_instruction() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Module(module("App")),
                TopLevelItem::Class(class("Svc", vec![method_returning_int("v", 42)])),
            ],
        };

        let ir = IrGenerator::new().generate_ir(&ast).expect("must lower");
        let first_fn = &ir.functions[0];
        let first_block = &first_fn.blocks[0];

        assert!(first_block
            .instructions
            .iter()
            .any(|ins| { matches!(ins, IrInstruction::Return(Some(IrValue::Int(42)))) }));
    }

    #[test]
    fn lowers_param_types() {
        let method = MethodDecl {
            annotations: vec![],
            name: "sum".to_string(),
            params: vec![
                Param {
                    annotations: vec![],
                    name: "a".to_string(),
                    ty: TypeExpr::Named("Int".to_string()),
                    is_private: false,
                    span: span(),
                },
                Param {
                    annotations: vec![],
                    name: "b".to_string(),
                    ty: TypeExpr::Named("Int".to_string()),
                    is_private: false,
                    span: span(),
                },
            ],
            return_type: TypeExpr::Named("Int".to_string()),
            body: Block {
                stmts: vec![Stmt::Return(Some(crate::parser::ast::Expr::IntLit(0)))],
            },
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Module(module("App")),
                TopLevelItem::Class(class("Math", vec![method])),
            ],
        };

        let ir = IrGenerator::new().generate_ir(&ast).expect("must lower");
        assert_eq!(ir.functions[0].params.len(), 2);
        assert_eq!(ir.functions[0].params[0], ("a".to_string(), IrType::Int));
        assert_eq!(ir.functions[0].params[1], ("b".to_string(), IrType::Int));
    }

    #[test]
    fn generates_static_di_registry() {
        let mut app = module("App");
        app.providers = vec![crate::parser::ast::ProviderBinding::Simple(
            "Repo".to_string(),
        )];

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Module(app),
                TopLevelItem::Class(injectable_class("Repo", vec![])),
            ],
        };

        let ir = IrGenerator::new().generate_ir(&ast).expect("must lower");
        assert_eq!(ir.di_registry.len(), 1);
        assert_eq!(ir.di_registry[0].token, "Repo");
        assert_eq!(ir.di_registry[0].factory_fn, "factory::Repo");
    }

    #[test]
    fn generates_static_route_table() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Module(module("App")),
                TopLevelItem::Class(controller_class(
                    "UserController",
                    "/users",
                    vec![route_method("list", "get", "/")],
                )),
            ],
        };

        let ir = IrGenerator::new().generate_ir(&ast).expect("must lower");
        assert_eq!(ir.route_table.len(), 1);
        assert_eq!(ir.route_table[0].method, "GET");
        assert_eq!(ir.route_table[0].path, "/users");
        assert_eq!(ir.route_table[0].handler_fn, "UserController.list");
    }
}
