# ADR 0003: Core Case Eliminator

## Status

Accepted

## Context

Core (`parlang-core`) is strict and deliberately minimal. Until now it had no
constructor-eliminating term: ADR 0002 deferred list deconstruction, and no
general mechanism for matching on constructors existed. Roadmap slice 5
required deciding the shape of Core's eliminator before the real elaborator
(slice 7) is built.

## Decision

Core gains a single low-level tag-switch eliminator:

````
Term::Case(Box<Term>, Vec<Alt>)
Alt { con: String, binders: Vec<String>, body: Term }
````

Each alternative names one constructor and binds its fields positionally with
unannotated binders (like `Let`, unlike `Lam`). Patterns are pre-flattened by
elaboration; rich patterns and exhaustiveness checking stay in the
surface/elaborator. There is one alternative per constructor of the scrutinee's
type and no default alternative: tables are exhaustive over the complete
constructor family (Wadler 1987, ch. 5).

Rejected alternatives:

- **Church/Scott encodings** of data: they hide the tag from later analysis and
  are hostile to a strict backend and to kernel-eligibility reasoning.
- **A rich `Case`-with-patterns term** (nested patterns, guards) in Core: this
  would duplicate surface machinery in Core and widen the frozen IR.

This supersedes ADR 0002's deferral of constructor deconstruction: a tag-switch
`Case` is the general eliminator ADR 0002 pointed at.

## Consequences

- Core now has exactly one eliminator; `Cons`/`Nil` and all ADTs are matched
  through `Case`. The `Term`/`Value` constructor-value shape (ADR 0001, ADR
  0002) is unchanged.
- **Deferred GPU/kernel-eligibility debt:** a tag-switch is analysable by a
  kernel-eligibility predicate (finite, closed set of branches, no hidden
  control flow), so this decision does not foreclose kernel eligibility.
  Nothing about kernels is decided now; the predicate itself is future work.
- A throwaway monomorphic elaborator probe (roadmap slice 5) was built in
  `crates/parlang/tests/elaborator_probe.rs` to answer one question: does this
  `Case` shape survive contact with real elaboration of `match`? The probe's
  differential tests (the four example programs plus nested, overlapping,
  non-variable-scrutinee and nested-match cases) and a shape test (`match`
  elaborates to a `Case` with positional binders) pass against this shape. The
  probe is disposable and will be deleted by an operator.
- **Finding F1:** Core's `Lam` requires a `BaseType`, which cannot express ADT-
  or function-typed binders. The probe maps `Int`/`Bool` faithfully and uses
  `BaseType::Unit` as an inert placeholder otherwise. Slice 6's System-F binder
  types replace `BaseType` on `Lam` and retire this placeholder. Results are
  unaffected because the annotation is inert at runtime.
- **Finding F2 (definite):** the backtracking-free, column-wise specialisation
  keeps default rows in every alternative, so a shared arm body is duplicated
  across `Case` alternatives. This is acceptable for a spike; the real
  elaborator needs join points (let-bound continuations) or a decision-tree
  sharing strategy to avoid the duplication.
- **Finding F3:** Core has no `error`/FAIL term, so a non-exhaustive match can
  only be rejected at compile time (the probe raises `NonExhaustive` on an empty
  matrix); there is no runtime failure value to emit into a `Case`. This is
  consistent with roadmap follow-up #4 (blocking exhaustiveness in the surface).

## References

- Simon Peyton Jones, *The Implementation of Functional Programming Languages*
  (1987), ch. 5 (Wadler, "Efficient Compilation of Pattern-Matching"), ch. 4.4.
- Luc Maranget, "Compiling Pattern Matching to Good Decision Trees" (2008).
- ADR 0001 (`0001-arc-c-eval-model.md`): frozen `Term`/`Value` model.
- ADR 0002 (`0002-arc-c4-list-encoding.md`): list encoding; deconstruction now
  decided here.
