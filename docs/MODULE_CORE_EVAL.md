# Module: parlang-core eval (Arc C Core VM)

## Role

The `parlang-core` eval module implements a strict, call-by-value
tree-walking virtual machine over the Arc B Core `Term` IR. It is the runtime
counterpart to the Core data model established in Arc B, and its behavior is
formally specified in [docs/CORE_OPERATIONAL_SEMANTICS.md](CORE_OPERATIONAL_SEMANTICS.md)
and justified in [docs/adr/0001-arc-c-eval-model.md](adr/0001-arc-c-eval-model.md).

## Public API

### `Value`

The runtime value domain, mirroring `Lit`'s base types plus function and
constructor values:

- `Int(i64)`, `Bool(bool)`, `Unit`, `Str(String)`, `Float(f64)`
- `Closure { param, body, env }` — a non-recursive function closure.
- `RecClosure { name, param, body, env }` — a **distinct** variant from
  `Closure`. Unlike `Closure`, a `RecClosure` carries the name it was bound
  under in its originating `LetRec`, so that name can be re-inserted into
  the environment at every application, enabling recursion of arbitrary
  depth (see "Recursion" below).
- `Con(name, Vec<Value>)` — a constructor application value.

### `Environment`

A persistent, clone-and-extend environment mapping names to values:

- `Environment::new() -> Environment`
- `lookup(&self, name: &str) -> Option<&Value>`
- `extend(&self, name: &str, value: Value) -> Environment` — returns a new
  environment; the receiver is left unchanged (value semantics).

### `EvalError`

All evaluation error variants and their `Display` messages:

- `UnboundVar` — a variable was referenced but not found in the environment.
- `NotAFunction` — application was attempted on a non-function value.
- `TypeMismatch` — an operation received operand(s) of the wrong type
  (e.g., non-`Int` operands to arithmetic or `<`, or a non-`Lam` bound by
  `LetRec`).
- `DivisionByZero` — integer division or remainder by zero.
- `ArithmeticOverflow` — a checked arithmetic operation overflowed or
  underflowed.
- `NotComparable` — `eq` was applied to non-base-type operands (functions or
  constructor values).

### `EvalResult`

A type alias for `Result<Value, EvalError>`.

### `eval`

```rust
pub fn eval(term: &Term, env: &Environment) -> EvalResult
```

Evaluates a Core `Term` under a given `Environment`, following the big-step
rules in [docs/CORE_OPERATIONAL_SEMANTICS.md](CORE_OPERATIONAL_SEMANTICS.md).

## Primitive encoding

Per ADR 0001 Decision 3 (Option A), primitives and branching are encoded as
reserved constructor names rather than dedicated `Term` variants, keeping the
shared `Term` AST frozen:

- `Con("if", [c, t, e])` — strict in `c` (must evaluate to `Bool`), lazy in
  the untaken branch (only the taken branch of `t`/`e` is evaluated).
- `Con("+"|"-"|"*"|"/", [a, b])` — both operands must be `Int`; checked
  arithmetic (`DivisionByZero` / `ArithmeticOverflow` / `TypeMismatch` on
  mismatch).
- `Con("<", [a, b])` — `Int` operands only, yields `Bool`.
- `Con("eq", [a, b])` — restricted to base values (`Int`, `Bool`, `Unit`,
  `Str`, `Float`); `Float` operands are compared via `f64::to_bits`
  bit-equality; function or constructor operands are rejected with
  `NotComparable`.
- `Con("strlen", [s])` — `s` must be `Str`; returns the `Int` count of
  UTF-8 characters in `s`. See `MODULE_CORE_PRELUDE.md` for the Core-defined
  helpers (`isEmpty`, `nonEmpty`, etc.) built on top of this primitive.
- `Con("strcat", [a, b])` — both `a` and `b` must be `Str`; returns their
  concatenation as a `Str`. See `MODULE_CORE_PRELUDE.md` for the Core-defined
  helpers built on top of this primitive.

The `eq` float semantics are intended, not accidental: under bit-equality,
`NaN == NaN` is **true**, and `0.0 != -0.0` (signed zero is distinct from its
negation).

## Recursion

`LetRec` evaluation builds a self-binding recursive closure: `Term::LetRec(n, v, b)`
requires `v` to be a `Term::Lam`, wraps it as a `Value::RecClosure` capturing
the environment at definition time (without pre-inserting `n`), and binds `n`
to that closure before evaluating `b`. Crucially, the recursive name is
**re-bound at every application** — not just once at definition — which is
what allows recursion of arbitrary depth to terminate correctly rather than
unwinding exactly one level and raising `UnboundVar`. See ADR 0001 Decision 4
for the full rationale.

## List demonstration

List computation is demonstrated via a Church/Scott-encoded list fold
(`nil`/`cons`/`sum`), which performs genuine computation over encoded lists,
complementing the strict, direct construction of `Value::Con` values.

## References

- [docs/CORE_OPERATIONAL_SEMANTICS.md](CORE_OPERATIONAL_SEMANTICS.md)
- [docs/adr/0001-arc-c-eval-model.md](adr/0001-arc-c-eval-model.md)
- Benjamin C. Pierce, *Types and Programming Languages* (TAPL), Ch. 5, 6, 7.
- Olivier Danvy, "A Rational Deconstruction of Landin's SECD Machine",
  BRICS-RS-03-13.
