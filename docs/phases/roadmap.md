# Compiler Phase Roadmap

This file tracks planned and completed implementation work across phases.

## Status Legend

- done: completed and validated
- in_progress: currently being implemented
- planned: not started yet

## Phase Status

| Phase | Name | Status | Primary Output |
|---|---|---|---|
| 0 | Project Setup | done | Rust workspace, structure, CI/tooling baseline |
| 1 | Lexer | done | Token model, lexer engine, lexer tests |
| 2 | Parser & AST | done | AST definitions and recursive-descent parser |
| 3 | Name Resolution | done | Symbol tables and name binding pass |
| 4 | Type Checker | done | Strict type rules and serializability checks |
| 5 | Annotation Processor | done | Annotation validation and semantic metadata |
| 6 | DI Graph | done | Provider graph + DI validation rules |
| 7 | Module Graph | done | Module dependency graph + visibility checks |
| 8 | Route Table | done | Static route registry and route validation |
| 9 | Lifecycle Pipeline | done | Static middleware/guard/pipe/interceptor chains |
| 10 | IR & Codegen | planned | IR lowering + Cranelift-first native output |
| 11 | Error Reporting | planned | Structured diagnostics + human/JSON output |
| 12 | CLI | planned | `arwa` command implementations |
| 13 | Scaffolding & Blueprint | planned | Template registry and project generation |
| 14 | Standard Library | planned | `http.rw`, `result.rw`, `json.rw` |
| 15 | Testing Infrastructure | planned | Unit/e2e harness and fixture programs |
| 16 | Documentation | planned | User/dev documentation set |
| 17 | Polish & Release | planned | hardening, performance, release packaging |

## Execution Notes

- Source of truth hierarchy:
  - `requirements.md` defines architecture and acceptance requirements.
  - `tasks.md` tracks execution checklist.
- Execution mode: strict phase-by-phase progression.
- Toolchain target: Rust stable latest.
- Phase 10 policy: Cranelift-first delivery, LLVM optional and non-blocking.
