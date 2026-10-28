# parlang-core

A small, GHC-style **Core intermediate representation** for ParLang, distinct
from the surface `parlang::ast::Expr`.

## Scope

- System-F-*shaped* but **monomorphic** Core term AST
  (`Term`: `Var` / `Lam` / `App` / `Let` / `LetRec` / `Lit` / `Con`).
- Core-owned base types (`BaseType`: `Int` / `Bool` / `Float` /
  `Unit` / `String`), used to annotate lambda binders and classify literals.
- DOT dumper (`core_to_dot`, header `digraph Core`) and a text dump
  (`Term::to_text` / `Display`), mirroring the surface dumpers.
- A tiny, dependency-free builder (`builder` module, smart
  constructors) for hand-writing Core in tests.
- A strict, call-by-value, environment-based evaluator (`eval`, `Value`,
  `Environment`, `EvalError`) over `Term`, with reserved primitives
  (`if`, `+`, `-`, `*`, `/`, `<`, `eq`, `strlen`, `strcat`), recursion via
  `LetRec` / `Value::RecClosure`, and checked arithmetic.
- A small `prelude()` environment (`not`, `isEmpty`, `nonEmpty`) built on
  those primitives.

Core now includes a `Case` tag-switch eliminator (one alternative per
constructor; see `docs/adr/0003-core-case-eliminator.md`). A full elaboration
pass (surface -> Core) is not yet shipped; a deliberately throwaway elaborator
probe lives in `crates/parlang/tests/elaborator_probe.rs` and exercises the
`Case` shape against real `match` compilation.

## ADR outcomes

- **Named vs De Bruijn:** Core uses a **named** representation — TAPL Section 6
  ("Nameless Representation of Terms", p.97) *option (1)*: symbolic names with
  explicit, capture-avoiding renaming on the fly. Its original consumers (the
  builder and the dumpers) benefit far more from hand-writability and
  dump-readability than from De Bruijn's capture-safe substitution, whose
  payoff only arrives with substitution-heavy passes. This remains
  **revisitable**: the evaluator performs beta-reduction with
  environment-capturing closures rather than substitution, so the on-the-fly
  renaming cost TAPL describes for option (1) has not been incurred. Should a
  substitution-based pass ever be added, the canonical exit is TAPL option (3),
  a De Bruijn representation using the
  shifting/substitution operations of TAPL Section 6.2-6.3 (p.80-81).
  References: SPJ, *Implementation of Functional Programming Languages*
  (1987), ch.3-6 (enriched lambda calculus, explicit `let`/`letrec`); TAPL
  ch.6.
- **`let` vs `letrec`:** kept as **distinct nodes** (SPJ ch.3-6) so recursion
  is explicit for later stages, orthogonal to the binder-naming decision.
- **Builder vs parser:** smart-constructor **builder** only — no `combine`
  dependency added to this zero-dependency crate.
- **Polymorphism:** System-F forall / type variables / type-lambda / type
  application are **deferred** to a later sub-step; the `BaseType` binder
  annotation and module notes mark the seam.
- **`String` base type:** Core carries a `String` base type with **no surface
  counterpart yet**. When elaboration lands, the elaborator must treat
  Core `String` as having no surface source — a partiality to handle
  deliberately.
