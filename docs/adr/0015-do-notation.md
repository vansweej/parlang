# ADR 0015: Do-Notation

## Status

Accepted (M2.4b).

## Context

M2.4a added constructor classes `Functor` and `Monad`. Chains of
`bind` calls with nested lambdas are hard to read.

## Decision

- Syntax `do { stmt; ...; expr }` with braces and semicolons like class
  bodies; a trailing `;` is allowed; no layout rule.
- The parser desugars a do block into existing nodes: `x <- e; rest`
  becomes `bind e (fun x -> rest)`, `e; rest` binds the name `_` (which
  surface code cannot write), and `let y = e; rest` becomes a `let`.
- An empty block or one not ending in an expression is a parse error.
- `<-` is only recognised in statement position; elsewhere `x <-1`
  still means `x < -1`.

## Alternatives rejected

- An `Expr::Do` node with forced Monad-method resolution: it needs
  changes in the typechecker, elaborator and evaluator for no
  user-visible gain at this stage.

## Consequences

- No new AST, type or Core node; both evaluators get do-notation for
  free.
- `do` is now a keyword.
- Without a `bind` in scope a do block is the type error "unbound
  variable bind".
- `bind` and `return` resolve like any name: a user binding (including
  a top-level `let bind = ...`) captures every later do block
  (rebindable, by design).
- A do block may be a bare function argument: `f do { m }`.
- Deferred: the `>>=` operator, pattern binds, layout,
  Applicative/MonadFail and forced method resolution.
