# Exhaustiveness Checking

## Overview

ParLang checks every `match` expression for exhaustiveness during type
checking. `infer_match` calls `check_match` in
`crates/parlang/src/match_coverage.rs`. The check is sound: a match it accepts
always has an arm for every value of the scrutinee's type. A non-exhaustive
match is rejected with `TypeError::NonExhaustiveMatch { missing }`, and the
program is not evaluated. No runtime warning is ever printed.

## How coverage is decided

Coverage is decided per type:

- `Bool`: both `true` and `false` must be covered.
- `Unit`: `()` covers the type.
- Tuples and records each have a single constructor. Their components are
  checked position by position. A partial record pattern covers the fields it
  omits.
- Data types are covered by their declared constructors, each with its
  payload covered.
- `Int`, `Char` and `Float` literals never complete a match. Add a variable or
  `_` arm.
- Nested patterns are checked position by position, so `Some (Some n)` plus
  `None` still misses `Some None`.

## Witnesses

`missing` lists example patterns that no arm matches, for instance `_`,
`true`, `()`, `Some None`, `Cons _ _`, `Some (Some _)`, `(Nil, Cons _ _)` and
`{x: _, y: _}`.

## Examples

Rejected:

```parlang
data Option a = Some a | None;
let f = fun o -> match o with
| Some (Some n) -> n
| None -> 0
in f None
```

Error: `Non-exhaustive match: missing Some None`

Accepted:

```parlang
data Option a = Some a | None;
let f = fun o -> match o with
| Some (Some n) -> n
| Some None -> 1
| None -> 0
in f None
```

## Unknown constructors

Every constructor in a pattern or expression needs a `data` declaration in
scope. Otherwise type checking fails with `UnknownConstructor`.

## Runtime

`EvalError::PatternMatchNonExhaustive` can only occur when code is evaluated
without type checking, through the library `eval` API.

## See also

- [Sum Types](SUM_TYPES.md)
- [Typechecker Module](MODULE_TYPECHECKER.md)
