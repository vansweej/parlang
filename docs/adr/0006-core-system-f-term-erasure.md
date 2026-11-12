# ADR 0006: Type-Erased System-F Core Terms

## Status

Accepted. Builds on ADR 0005 (Core `Type`) and ADR 0001 (strict Core VM).

## Context

Slice 6d makes Core explicitly typed: `Term` gains type abstraction `TyLam` and
type application `TyApp`, `Con` gains explicit type arguments, and `LetRec` gains
a type annotation on its recursive binder. We must fix the runtime meaning of the
new type-level nodes.

## Decision

Type-level constructs are ERASED at evaluation time:

- `eval(TyLam(a, t)) = eval(t)`
- `eval(TyApp(t, ty)) = eval(t)`
- `Con` type arguments are ignored by `eval`; a `Con` value carries only its
  evaluated value arguments.
- `LetRec`'s annotation is ignored by `eval` (it is shown in dumps, for teaching).

Types play no part in the strict, call-by-value dynamic semantics; they exist for
the Core Lint checker (slice 6e) and for legible dumps. This mirrors GHC, which
erases type lambdas and type applications before runtime.

## Consequences

- The VM, `Value` and `Environment` are unchanged in shape; erasure is a one-line
  passthrough per node.
- Well-typedness is NOT enforced by `eval`; that is the Core Lint pass (6e).
- Named capture-avoiding `Type::subst` and `Type::alpha_eq` (added in 6d) back the
  type-level reasoning Lint needs; the named representation's De Bruijn exit
  (ADR 0005) remains available but was not taken.

## References

- `docs/adr/0005-core-type-enum.md` - the Core `Type` enum.
- `docs/adr/0001-arc-c-eval-model.md` - the strict Core VM.
- `crates/parlang-core/src/eval.rs` - the erasure arms.
