# ADR 0012: Surface Higher-Kinded Types on Core

## Status

Accepted

Builds on `docs/adr/0011-core-kinds.md` (M2.1a) and the surface kind
checker of slice M2.1b-1 (`crates/parlang/src/kind_check.rs`). Implements
slice M2.1b-2.

## Context

The surface typechecker infers data parameter kinds and accepts
higher-kinded programs such as `examples/hkt.par`
(`data Wrap f a = MkWrap (f a)` used at `Wrap Option Int`). Core Lint
already checks kinds, but the elaborator gave every binder and data
parameter kind `*` and rejected applied type variables, so such programs
ran only on the surface evaluator.

## Decision

- The typed AST carries the inferred parameter kinds: `TypedDecl::Data`
  and `TypedExprKind::TypeDef` have `param_kinds`. They are captured from
  kind inference before any body or `let` value is inferred, so a nested
  same-name type cannot change them. Core `DataType.param_kinds` uses them.
- Each let-generalised variable becomes a `TyLam` of its inferred kind.
  `kind_check::infer_type_var_kinds` infers the kinds from the scheme's
  type, using the parameter kinds of registered data types. Unsolved
  kinds default to `*`.
- An applied type variable `f a` in scope becomes the Core application
  `App(Var tN, ..)`.
- A partially applied data type such as `Option` in `Wrap Option Int` is
  accepted and its kind is checked by Core Lint. Over-application remains
  `ElabError::Unsupported`.
- Residual variables: a bare residual variable still defaults to `Unit`.
  A residual applied variable is `Unsupported`. A residual variable used at
  a non-`*` kind therefore always fails, either at elaboration or in Lint
  with `KindMismatch`; it is never silently accepted.

## Consequences

- `examples/hkt.par` elaborates, passes Lint, runs on Core and agrees with
  the surface evaluator; it left the oracle's expected-rejection list.
  Recursive higher-kinded fields such as `data Fix f = In (f (Fix f))` also
  run on Core.
- Kind errors stay in Core Lint, matching ADR 0011. Elaboration's kind
  inference is best effort and never reports errors itself.
- Known limitation: the elaborator's record of data parameter kinds is
  flat, not scoped. A nested `data` expression that re-declares a type name
  replaces the earlier kinds for the rest of elaboration, as the earlier
  parameter-count record did. Core Lint still checks every use.
- Type classes over higher-kinded parameters remain future work.

## References

- `docs/adr/0009-surface-to-core-elaboration.md`
- `docs/adr/0011-core-kinds.md`
- `docs/MODULE_ELABORATE.md`
