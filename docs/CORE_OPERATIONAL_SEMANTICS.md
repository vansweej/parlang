# Core Operational Semantics (Arc C)

> This document assumes ADR [0001-arc-c-eval-model](adr/0001-arc-c-eval-model.md)
> Decision 3 Option A: primitives and branching are encoded as reserved
> constructor names dispatched by the VM, not as dedicated `Term` variants.

## Overview

The Arc C evaluator gives a **big-step, call-by-value, environment-based**
operational semantics for the Arc B Core `Term` IR. It is realized by
`parlang-core::eval`. This follows TAPL Ch. 5 (call-by-value operational
semantics), Ch. 6 (nameless/De Bruijn representation), Ch. 7 (an ML
implementation of the evaluator), and BRICS-RS-03-13 (Danvy, "A Rational
Deconstruction of Landin's SECD Machine").

## Values

The value domain is:

```
v ::= n                         -- integer
    | b                         -- boolean
    | ()                        -- unit
    | s                         -- string
    | f                         -- float
    | <closure ρ, x, t>         -- function closure
    | <recclosure ρ, g, x, t>   -- recursive function closure
    | C(v₁..vₙ)                 -- constructor application
```

This maps to the Rust `Value` enum:

- `Int(i64)`
- `Bool(bool)`
- `Unit`
- `Str(String)`
- `Float(f64)`
- `Closure { param, body, env }`
- `RecClosure { name, param, body, env }` — a distinct variant from
  `Closure`, carrying the recursive binding's name.
- `Con(name, Vec<Value>)`

## Environments

ρ is a finite map from names to values. Environments are persistent:
extending an environment produces a new environment via clone-and-extend,
leaving the original unchanged: `extend(&self, name, value) -> Environment`.

## Evaluation rules

The big-step judgement `ρ ⊢ t ⇓ v` is defined by cases on `Term`:

### `Var`

Lookup the name in ρ. An unbound name is an error (`UnboundVar`).

### `Lit`

Injects the literal into the corresponding value (`Int`, `Bool`, `Unit`,
`Str`, `Float`).

### `Lam(x, τ, t)`

```
ρ ⊢ Lam(x, τ, t) ⇓ <closure ρ, x, t>
```

The type annotation τ is ignored at runtime.

### `App(t₁, t₂)`

Strict in both positions:

```
ρ ⊢ t₁ ⇓ callee
ρ ⊢ t₂ ⇓ v_arg
```

- If `callee` is `<closure cenv, x, body>`: evaluate `body` under
  `cenv[x ↦ v_arg]`.
- If `callee` is `<recclosure cenv, g, x, body>`: evaluate `body` under
  `cenv[g ↦ callee][x ↦ v_arg]` — the rec-name `g` is re-bound at EACH
  application.
- Otherwise: `NotAFunction`.

### `Let(x, t₁, t₂)`

Strict:

```
ρ ⊢ t₁ ⇓ v₁
ρ[x ↦ v₁] ⊢ t₂ ⇓ v
─────────────────────
ρ ⊢ Let(x, t₁, t₂) ⇓ v
```

### `LetRec(x, t₁, t₂)`

`t₁` is expected to be a `Lam`; it becomes a `RecClosure` capturing the
current environment ρ **without** pre-inserting the name `x` into it. `t₂` is
then evaluated under `ρ[x ↦ recclosure]`.

### `Con` reserved-name rules (Option A)

- **`Con("if", [c, thn, els])`**: `c ⇓ Bool(b)`; if `b` is `true`, evaluate
  `thn`; otherwise evaluate `els`. The other branch is **not** evaluated.
- **`Con("+"/"-"/"*"/"/" , [a, b])`**: both `a` and `b` evaluate to `Int`;
  apply checked arithmetic. Division by zero yields `DivisionByZero`. A
  `None` result from a checked operation (overflow/underflow) yields
  `ArithmeticOverflow`. Non-`Int` operands yield `TypeMismatch`.
- **`Con("<", [a, b])`**: `Int` operands only, result is `Bool`. Non-`Int`
  operands yield `TypeMismatch` (there is no float comparison).
- **`Con("eq", [a, b])`**: base-type operands only. `Float` operands are
  compared via `f64::to_bits` bit-equality. `Closure`/`RecClosure`/`Con`
  operands yield `NotComparable`.
- A generic `Con(name, args)` with a non-reserved `name` evaluates all `args`
  strictly, left-to-right, and yields `Con(name, [v..])`.

## Errors

`EvalError` variants:

- `UnboundVar`
- `NotAFunction`
- `TypeMismatch`
- `DivisionByZero`
- `ArithmeticOverflow`
- `NotComparable`

## Note on branching laziness

General `Con` arguments are evaluated strictly, left-to-right. The reserved
`if` primitive is a special case: it is lazy in its two branches — only the
taken branch is evaluated.

## Note on `eq` float semantics

`eq` on `Float` uses `f64::to_bits` bit-equality. This is **intended**, not a
bug: under bit-equality, `NaN == NaN` is **true**, and `0.0 != -0.0` (signed
zero is distinct from its negation).

## References

- Benjamin C. Pierce, *Types and Programming Languages* (TAPL), Ch. 5, 6, 7.
- Olivier Danvy, "A Rational Deconstruction of Landin's SECD Machine",
  BRICS-RS-03-13.
