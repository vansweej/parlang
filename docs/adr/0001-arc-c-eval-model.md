# ADR 0001: Arc C Evaluation Model

## Status

Accepted

## Context

Arc C ships a STRICT (call-by-value) tree-walking VM over the existing Arc B
Core `Term` IR (`Var`, `Lam(String, BaseType, Box<Term>)`, `App`, `Let`,
`LetRec`, `Lit`, `Con(String, Vec<Term>)`). The crate `parlang-core` is
dependency-free (std only), targets edition 2021, contains no
`unwrap`/`panic!` in library code, uses checked arithmetic, and uses inline
format args. The central seam for this arc is that Core has **no** `If` node
and **no** arithmetic/comparison/`BinOp` node — the roadmap mandates ONE small
Core, so adding `Term` variants for these concerns is deliberately avoided.

This ADR records the four foundational decisions that shape the Arc C
evaluator.

## Decision

### Decision 1 — Evaluation strategy

We adopt a strict, call-by-value, environment-based tree-walking evaluation
strategy. Environments are explicit substitutions (finite maps from names to
values) rather than syntactic substitution performed directly into terms;
closures capture an environment instead of having their bodies rewritten.
This follows TAPL Ch. 5 (call-by-value operational semantics), Ch. 6
(nameless/De Bruijn representation), and Ch. 7 (an ML implementation of the
evaluator), and is further justified by BRICS-RS-03-13 (Danvy, "A Rational
Deconstruction of Landin's SECD Machine"), which motivates environment
machines over naive substitution-based evaluators.

### Decision 2 — Binder representation (re-evaluation of the Arc B named-vs-De-Bruijn choice)

Arc B chose named binders (TAPL §6 option (1): symbolic names with explicit,
capture-avoiding renaming performed on the fly), explicitly flagging this as
revisitable once a substitution-heavy pass appeared. This ADR re-opens that
choice for Arc C.

Conclusion: because the VM uses environments as explicit substitutions —
closures capture an environment, and no syntactic substitution into term
bodies ever occurs — the on-the-fly capture-avoiding renaming cost that TAPL
warns about for option (1) is **not incurred** here. Variable lookup is
performed by name against the environment, and no term-level
substitution/renaming happens during evaluation. We therefore **retain named
binders** for Arc C.

Exit criterion: switching to a De Bruijn representation (TAPL §6 option (3))
is worthwhile only if a future pass performs syntactic substitution or
normalization under binders directly on terms (e.g., a future optimizer or
partial evaluator). This decision is made BEFORE any such substitution
machinery is built.

### Decision 3 — Primitives & branching encoding = Option A (DECIDED)

The resolved decision is **Option A**: reserved constructor names are
dispatched by the VM rather than adding new `Term` variants — the `Term` enum
stays UNTOUCHED. Reserved names:

- `Con("if", [c, t, e])` — strict in the condition, lazy in the untaken
  branch.
- `Con("+"|"-"|"*"|"/", [a, b])` — checked arithmetic on `Int`.
- `Con("<", [a, b])` — `Int` operands only.
- `Con("eq", [a, b])` — base-type operands only.

**Option B** (adding dedicated `Term::If`/`Term::Prim` AST variants) is the
**rejected alternative**: it mutates the shared AST used by other arcs and
ripples into `display.rs`/`dot.rs`/`builder.rs`/typechecker, whereas Option A
keeps the AST frozen and localizes all VM concerns to `eval.rs`. This fork is
now resolved, not open.

Accepted consequence: a user constructor literally named `if`/`eq`/`+`/etc.
is shadowed by primitive dispatch.

### Decision 4 — Recursion via `RecClosure` re-bound at each application

`Term::LetRec(n, v, b)` requires `v` to be a `Term::Lam(param, _ty, body)`; it
builds `Value::RecClosure { name: n.clone(), param: param.clone(), body: body.clone(), env: env.clone() }`,
capturing the CURRENT env WITHOUT pre-inserting the name into it, binds `n`
to that `RecClosure` in env, and evaluates `b`. If `v` is not a `Lam`,
evaluation returns a `TypeMismatch` error. At `Term::App(f, a)`, after
evaluating `f`: a `Value::Closure { param, body, env: cenv }` evaluates the
argument and evaluates `body` under `cenv.extend(&param, v_arg)`; a
`Value::RecClosure { name, param, body, env: cenv }` evaluates the argument
and evaluates `body` under `cenv.extend(&name, <the RecClosure itself, cloned>).extend(&param, v_arg)`
— RE-INSERTING the name at EVERY application is what makes recursion of
arbitrary depth work; any other callee value yields `EvalError::NotAFunction`.

Consequence: a plain-`Closure` one-shot self-bind (into the captured env at
`LetRec`-eval time) unwinds exactly one level and then raises `UnboundVar`,
which is why `RecClosure` re-binding is required. We recommend `RecClosure`
over an `Rc`/`RefCell` back-patched cell because it stays dependency-free and
needs no interior mutability.

### List operations

List computation is demonstrated honestly via a Church/Scott-encoded fold
(`nil`/`cons`/`sum`), which performs real computation, complemented by a
value-level strict `Value::Con` construction check.

## Consequences

- The `Term` enum remains unchanged from Arc B; all VM-specific concerns
  (primitives, branching, recursion) live entirely in `eval.rs`.
- Named binders keep the evaluator and its captured closures directly
  legible, at the cost of revisiting this decision if a substitution-heavy
  pass is added later (see Decision 2's exit criterion).
- Reserved constructor names (`if`, `+`, `-`, `*`, `/`, `<`, `eq`) shadow any
  user-defined constructor of the same name.
- Recursive functions defined via `LetRec` support arbitrary recursion depth
  because the recursive binding is re-inserted at every application, not
  just once at definition time.

## References

- Benjamin C. Pierce, *Types and Programming Languages* (TAPL), Ch. 5
  (call-by-value operational semantics), Ch. 6 (nameless/De Bruijn
  representation), Ch. 7 (an ML implementation of the evaluator).
- Olivier Danvy, "A Rational Deconstruction of Landin's SECD Machine",
  BRICS-RS-03-13.
- Simon Peyton Jones, *The Implementation of Functional Programming
  Languages* (1987), ch. 3-6 (the `let`/`letrec` split already cited in
  `term.rs`).
