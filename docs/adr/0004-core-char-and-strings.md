# ADR 0004: Core Char and Strings

## Status

Accepted

Supersedes the "`String` base type" outcome recorded in
`crates/parlang-core/README.md` and partly supersedes the C4 primitive-string
note in `docs/adr/0001-arc-c-eval-model.md` (Decision 3).

## Context

An earlier arc (C4) added a primitive string to Core: `Lit::Str`,
`BaseType::String`, `Value::Str`, the reserved `strlen`/`strcat` primitives,
a `Str` arm in `eq`, the `str_` builder, and the `isEmpty`/`nonEmpty` prelude
entries. This was a false start: a primitive `Str` can never receive a surface
string, because the surface parser desugars a string literal into a list of
character constructors (`Cons('a', Cons('b', ... Nil))`). Core also had no
character type at all.

## Decision

- Core gains a `Char` type: `BaseType::Char`, `Lit::Char(char)`,
  `Value::Char(char)`. A Core `Char` is a **Unicode scalar value** (a code
  point), exactly Haskell's `Char` and Rust's `char` — NOT a grapheme
  cluster. A string of n scalar values is a list of n `Char`s; an emoji built
  from combining or zero-width-joiner sequences is several `Char`s.
- `eq` and `<` accept two `Char`s: `eq` by scalar equality, `<` by scalar
  ordering (Rust `char` `Ord`), mirroring the surface language.
- A string in Core is `List Char` under ADR 0002's `Cons`/`Nil` encoding.
  There is no primitive string value.
- The primitive-string false start is retired: `Lit::Str`,
  `BaseType::String`, `Value::Str`, the `strlen` and `strcat` reserved
  primitives, the `Str` arm of `eq`, the `str_` builder, and the prelude
  `isEmpty`/`nonEmpty` entries are all removed. After this change, those two
  retired names are ordinary constructor names, not reserved dispatch.
- The prelude retains only `not`. Rebuilding `isEmpty`/`nonEmpty` over
  `List Char` is deferred: Core `Lam` binders still require a `BaseType`,
  which cannot express `List Char` until the Core `Type` enum arrives
  (slice 6c). A real string prelude arrives with elaboration and the standard
  library later.

## Consequences

- Rendering a `List Char` readably (as `"ab"` rather than nested `Cons`
  cells) is deferred to the temporary host-side value renderer (slice 7) and,
  later, a real `Show` class. The `Char` `Display` renders a single scalar as
  a Rust/Haskell char literal (for example `'a'`).
- The `eq` mismatched-base-type behaviour (returning `false` rather than an
  error for operands of different base types) is unchanged; revisiting it is a
  separate deferred follow-up.

## References

- `docs/adr/0002-arc-c4-list-encoding.md` — the frozen `Cons`/`Nil` list
  encoding.
- `docs/CORE_OPERATIONAL_SEMANTICS.md` — the living evaluator specification.
