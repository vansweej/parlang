# ADR 0007: Core Explicit Typing and Lint

## Status

Accepted. Builds on ADR 0005 (Core `Type` enum) and ADR 0006 (System-F term
erasure); consistent with ADR 0003 (Core `Case` eliminator) and ADR 0001
(strict Core VM).

## Context

Slice 6 introduced a System-F-shaped `Type` enum (ADR 0005) and the
corresponding term-level constructs `TyLam`/`TyApp` plus explicit `Con` type
arguments and a `LetRec` binder annotation (ADR 0006). With those pieces in
place, Core needs a type checker. The central design question is whether that
checker performs Hindley-Milner-style inference (unification, fresh
metavariables, generalization) or is a simple, pure checker that merely
verifies annotations already present in the term.

## Decision

Core is made **explicitly typed**: every node that introduces a type decision
already carries enough annotation for its type to be *synthesized* rather than
*inferred*. Concretely:

- `Term::Lam`'s binder carries a full `Type` (already true from slice 6c).
- `Term::Con`'s explicit type arguments (already true from slice 6d) fully
  determine a data constructor's instantiation; no unification against field
  types is needed beyond substitution.
- `Term::LetRec`'s recursive binder carries a `Type` annotation (already true
  from slice 6d), so the recursive occurrence's type is known before its body
  is checked, rather than having to be inferred via a fixpoint.

Given these annotations, **Core Lint** (`crates/parlang-core/src/lint.rs`) is
implemented as a **pure bidirectional type checker with no unification and no
metavariables**: it is purely a `synth` (type synthesis) judgement over
explicitly-typed terms, with no `check`-against-an-expected-type judgement
needed because every node either carries its own annotation or inherits its
type from already-synthesized children. This is a deliberately minimal
design: Lint's job is to CONFIRM that an already-elaborated (or hand-built)
term is well-typed, not to RECONSTRUCT missing types the way the surface
Hindley-Milner inferencer (`crates/parlang/src/typechecker.rs`) does.

`Let` remains the one UNANNOTATED binder (as `Term::Let` carries no type
field): its bound value's type is synthesized and used directly as the
binding's type when checking the body. This is a deliberate asymmetry with
`Lam`/`LetRec`, which do carry annotations: `Let` has no self-reference to
resolve (unlike `LetRec`) and no call-site argument to check against (unlike
`Lam`), so there is nothing for an annotation on `Let` to usefully constrain
beyond what synthesis already determines.

`Case` alternative binders (ADR 0003) are likewise left **unannotated** at
the term level: an `Alt`'s `binders: Vec<String>` carry no type annotation.
Lint derives each binder's type positionally from the data environment: the
owning data type's declared field types, with the data type's own parameters
substituted by the scrutinee's instantiated type arguments. This keeps `Case`
terms exactly as compact as ADR 0003 specified (no redundant per-binder
annotations that would just repeat the data declaration), while still giving
Lint everything it needs via the data/kind environment rather than via
unification.

Substitution and alpha-equivalence, used throughout Lint (for `TyApp`'s
beta-reduction, for comparing an argument's type against a parameter/field
type, and for comparing two branches' types), are the capture-avoiding,
fresh-renaming `Type::subst` and `Type::alpha_eq` operations over the
**named** type-variable representation introduced in slice 6d. ADR 0005
flagged named binders as *revisitable*, with the canonical exit being a De
Bruijn representation (TAPL option (3)) if a substitution-heavy pass ever
needed it. Lint IS such a pass, and it is built directly on the named
`subst`/`alpha_eq` rather than on a De Bruijn representation: the named
machinery (with on-the-fly fresh-variable renaming to avoid capture) is
sufficient and keeps dumps/diagnostics legible, so the De Bruijn exit is
(again) not taken.

## Consequences

- Because Lint only accepts explicitly-typed (and therefore necessarily
  well-typed, once Lint-clean) Core, the `eq` primitive's runtime behavior on
  **mismatched** base-type operands (e.g. comparing an `Int` to a `Bool`) can
  never actually occur in Lint-clean Core: such a term is rejected by Lint's
  `eq` rule (`EqTypeMismatch`/`NonBaseEq`) before it would ever reach `eval`.
  Consistent with this, `eval`'s `eq` primitive now returns a hard
  `TypeMismatch` error on mismatched base operands rather than silently
  returning `false` (follow-up #1): the old permissive behavior was a vestige
  of untyped Core and is no longer reachable once a term has passed Lint, so
  tightening it costs nothing for well-typed programs and gives a clearer
  error for hand-built ill-typed ones.
- Lint adds no new `Term` constructs and performs no rewriting: it is a
  read-only checker (`lint(term, env) -> Result<Type, LintError>`) that can be
  run optionally before `eval`, exactly mirroring how the surface
  `typecheck`/`eval` split works, but without any unification machinery on
  the Core side.
- A standing guard exercises Lint end-to-end: the prelude's `not` function,
  a `letrec`-based factorial, a `TyLam`/`TyApp` polymorphic identity
  instantiated at two different base types, and a `List` length function
  compiled via `Case` are each linted (asserting the expected synthesized
  type) AND separately evaluated (asserting the expected runtime value),
  confirming that Lint-accepted programs are also programs `eval` runs
  correctly.

## References

- `docs/adr/0005-core-type-enum.md` — the `Type` enum and its named-binder
  `subst`/`alpha_eq` exit.
- `docs/adr/0006-core-system-f-term-erasure.md` — `TyLam`/`TyApp` erasure at
  runtime.
- `docs/adr/0003-core-case-eliminator.md` — the `Case` eliminator's exhaustive,
  unannotated-binder shape.
- `docs/MODULE_CORE_LINT.md` — the Lint module's full typing-rule reference.
