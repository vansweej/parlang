# Kernel Eligibility

`parlang kernel-check` reports whether a top-level function could later run as
a GPU kernel. It is an analysis only: the GPU sublanguage is a subset of the
CPU language, there is one Core IR and no GPU backend, and every program still
runs on the CPU interpreter. Design rationale: `docs/adr/0010-kernel-eligibility-predicate.md`.

## Usage

```bash
parlang kernel-check crates/parlang/examples/recursion.par --fn sum_to_n
```

- Eligible: stdout `eligible: <name>`, exit code 0.
- Rejected: stderr `rejected: <name>` followed by one line per violation,
  `  <id> <rule name>: <function>: <Core node>`, exit code 1.
- Unknown function: `kernel-check error: ...`.

The function is looked up on the program's top-level `let` spine (the innermost
binding of the name). Earlier top-level functions it calls are checked too, and
their violations are listed after its own.

## Rules

| Id | Name | Summary |
|----|------|---------|
| R1 | kernel types | `Int`, `Float`, `Bool`, `Char`, `Unit` and R4-allowed data types; no type variables or function values |
| R2 | first-order | only saturated direct calls to itself, join points or eligible top-level functions |
| R3 | recursion | only self tail recursion (a loop) |
| R4 | data | only acyclic data types with kernel-typed fields |
| R5 | polymorphism | generalised functions must be instantiated (not yet supported by the CLI) |
| R6 | reserved | future SOAC operator arguments; never reported today |

Rule ids are stable and never renumbered.

## Examples

An eligible tail-recursive loop (`sum_to_n` and `gcd` in
`crates/parlang/examples/recursion.par` pass):

```
let sum_to_n = rec sum_helper -> fun acc -> fun n ->
    if n == 0
    then acc
    else sum_helper (acc + n) (n - 1);
```

The `factorial` in the same file is rejected by R3 because the recursive call
in `n * f (n - 1)` is not in tail position:

```
rejected: factorial
  R3 recursion: factorial: (f -(n, 1))
```

The accumulator rewrite is eligible:

```
let factorial_acc = rec f -> fun acc -> fun n ->
    if n == 0
    then acc
    else f (acc * n) (n - 1);
0
```

A match on a monomorphic, non-recursive data type is eligible. The join points
`$j<N>` that match compilation introduces are recognised and inherit tail
position:

```
data Shape = Circle Int | Square Int;
let area = fun s -> match s with | Circle r -> r * r | Square w -> w * w;
0
```

Generalised (polymorphic) functions are rejected by R5 in this slice, because
the CLI passes no type instantiation.

A list length function is rejected by R4 (`List` is recursive), R1 (the `List`
parameter), R3 (the non-tail `1 + go rest`) and R5 (it is generalised), among
others:

```
data List a = Nil | Cons a (List a);
let len = rec go -> fun xs -> match xs with | Nil -> 0 | Cons x rest -> 1 + go rest;
0
```

## Future: type classes

Eligibility is defined on specialised Core. Dictionaries introduced by type
classes must be specialised away before the check, and higher-kinded type
variables are never kernel types until instantiated.
