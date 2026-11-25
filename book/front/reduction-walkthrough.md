# Watching a program reduce

Everything in this chapter is produced by running the interpreter while
the book is built; nothing is pasted by hand. The program is
`crates/parlang/examples/mvp.par`. The test suite runs it on the surface
evaluator (`executes_mvp_example`) and on the Core machine
(`mvp_example_runs_on_core`), both in
`crates/parlang/tests/examples_execution_tests.rs`.

## The source program

```{.parlang include="crates/parlang/examples/mvp.par"}
```

## The typed AST

Output of `parlang mvp.par --dump --stage typed`: the program after
Hindley-Milner inference, every node annotated with its type.

```{.text include="book/build/dumps/mvp-typed.txt"}
```

## The System F Core term

Output of `parlang mvp.par --dump --stage core`: the elaborated,
explicitly typed Core term.

```{.text include="book/build/dumps/mvp-core.txt"}
```

## Core lint

Output of `parlang mvp.par --dump --stage lint`: Core lint re-checks the
elaborated term and reports its type.

```{.text include="book/build/dumps/mvp-lint.txt"}
```

## The result on both evaluators

The surface evaluator (`parlang mvp.par`):

```{.text include="book/build/dumps/mvp-result.txt"}
```

The Core CEK machine (`parlang mvp.par --core`):

```{.text include="book/build/dumps/mvp-core-result.txt"}
```

## A CEK machine trace

A full trace of `mvp.par` is too long to print, so the trace uses the
three-line program `crates/parlang/examples/simple.par`:

```{.parlang include="crates/parlang/examples/simple.par"}
```

Its Core term:

```{.text include="book/build/dumps/simple-core.txt"}
```

And the reduction view printed by `parlang trace simple.par --fuel 10000`:

```{.text include="book/build/dumps/simple-trace.txt"}
```
