# ADR 0011: Core Kinds

## Status

Accepted

Implements the kind slot deferred by `docs/adr/0005-core-type-enum.md`
(milestone 2, slice M2.1a).

## Context

Milestone 2 adds typeclasses and higher-kinded types such as a
`Functor f` over `f : * -> *`. Core already represents the type
application `f a` (binary `App`), but every binder was implicitly kind
`*`, so Lint rejected variable-headed application.

## Decision

- A `Kind` enum in `parlang-core`: `Star` (`*`) and `Arrow` (`k1 -> k2`),
  rendered right-associatively. Core has no kind variables and no kind
  inference: Core is explicitly typed, so kinds are always given.
- `Type::Forall` and `Term::TyLam` carry the binder's kind. The builders
  `forall` and `tylam` default to `*`; `forall_k` and `tylam_k` take a
  kind.
- `DataType` carries `param_kinds`, parallel to `params`.
  `try_with_data` rejects a length mismatch
  (`ParamKindCountMismatch`) and checks field types with each parameter
  at its declared kind. `with_data` does not validate and is meant
  only for the prelude and tests.
- Lint is a kind checker. A type at a `*` position must have kind `*`,
  and a constructor applied to the wrong number of arguments there is
  still `ArityMismatch`. A type argument is checked against its binder's
  or parameter's kind, so `f a` with `f : * -> *` is accepted and a
  partially applied constructor such as `Pair Int` may instantiate `f`.
  Other violations are `KindMismatch`.
- A `*` binder prints exactly as before; any other binder prints its
  kind, for example `forall (f : * -> *). f a`. Existing dumps and
  goldens are byte-identical.
- Kernel eligibility (ADR 0010) is unchanged. Every `TyLam` is an R5
  (polymorphism) violation whatever its kind, and a higher-kinded
  variable is never a kernel type.

## Consequences

- The surface language (slice M2.1b) only needs to infer kinds for data
  declarations and pass them on; Core already checks them. This is done:
  see `docs/adr/0012-surface-higher-kinded-types.md`.
- Kind errors are reported by Core Lint, matching ADR 0005's split of
  responsibilities.
