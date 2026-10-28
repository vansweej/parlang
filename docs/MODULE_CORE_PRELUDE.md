# Module: parlang-core prelude (Arc B Core seam)

## Role

The `parlang-core` prelude module provides the pure standard prelude built
atop the Arc C Core VM. It currently provides a single Core-defined function,
`not`, composed using the existing `Term` builder API without any new `Term`
variants or evaluator primitives. Strings in Core are `List Char` (ADR 0004);
a string prelude is deferred until Core can type `List Char` binders.

## Public API

### `prelude() -> Result<Environment, PreludeError>`

Builds and returns an `Environment` pre-populated with the prelude bindings.
Each binding is constructed as a `Term` via the `builder` module, evaluated
against the environment accumulated so far, and then bound into the
environment returned to the next seed step.

### `PreludeError`

An error type unifying the two failure modes that can occur while building
the prelude:

- `PreludeError::Build(BuildError)` — a term failed to build (e.g., an empty
  binder or constructor name).
- `PreludeError::Eval(EvalError)` — a seed term failed to evaluate.

`PreludeError` implements `Display`, `std::error::Error`, and `From<BuildError>`
/ `From<EvalError>` for ergonomic `?`-based composition.

## Core-defined functions

One function is defined purely in terms of Core's `Lam`/`Con`/`App`
constructs, with no new evaluator primitives:

- **`not : Bool -> Bool`** — `\b: bool. if b then false else true`, built as
  `lam("b", BaseType::Bool, con("if", vec![var("b")?, bool_(false), bool_(true)])?)`.

## Strings

Strings in Core are `List Char` under ADR 0002's `Cons`/`Nil` encoding (see
[adr/0004-core-char-and-strings.md](adr/0004-core-char-and-strings.md)). There
are no host string primitives: the earlier primitive-string false start has
been retired, and those names are ordinary constructor names again.

## Incremental seeding mechanism

`prelude()` seeds the environment incrementally via `extend`-chaining:

1. Start from `Environment::new()`.
2. Build and evaluate the `not` term against the current environment, then
   `extend` the environment with the resulting closure bound to `"not"`.
3. Return the final environment.

Because `Environment::extend` returns a new environment rather than mutating
in place, each seeding step produces a new environment value that is passed
forward to the next step, so a later prelude function can resolve earlier
ones by name when it is evaluated.

## Deferred: higher-order prelude functions

Higher-order prelude functions (e.g., `map`, `filter`, `fold` over Core
function-typed values) are deferred to the arc that introduces Core function
types. `BaseType` cannot yet annotate a function-typed binder — it only
covers `Int`, `Bool`, `Float`, `Unit`, and `Char` — so a prelude function
whose parameter or result is itself a function cannot yet be given a faithful
binder type annotation. This module's only Core-defined function, `not`, has
a first-order (`Bool`-typed) binder, which is why it is expressible today.

## References

- [MODULE_CORE_EVAL.md](MODULE_CORE_EVAL.md)
- [CORE_OPERATIONAL_SEMANTICS.md](CORE_OPERATIONAL_SEMANTICS.md)
- [adr/0001-arc-c-eval-model.md](adr/0001-arc-c-eval-model.md)