# ADR 0013: Type Classes (Haskell 98, single-parameter)

## Status

Accepted

Builds on `docs/adr/0012-surface-higher-kinded-types.md` and
`docs/adr/0010-kernel-eligibility-predicate.md` (the specialised-Core
contract). Implements slices M2.2a, M2.2b1, M2.2b2a and M2.2b2b-1.

## Context

ParLang had `==` and `!=` as ad hoc polymorphic operators and no way to
declare overloaded functions. Higher-kinded types (ADRs 0011 and 0012)
make constructor classes possible. The older design in
`docs/HIGHER_KINDED_TYPES_DESIGN.md` used a `typeclass` keyword and is
superseded by this ADR.

## Decision

- Classes have exactly one parameter. Its kind is inferred from the
  method signatures, so a constructor class has a parameter of kind
  `* -> *`.
- Syntax: `class [ctx =>] C a where { m : ty; ... };` and
  `instance [ctx =>] C head where { m = expr; ... };`. Contexts are
  `C a =>` or `(C a, D b) =>`. `class`, `instance` and `where` are
  keywords.
- Coherence is Haskell 98 strict. An instance head is a base type, a
  bare data type, or a data type applied to distinct type variables.
  Type synonym heads are rejected. Overlapping or duplicate instances
  are rejected, built-ins included. There is no defaulting: an
  ambiguous constraint is an error, and so is a missing instance.
- `==` and `!=` mean the built-in class `Eq` with method `eq`. Its
  built-in instances are `Int`, `Bool`, `Char` and `Float`. `==` on
  tuples, lists and strings is now a compile-time `NoInstance` error.
- A user binding named like a method shadows the method. `==` always
  means `Eq`.
- The program body is implicitly generalised, as in GHCi: a residual
  constraint is accepted when its variables occur in the body type.
- Classes, methods and instances are visible in source order, only
  after their declaration.
- There are no default methods. Every method is defined exactly once
  per instance.
- Instance method bodies are checked against the class signature at the
  head. Rigidity is enforced by substitution, without skolem types. The
  givens are the instance context closed under superclasses (ADR 0016);
  a missing context constraint is reported as `NoInstance`.
- Context reduction is a worklist of one-way head matching.
- Execution: implemented by `docs/adr/0014-dictionary-passing.md`.
  One dictionary insertion mechanism feeds the surface evaluator, both
  REPLs and Core elaboration, so class programs run on every front
  end. It honours ADR 0010, because eligibility is decided on
  specialised Core. Classes combined with `load` are rejected
  uniformly.

## Consequences

- New `TypeError` variants: `NoInstance`, `AmbiguousConstraint`,
  `DuplicateClass`, `DuplicateMethod`,
  `MethodSignatureMissingClassParam`, `UnknownClass`,
  `InvalidInstanceHead`, `TypeSynonymInstance`, `DuplicateInstance`,
  `InvalidInstanceContext`, `UnknownInstanceMethod`,
  `DuplicateInstanceMethod`, `MissingInstanceMethod` and
  `InstanceMethodTooSpecific`. M2.5 added `InvalidSuperclassContext`.
- `ClassesNotSupportedYet` is removed. `EvalError::ClassesNotExecutable`
  stays for the untyped `load` path.
- Programs that compared tuples, lists or strings with `==` no longer
  type-check.
- Known limitations and roadmap:
  - M2.4a (done): context classes are kind-checked against their head
    variable; constructor classes such as `Functor` and `Monad` run over
    `List` and `Option` (see `examples/monad.par`). M2.4b: do-notation (done, see `docs/adr/0015-do-notation.md`).
  - M2.5 (done): superclasses, see `docs/adr/0016-superclasses.md`
    and `examples/ord.par`. A built-in `Show` is deferred.
  - REPL rigidity is covered through the free variables of the
    environment.
  - Execution limitations are listed in ADR 0014.

## References

- `docs/MODULE_TYPECHECKER.md` (Type Classes and Methods, Instances)
- `docs/MODULE_ELABORATE.md`
- `docs/adr/0009-surface-to-core-elaboration.md`
- `docs/adr/0010-kernel-eligibility-predicate.md`
- `docs/adr/0011-core-kinds.md`
- `docs/adr/0012-surface-higher-kinded-types.md`
- `docs/HIGHER_KINDED_TYPES_DESIGN.md` (its type-class parts are
  superseded)
