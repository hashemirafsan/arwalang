# Technical Documentation

This directory contains phase-oriented implementation documentation for the ArwaLang compiler.

## Purpose

- Keep a technical record of what was built in each phase.
- Capture design decisions, validation results, and known gaps.
- Make handoff and review easier for future contributors.

## Structure

- `docs/phases/roadmap.md`: phase-by-phase plan and status snapshot.
- `docs/phases/phase-00-project-setup.md`: implementation record for Phase 0.
- `docs/phases/phase-01-lexer.md`: implementation record for Phase 1.
- `docs/phases/phase-02-parser-ast.md`: implementation record for Phase 2.
- `docs/phases/phase-03-name-resolution.md`: implementation record for Phase 3.
- `docs/phases/phase-04-type-checker.md`: implementation record for Phase 4.
- `docs/phases/phase-05-annotation-processor.md`: implementation record for Phase 5.
- `docs/phases/template.md`: template to document upcoming phases.

## Update Policy

- After completing a phase, add or update its `phase-xx-*.md` document.
- Every phase document should include:
  - objective
  - scope delivered
  - key implementation details
  - tests and validation commands
  - known limitations and next actions
