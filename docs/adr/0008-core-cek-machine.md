# ADR 0008: Core CEK Machine

## Status

Accepted

## Context

The original Arc C implementation used a recursive big-step evaluator. It
preserved strict call-by-value environment semantics, but recursive host calls
made individual evaluation transitions invisible and tied evaluation depth to
the native stack.

## Decision

The Core evaluator is a CEK machine derived from the big-step evaluator by the
functional correspondence: CPS-transform the evaluator and then
defunctionalize its continuations into the `Frame` enum. This follows
Ager/Biernacki/Danvy/Midtgaard, BRICS-RS-03-13. The machine state is
`<control, rho, k>`, where `control` is a term or value, `rho` is an
environment, and `k` is a continuation stack.

The CEK machine replaces the recursive evaluator. `eval` now delegates to
`run(term, env, None)`, retaining the strict call-by-value environment
semantics and error behavior of ADR 0001 Decision 1.

`StepKind` has the following taxonomy:

- Administrative: `Lookup`, `Construct`, and `Push`.
- Reductions: `Beta`, `Unfold`, `Delta`, `Branch`, `CaseSelect`, `LetBind`,
  and `Erase`.
- Terminal: `Halt`.

There is no `Pop` kind: returning through a frame is classified by the
transition it enables. `AppCall(RecClosure)` performs the `Unfold` transition,
re-binding the recursive name at every application as required by ADR 0001
Decision 4.

`State::readback` unloads a CEK state into a Core `Term` by plugging control
into `Frame`s with `plug` and by applying `subst_env`. Runtime values read back
as closed terms. This supplies both the raw machine view and a textbook
term-reduction view for a trace.

`Environment` is an `Rc` scope chain. `extend` is O(1), and `PartialEq` is a
manual comparison of visible map-shaped bindings. `run` and `trace` accept
optional fuel and report `EvalError::StepLimit` when the transition limit is
exhausted.

## Consequences

Evaluation no longer depends on native recursive calls, and clients can step,
trace, display, and read back evaluation. The big-step realization from ADR
0001 is retired, while its strict call-by-value environment semantics remains
unchanged.
