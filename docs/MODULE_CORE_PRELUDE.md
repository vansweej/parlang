# Module: parlang-core prelude (Arc B Core seam)

## Role

The `parlang-core` prelude module provides the pure standard prelude built
atop the Arc C Core VM. It demonstrates the seam between host-provided
primitives (`strlen`, `strcat`) and Core-defined functions (`not`, `isEmpty`,
`nonEmpty`), all composed using the existing `Term` builder API without any
new `Term` variants or evaluator primitives.

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

Three functions are defined purely in terms of Core's `Lam`/`Con`/`App`
constructs, with no new evaluator primitives:

- **`not : Bool -> Bool`** — `\b: bool. if b then false else true`, built as
  `lam("b", BaseType::Bool, con("if", vec![var("b")?, bool_(false), bool_(true)])?)`.
- **`isEmpty : String -> Bool`** — `\s: string. eq (strlen s) 0`, built as
  `lam("s", BaseType::String, con("eq", vec![con("strlen", vec![var("s")?])?, int(0)])?)`.
- **`nonEmpty : String -> Bool`** — `\s: string. not (isEmpty s)`, built as
  `lam("s", BaseType::String, app(var("not")?, app(var("isEmpty")?, var("s")?)))`.

## Host primitives

Two host-provided string primitives are relied upon by the Core-defined
functions above, both implemented directly in the evaluator (see
[MODULE_CORE_EVAL.md](MODULE_CORE_EVAL.md)):

- **`strlen`**: returns the UTF-8 character count of a string as an `Int`.
- **`strcat`**: concatenates two strings.

## Incremental seeding mechanism

`prelude()` seeds the environment incrementally via `extend`-chaining:

1. Start from `Environment::new()`.
2. Build and evaluate the `not` term against the current environment, then
   `extend` the environment with the resulting closure bound to `"not"`.
3. Build and evaluate the `isEmpty` term against the environment from step
   2 (which now has `not` in scope, though `isEmpty` does not use it), then
   `extend` with the result bound to `"isEmpty"`.
4. Build and evaluate the `nonEmpty` term against the environment from step
   3 (which now has both `not` and `isEmpty` in scope, both of which
   `nonEmpty`'s body references), then `extend` with the result bound to
   `"nonEmpty"`.
5. Return the final environment.

Because `Environment::extend` returns a new environment rather than mutating
in place, each seeding step produces a new environment value that is passed
forward to the next step — this is what allows `nonEmpty`'s body to resolve
`not` and `isEmpty` by name when it is evaluated.

## Deferred: higher-order prelude functions

Higher-order prelude functions (e.g., `map`, `filter`, `fold` over Core
function-typed values) are deferred to the arc that introduces Core function
types. `BaseType` cannot yet annotate a function-typed binder — it only
covers `Int`, `Bool`, `Float`, `Unit`, and `String` — so a prelude function
whose parameter or result is itself a function cannot yet be given a faithful
binder type annotation. This module's Core-defined functions (`not`,
`isEmpty`, `nonEmpty`) all have first-order (`Bool`/`String`-typed) binders,
which is why they are expressible today.

## References

- [MODULE_CORE_EVAL.md](MODULE_CORE_EVAL.md)
- [CORE_OPERATIONAL_SEMANTICS.md](CORE_OPERATIONAL_SEMANTICS.md)
- [adr/0001-arc-c-eval-model.md](adr/0001-arc-c-eval-model.md)