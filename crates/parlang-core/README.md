# parlang-core

A small, GHC-style **Core intermediate representation** for ParLang, distinct
from the surface `parlang::ast::Expr`.

## Scope

- System-F-shaped, **explicitly typed** Core term AST
  (`Term`: `Var` / `Lam` / `App` / `Let` / `LetRec` / `Lit` / `Con` / `TyLam` /
  `TyApp`). `Con` carries explicit type arguments and `LetRec` a binder
  annotation; `TyLam` / `TyApp` are erased at runtime (ADR 0006).
- Core-owned base types (`BaseType`: `Int` / `Bool` / `Float` /
  `Unit` / `Char`), used to annotate lambda binders and classify literals.
- A System-F-shaped Core type (`Type`: `Base` / `Var` / `Con` / `Fun` /
  `App` / `Forall`) with ASCII `Display`, `free_vars`, and type builders.
  `BaseType` is absorbed as the `Base` leaf and renders capitalized (`Int`,
  `Bool`, ...). As of slice 6c `Term::Lam` annotates its binder with a full
  `Type`. See `docs/adr/0005-core-type-enum.md`.
- DOT dumper (`core_to_dot`, header `digraph Core`) and a text dump
  (`Term::to_text` / `Display`), mirroring the surface dumpers.
- A tiny, dependency-free builder (`builder` module, smart
  constructors) for hand-writing Core in tests.
- A strict, call-by-value CEK abstract machine with a steppable trace and
  read-back view (`eval`, `Value`, `Environment`, `EvalError`) over `Term`,
  reserved primitives (`if`, `+`, `-`, `*`, `/`, `<`, `eq`), and recursion via
  `LetRec` / `Value::RecClosure`. See ADR 0008.
- A small `prelude()` environment (`not`) built on those primitives.

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
- **Polymorphism:** the System-F Core `Type` enum (forall / type variables /
  type-constructor application) landed in slice 6b
  (`docs/adr/0005-core-type-enum.md`); slice 6c switched the `Lam` annotation
  from `BaseType` to `Type`, and slice 6d added the type-lambda /
  type-application TERMS (`TyLam` / `TyApp`, erased at runtime; ADR 0006).
- **`Char` and strings:** Core carries a `Char` base type — a Unicode scalar
  value, like Haskell's `Char`. Strings are `List Char` under ADR 0002's
  `Cons`/`Nil` encoding; there is no primitive string value. The earlier
  primitive-string false start has been retired. See ADR 0004
  (`docs/adr/0004-core-char-and-strings.md`).
