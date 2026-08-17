# ADR 0002: Arc C4 List Encoding

## Status

Accepted

## Context

Arc E, Arc F, and Arc G require a stable, frozen encoding for list values so
that later arcs (elaboration, GPU kernel eligibility analysis, etc.) can rely
on a single canonical shape rather than re-deriving one ad hoc. Arc C4 itself
ships no list code: no list construction, no list deconstruction, and no
list-related primitives are introduced by Arc C4. This ADR exists purely to
record the encoding decision ahead of time, so it is recorded-but-not-yet
-exercised by any Arc C4 code path.

## Decision

The frozen list encoding is:

- `Con("Cons", [head, tail])` for a non-empty list cell, and
- `Con("Nil", [])` for the empty list.

This mirrors the constructor-application shape already used elsewhere in the
Core `Term`/`Value` model (`Term::Con(String, Vec<Term>)` /
`Value::Con(String, Vec<Value>)`), so lists are ordinary constructor values
with no special-cased `Term` or `Value` variant.

List deconstruction — pattern matching or an equivalent eliminator over
`Cons`/`Nil` — is explicitly **deferred to Arc F**, rather than being solved
now with a host primitive. Introducing a host-level list-deconstruction
primitive in Arc C4 would enlarge the backend and widen the GPU
kernel-eligibility surface prematurely, before Arc F's elaboration work has
settled how pattern matching over constructors is compiled in general. Since
Arc C4 ships no list code at all, there is nothing to eliminate yet, and no
motivation to special-case `Cons`/`Nil` ahead of that general mechanism.

This encoding is revisitable: if a future arc's needs are not well served by
plain `Con("Cons", ...)`/`Con("Nil", [])` values (for example, if GPU
kernel-eligibility analysis wants a distinguished list representation), this
decision can be reopened without disturbing the frozen `Term`/`Value` shapes
recorded in ADR 0001, since the encoding lives entirely at the constructor-name
level.

## Consequences

- `Cons`/`Nil` are ordinary reserved constructor names, not new `Term` or
  `Value` variants; the Core AST and value domain established in Arc B/C
  remain unchanged.
- Arc C4 records this encoding but does not exercise it: no Arc C4 code
  constructs or destructures `Cons`/`Nil` values.
- Arc E, Arc F, and Arc G can rely on `Con("Cons", [head, tail])` /
  `Con("Nil", [])` as the canonical list shape without needing to invent or
  negotiate an encoding themselves.
- List deconstruction (pattern matching or an eliminator) is explicitly out
  of scope until Arc F, avoiding a premature host primitive that would
  enlarge the backend and GPU kernel-eligibility surface.

## References

- ADR 0001 (`0001-arc-c-eval-model.md`), Decision 3: reserved constructor
  names dispatched by the VM, keeping the `Term` AST frozen.