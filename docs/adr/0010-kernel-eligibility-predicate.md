# ADR 0010: Kernel Eligibility Predicate

## Status

Accepted

Implements the predicate deferred by `docs/adr/0003-core-case-eliminator.md`.

## Context

The roadmap fixes the GPU sublanguage as a subset of the CPU language: one
Core IR, no separate kernel IR and no GPU backend. Every program still runs on
the CPU interpreter. Roadmap slice 8 needs a precise, testable statement of
which Core functions could later run as GPU kernels. The constraints follow the
SIMT model described by Henriksen (Futhark PhD thesis, ch. 4): no dynamic
parallelism and no general recursion inside kernels.

## Decision

- Kernel eligibility is a pure predicate over Core:
  `Core -> Eligible | Rejected(violations)`. It never transforms the term.
- The unit of analysis is a top-level function on the root `Let` spine of the
  elaborated program (the innermost binding of the name). Every earlier spine
  binding it references is checked transitively; callee violations are listed
  after the function's own. Unknown functions and malformed Core (for example
  an unbound variable) are errors, not rejections.
- Rule ids are stable and never renumbered:
  - R1 kernel types: base types `Int`, `Float`, `Bool`, `Char`, `Unit`, and
    R4-allowed data types applied to kernel types. No type variables, foralls
    or function types, except a function's own leading parameters, a self loop
    and join points.
  - R2 first-order: only saturated direct calls to the self loop, join points
    or eligible earlier top-level functions; no other lambdas; a join call
    must be a jump.
  - R3 recursion: only self tail recursion (a loop). Tail position flows
    through `if` branches, case alternatives, let bodies and join-point bodies.
  - R4 data: constructors and case only on data types with an acyclic datatype
    graph and kernel-typed fields (tuples, records, Option-like ADTs pass;
    `List`, strings and trees fail).
  - R5 polymorphism: a generalised function must be instantiated. The CLI
    passes no instantiation in this slice, so generalised functions and
    generalised callees are rejected.
  - R6 reserved for future SOAC operator arguments (map/reduce/scan); never
    produced today.
- Join points are let-bound lambdas created by match compilation, named
  `$j<N>` via the shared constant `parlang_core::eligibility::JOIN_POINT_TAG`.
  They are recognised explicitly, and their bodies inherit the match's tail
  position.
- The CLI surface is `parlang kernel-check <file> --fn <name>`.

## Consequences

- Milestone 2 contract: eligibility is defined on specialised Core. When
  typeclasses arrive (dictionary passing, that is records of functions),
  dictionaries must be specialised or inlined away before the check, otherwise
  R1 and R2 reject every constrained function. Higher-kinded type variables
  (`Type::App(Var f, a)`) are never kernel types until instantiated. The
  predicate API already takes an instantiation list for this reason.
- Per-instantiation checking (lifting R5) is future work.
- Non-tail recursion such as a naive factorial is rejected and needs an
  accumulator rewrite; see `docs/KERNEL_ELIGIBILITY.md`.
