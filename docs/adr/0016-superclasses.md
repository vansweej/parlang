# ADR 0016: Superclasses

## Status

Accepted (M2.5a, M2.5b, M2.5c).

Builds on `docs/adr/0013-type-classes.md` and
`docs/adr/0014-dictionary-passing.md`.

## Context

ADR 0013 parsed no superclasses, so a class such as `Ord` could not
reuse `Eq`. Constrained code had to list every class separately and
could not derive one dictionary from another.

## Decision

- Syntax: `class [ctx =>] C a where { ... };`. The context uses the
  instance-context syntax, `S a =>` or `(S a, T a) =>`.
- A superclass constraint must be on the class parameter, and each
  superclass is listed once (`InvalidSuperclassContext`). The
  superclass must already be declared (`UnknownClass`), so
  self-reference and cycles are impossible. Its parameter kind must
  equal the class parameter kind (`KindMismatch`).
- Entailment: the givens of an instance method are the instance
  context closed under superclasses (`TypeEnv::superclass_closure`).
- An instance needs instances of all superclasses at its head, declared
  earlier in source order, whose contexts follow from the instance
  context closed under superclasses (`NoInstance` otherwise).
- There is no constraint simplification. A function that uses `==` and
  `lte` gets `(Eq a, Ord a)`, and both dictionaries are passed.
- The typed dump prints a `superclasses: Eq` line for a class with
  superclasses.
- Dictionaries: the `$super_<S>` fields come first, in the order of
  `ClassInfo.superclasses`, followed by the methods. This holds both
  for surface records and for the fields of the Core `$Dict_C p`
  constructor, where each superclass field has type `$Dict_S p`.
- Evidence search tries three things in order:
  1. an exact given;
  2. a projection out of a given whose class has the wanted class as a
     transitive superclass at identical arguments
     (`TypeEnv::superclass_path`, the shortest path found by BFS).
     `Evidence::Super` renders as a chain of `FieldAccess` on
     `$super_S` fields;
  3. an instance.
- On Core, superclass selection is a `Case` without `TyApp`
  (`elab_super_select`). Method field positions are offset by the
  number of superclasses.

## Alternatives rejected

- Built-in `Show` and `Ord` now. A built-in `Show` that replaces the
  renderer needs a built-in List/String prelude. A built-in `Ord` would
  retype `<`, `>`, `<=` and `>=`. That would make `max`, `min` and the
  treemap code constrained, so they would no longer be kernel-eligible
  under ADR 0010 R5 until a Core specialiser exists.
- Constraint simplification now. It is not needed for correctness and
  would change inferred types.
- Order-independent instances. They need a separate pass over all
  instance declarations and would break source-order visibility.
- Flattening superclass methods into the subclass dictionary. It breaks
  the uniform projection of a superclass dictionary and duplicates
  fields along diamonds.

## Consequences

- `examples/ord.par` declares a user `class Eq a => Ord a`, a
  `class Ord a => Top a` and `Box` instances with contexts. It
  evaluates to `11197` on the surface evaluator and on Core.
- New `TypeError` variant: `InvalidSuperclassContext`.
- Superclass dictionaries add one field per superclass to every
  dictionary of a subclass.
- Deferred (roadmap amendment 9): built-in `Show` replacing the
  renderer (needs a built-in List/String prelude, typed modules/std
  milestone); built-in `Ord` retyping `<`, `>`, `<=` and `>=`;
  constraint simplification; order-independent instances; REPL
  persistence of superclass instances across lines (untested);
  multi-parameter classes. Next: the operators backlog (named
  operators, sections, backtick infix, fixity, `>>=` as an alias for
  `bind`).

## References

- `crates/parlang/src/parser.rs` (`class_decl`)
- `crates/parlang/src/typechecker.rs` (`resolve_superclasses`,
  `check_superclass_instances`, `superclass_closure`,
  `superclass_path`)
- `crates/parlang/src/dictionary/evidence.rs`
- `crates/parlang/src/dictionary/program.rs`
- `crates/parlang/src/elaborate/dictionary.rs`
- `crates/parlang/examples/ord.par`
- `docs/adr/0013-type-classes.md`
- `docs/adr/0014-dictionary-passing.md`
