# ADR 0005: Core Type Enum

## Status

Accepted

Builds on `docs/adr/0003-core-case-eliminator.md` (which records Finding F1:
Core `Lam` binders can only carry a `BaseType`). Slice 6b adds the Core type
representation; retiring the F1 placeholder on `Term::Lam` is deferred to slice
6c.

## Context

Roadmap slice 6 ("Typed Core") makes Core a typed System-F IR so that
dictionary-passing typeclasses and higher-kinded types stay reachable. Slice 6b
introduces the `Type` enum only; `Term` is unchanged. The representation has to
admit variable-headed application (`f a`) without a later restructure, must
render legibly (pedagogical legibility is priority #1), and must keep slice 6c's
mechanical sweep from `BaseType` to the new `Type` cheap.

## Decision

- A new `crates/parlang-core/src/ty.rs` module defines `pub enum Type`,
  re-exported from `lib.rs` as `parlang_core::Type`. The module is named `ty`
  because `type` is a Rust keyword.
- `BaseType` is KEPT and ABSORBED as a leaf: `Type::Base(BaseType)`, with a
  `From<BaseType> for Type` impl. Literals, VM primitives and the lowercase
  golden rendering stay keyed on `BaseType`, and slice 6c's sweep becomes
  mechanical (`BaseType::Int` becomes `Type::Base(BaseType::Int)` or
  `.into()`).
- Type-constructor application is binary `App(Box<Type>, Box<Type>)`
  (GHC `AppTy`-style) with a nullary `Con(String)`, rather than saturated
  `Con(String, Vec<Type>)`. Binary application admits variable-headed
  application `f a` directly, so higher-kinded types (milestone 2) need no
  restructure. Arrow is a dedicated `Fun(Box<Type>, Box<Type>)`.
- Type variables are NAMED (`Var(String)`), matching `Term`'s named binders for
  dump legibility. Quantification is single-binder System F
  (`Forall(String, Box<Type>)`).
- No kinds in slice 6b. Binders are implicitly kind `*`; a kind slot on
  `Forall` is deferred to milestone 2.
- Display is ASCII only (no `forall` glyph): `forall a. a -> List a`. Arrows are
  right-associative with parenthesised arrow domains; application is
  juxtaposition with nested applications and non-atoms parenthesised (NOT the
  surface's lossy `SumType` rendering); `Base` leaves render via `BaseType`'s
  lowercase `Display`.
- Slice-6b structural surface is intentionally small: `free_vars` (deterministic
  `BTreeSet<String>` order) plus builders (`base_ty`, `tvar`, `tcon`, `tapp`,
  `tapps`, `fun_ty`, `forall`). Capture-avoiding substitution and
  alpha-equivalence are deferred to the slices that first need them (6d/6e), to
  keep the 6b surface minimal.

## Invariants and non-invariants

- Constructor arity and kinds are NOT structural invariants of `Type`:
  `App(Con("List"), ...)` can be over-applied or ill-kinded and still be a valid
  value of `Type`. Rejecting such types is the job of the Core Lint pass (slice
  6e) against a kind / constructor environment (milestone 2 kinds), not of the
  `Type` enum.
- Slice 6b performs no substitution under binders, so `free_vars` is capture-safe
  by construction (it only removes the bound name from the body's free set).
  Slice 6d, which introduces type-level substitution, must either implement
  capture-avoiding substitution over this NAMED representation or take the
  De Bruijn exit already recorded for the named representation in
  `crates/parlang-core/README.md` (the "Named vs De Bruijn" ADR-outcome bullet:
  the named choice is "revisitable", with "the canonical exit is TAPL
  option (3), a De Bruijn representation using the shifting/substitution
  operations of TAPL Section 6.2-6.3").
- Tuples and records are rejected by the slice-6b probe translation: records need
  a Core row type (not present), and tuples need arity-tagged constructors
  (decided at slice 7) that would otherwise collide with user `data Tuple ...`
  declarations.
- Mixed casing in rendered types (for example `List int`) is deliberate: `Base`
  leaves render lowercase to preserve the `Lam` goldens (`\x: int. x`).
  Unifying the casing is deferred to slice 6c, when `Lam`'s annotation switches
  from `BaseType` to `Type`.

## Rejected alternatives

- GHC-style absorption: make `Int` a nullary `Con` and retire `BaseType`.
  Rejected because literals, VM primitives and the lowercase goldens are keyed
  on `BaseType`; retiring it would churn slice 6c and the evaluator for no 6b
  benefit.
- Saturated application `Con(String, Vec<Type>)`. Rejected because it cannot
  express variable-headed application `f a` without a later restructure, which
  higher-kinded types require.

## STANDING GUARD

The enum is exercised by extending the existing throwaway elaborator probe
(`crates/parlang/tests/elaborator_probe.rs`). The probe translates inferred
surface types to Core `Type` via `core_type_of`, and translates generalized
surface schemes to Core via a scheme-to-Forall translation (`core_scheme_of`)
that wraps the body in nested `Forall`s, outermost binder first. Tests run over
REAL inference: the generalized schemes of `id` (one Forall) and `konst` (two
nested Foralls) are asserted closed (`free_vars` empty) with renderings built
from the schemes' own variable numbers, the open body type of `id` is checked to
have exactly one free variable, and a corpus sweep asserts that EVERY top-level
`let` decl scheme in `crates/parlang/examples/stdlib.par` translates to a closed
Core type. Unsupported constructs (records, tuples, constrained schemes)
hard-reject. No new integration-test crate is added (each one multiplies compile
memory).

## Consequences

- `Term::Lam` still carries a `BaseType` after 6b; the F1 placeholder is retired
  in slice 6c, when `Lam`'s annotation becomes `Type`.
- Substitution and alpha-equivalence are not yet available; passes needing them
  (Core Lint in 6e, `TyLam` / `TyApp` in 6d) add them when required.

## References

- `docs/adr/0003-core-case-eliminator.md` — Finding F1 (Lam binder `BaseType`).
- `crates/parlang-core/README.md` — the "Named vs De Bruijn" ADR outcome and its
  De Bruijn exit.
- `crates/parlang/tests/elaborator_probe.rs` — the slice-6b pull on `Type`.
- `crates/parlang-core/src/ty.rs` — the `Type` enum and its `Display`.
