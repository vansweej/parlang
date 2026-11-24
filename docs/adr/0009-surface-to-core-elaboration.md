# ADR 0009: Surface to Core Elaboration

## Status

Accepted

Supersedes in part `docs/adr/0005-core-type-enum.md` (its rejection of tuples
and records). Resolves Findings F2 and F3 of
`docs/adr/0003-core-case-eliminator.md`.

## Context

Roadmap slice 7 connects the surface language to the explicitly typed Core IR
(ADRs 0005-0008). Until slice 7 the only execution route was the surface
tree-walking evaluator; Core was exercised only by hand-built terms. Slice 7
needs a total, tested translation from every supported surface program to a
Core term that passes Core Lint and runs on the CEK machine, without changing
surface semantics or any existing golden.

## Decision

- Elaboration consumes the typed AST (slice 7b), not the untyped AST. Every
  Core binder and type application is read off the typed AST, so Core is
  explicitly typed by construction (ADR 0007).
- Each let-generalised variable of a let scheme becomes one `TyLam` (`/\`);
  each instantiation argument recorded on a surface `Var` becomes one `TyApp`
  (`@`). Residual (unconstrained) type variables default to `Unit`.
- Tuples are Core data types named by arity, `(,)`, `(,,)`, ..., each with a
  single constructor of the same name. Closed records are single-constructor
  Core data types named by their sorted field names, `{a,b}`, whose
  constructor fields are in sorted-name order. Row polymorphism was removed in
  roadmap slice 1, so records are closed and need no row type: this
  supersedes ADR 0005's "records need a row type" and its deferral of tuples.
  The unit value is Core `Unit`, booleans are Core `Bool`, user data types
  keep their constructor names, and strings are the user-declared `List Char`.
- Comparisons are rewritten to `<`, `eq` and `if`. Equality is restricted to
  base types (Core `eq` is defined only there).
- Hard rejections are an explicit, tested invariant (roadmap decision 6):
  `load`, equality at a non-base type (including a type variable), `rec` of a
  non-function, and a data type applied at the wrong arity all fail with
  `ElabError::Unsupported`; elaboration never silently drops a construct.
- Pattern matching compiles to nested Core `Case` with join points (`$j`
  names) shared between arms, resolving ADR 0003 Finding F2. Exhaustiveness
  is checked at type-check time (slice 7c, `match_coverage.rs`), so Core never
  needs a FAIL term, resolving Finding F3. A partial record pattern over a
  closed record covers all values and compiles over the full sorted field set.
- A differential oracle (`every_example_agrees_on_core` in
  `crates/parlang/tests/examples_execution_tests.rs`) runs every example on
  both routes and requires structurally agreeing values, except for an
  explicit list of examples rejected with a required error substring. The
  list fails when it goes stale.
- `parlang_core::render_value` is a temporary, type-directed renderer for Core
  results (strings, lists, tuples, records, constructors, `<function>`). It is
  to be replaced by a real `Show` class.
- The surface evaluator stays the default route. The Core route is reached
  with `--core`, `--stage core`, `--stage lint` and `trace` on both binaries.

## Consequences

- Every example either agrees on Core or is a listed, intentional rejection;
  `examples/records.par` and `examples/mvp.par` exercise records and the MVP
  acceptance program on both routes, and the driver goldens cover the MVP
  through every stage.
- Programs that `load` libraries or compare polymorphic values cannot run on
  Core until typed modules and a class-based equality exist.
- Polymorphic records (a record literal whose field types stay generalised)
  are encoded only when their field types are resolved; the examples pin
  record types with an identity helper.

## References

- `docs/adr/0003-core-case-eliminator.md` (Findings F2, F3)
- `docs/adr/0005-core-type-enum.md` (tuple and record rejection, superseded)
- `docs/adr/0006-core-system-f-term-erasure.md`
- `docs/adr/0007-core-explicit-typing-and-lint.md`
- `docs/adr/0008-core-cek-machine.md`
- `docs/MODULE_ELABORATE.md`
