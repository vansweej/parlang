# Elaborate Module (`crates/parlang/src/elaborate.rs`)

## Purpose

This module translates a type-checked `TypedProgram` into an explicitly typed
Core IR term (`parlang_core::Term`), using the `parlang_core::builder` smart
constructors. The result is checked with `check_elaborated`, which runs Core
Lint on the elaborated term and compares the synthesized type against the
expected surface-derived type.

## Public API

- `elaborate_program(program: &TypedProgram) -> Result<Elaborated, ElabError>`
  elaborates a type-checked program into Core.
- `check_elaborated(elab: &Elaborated) -> Result<CoreType, ElabError>` runs
  Core Lint on the elaborated term and checks its synthesized type against the
  expected surface-derived type.
- `Elaborated` carries the elaborated `term: Term`, the `env: LintEnv` the term
  needs (user declarations plus synthesized tuple and record types), and `ty:
  CoreType`, the surface program type converted to Core with residual type
  variables defaulted to `Unit`.
- `ElabError` enumerates every way elaboration or checking can fail:
  `Unsupported` (a construct not yet supported, naming the construct and a
  reason), `Data` (an invalid data declaration encountered while registering
  types), `Lint` (the elaborated term failed Core Lint), `TypeMismatch` (the
  Core-checked type does not match the expected surface type), and `Build` (a
  term failed to build).

## Type translation

- Base types (`Int`, `Bool`, `Char`, `Float`, `Unit`) translate directly to
  the corresponding Core base type.
- `Fun(a, b)` translates to a Core function type over the translated `a` and
  `b`.
- Type variables are translated with respect to a scope of surface variable
  ids bound by enclosing `TyLam`s. A variable bound in that scope becomes a
  named Core type variable `tN` (where `N` is the surface variable's id).
  A variable NOT in that scope (a residual, never-instantiated type
  variable) defaults to `Unit`.
- An applied type variable `f a` (`VarApp`) whose head is in scope becomes a
  left-nested Core application headed by `Var(tN)`. A residual head is
  rejected with an `Unsupported` error.
- Sum types translate to a left-nested Core type application: the type's name
  as the head, applied in order to each translated type argument. Partial
  application (fewer arguments than parameters, as in `Wrap Option Int`) is
  allowed and kind-checked by Core Lint; over-application is rejected with an
  `Unsupported` error naming the type and both argument counts.
- Tuples of arity two or more translate through a synthesized tuple type
  constructor, named `(,)`, `(,,)`, and so on (one comma per additional
  component beyond the first), with type parameters named `p1` through `pn`.
  Each synthesized tuple type is registered in the `LintEnv` lazily, the first
  time a tuple of that arity is encountered.
- Records translate through a synthesized record type constructor, named by
  the record's field names joined with commas inside braces, for example
  `{age,name}`, with the fields sorted alphabetically before being joined.
  Each synthesized record type is registered in the `LintEnv` lazily, the
  first time a record with that exact sorted field-name set is encountered.

## Data declarations

Surface `TypeAnnotation`s attached to data constructors are converted to Core
types by a mapping that mirrors the surface `type_annotation_to_type`
function: only the names `Int`, `Bool`, `Char`, and `Float` are recognized as
concrete base types; any other concrete name becomes a nullary Core type
constructor reference. This mapping performs no alias resolution of its own;
the typechecker rejects type aliases and unknown names in data payloads at
declaration time, so a data declaration's payload annotations contain only
base types, declared data types, and type variables. The parameter kinds
come from the typechecker (`param_kinds` on `TypedDecl::Data` and
`TypedExprKind::TypeDef`). Each data
declaration is registered into the `LintEnv` through
`LintEnv::try_with_data`, which validates the declaration (rejecting
duplicate type names, duplicate type parameters, and duplicate constructor
names, and checking that every field type is well-formed) before accepting
it.

## Term translation table

- **Literals** (`Int`, `Bool`, `Char`, `Float`) translate to the
  corresponding Core literal builder.
- **`Var`** translates to a Core variable reference, followed by a `TyApp`
  for each of the variable occurrence's instantiation type arguments, applied
  in the same order those type arguments appear on the typed variable node.
- **`fun`** (lambda) translates to a Core `Lam`, with the parameter's domain
  type taken from the function's own `Fun` type, and the body elaborated
  recursively.
- **Application** translates to a Core `App` of the elaborated function term
  to the elaborated argument term.
- **`let`** elaborates its value under the generalization recorded by the
  typechecker: for each of the let's generalized type variables, the
  elaborated value is wrapped in a Core `TyLam` whose kind is inferred from
  the scheme by `kind_check::infer_type_var_kinds`, with the first generalized
  variable ending up as the outermost `TyLam`. The body is then elaborated
  and bound using a Core `Let` over the (possibly `TyLam`-wrapped) value.
- **`rec`** elaborates its function body and requires the result to already
  be a Core `Lam` (since the surface `rec` form is always applied to a
  function); it is then wrapped using a Core `LetRec` with the recursive
  binder annotated by the `rec` node's own type, and the body of the
  `LetRec` is simply a reference back to the bound name.
- **`if`** elaborates its condition, then-branch, and else-branch, and
  combines them using the reserved `if` Core constructor application.
- **Binary operations** elaborate both operands and combine them using the
  corresponding reserved Core arithmetic or comparison constructor for `+`,
  `-`, `*`, `/`, and `<`. The operators `>`, `<=`, and `>=` are elaborated as
  rewrites in terms of `<`: `>` swaps the operand order passed to `<`; `<=`
  and `>=` compute the opposite strict comparison and negate it with `if`.
  These operand-order swaps and rewrites are safe specifically because Core
  is a pure language: reordering or restructuring the evaluation of operand
  terms has no observable effect beyond the resulting value, so the rewrites
  preserve program meaning exactly.
- **Equality** (`==`, `!=`) elaborates to the reserved `eq` Core constructor
  application directly; `!=` elaborates `eq` and then negates the result with
  `if`. The type checker only admits `Eq` at `Int`, `Bool`, `Char` and
  `Float`, so outside a constrained binding or an implicitly generalized
  program body the left operand's translated type is a Core base type; any
  other type is still rejected as unsupported.
- **Tuples** of arity two or more elaborate to an application of the
  synthesized tuple constructor to the elaborated tuple elements, with the
  tuple's type arguments taken from its own component types; the empty tuple
  elaborates directly to the Core unit value.
- **Records** elaborate to an application of the synthesized record
  constructor, with fields supplied to the constructor in sorted order
  (matching the order the record's type was registered in), regardless of
  the field order written in the surface source.
- **Tuple projection** and **field access** both elaborate through a Core
  `Case` over the synthesized tuple or record constructor, with a single
  alternative that binds every component or field positionally (using
  freshly generated names) and whose body is simply a reference to the
  selected binder.
- **Constructors** (user-defined data constructor applications) elaborate to
  an application of the named Core constructor, supplying the constructor's
  own instantiated type arguments (taken from the constructor application's
  own sum type) and the elaborated constructor arguments.
- **Type aliases** elaborate only their body; the alias name and its
  right-hand-side type expression carry no runtime or Core-level meaning and
  are dropped during elaboration.
- **`TypeDef`** (an inline data type declaration expression) registers its
  data type into the `LintEnv` exactly as a top-level data declaration does,
  then elaborates its body.
- At the **program level**, each top-level let declaration is elaborated (with
  the same generalization wrapping described above) and then nested, via
  Core `Let`, around the elaboration of the program's trailing body, with
  declarations nested in reverse declaration order so that earlier
  declarations remain in scope for later ones and for the body. A program
  with no trailing body elaborates to the literal integer `0` in its place.

## Fresh names

Some elaboration steps (projection and field access, in particular) need to
introduce binder names that cannot collide with any name written in the
surface source. These fresh names are generated with a reserved `$` prefix
followed by a short descriptive tag and a counter, for example `$p0` or
`$f1`. Because the surface language's identifiers can never begin with `$`,
no fresh name generated this way can ever collide with a user-written name.

## Pattern matching

Match expressions are compiled to Core decision trees by `elaborate/match_compile.rs`.

- The scrutinee is bound once to a fresh `$s` variable, because Core is strict.
- Arms form a clause matrix. Tuples and records become single constructors of synthetic types; records use their full sorted field set, and missing fields are wildcards. The unit pattern is a wildcard.
- Column heuristic: the leftmost refutable column of the first row.
- Data constructors become one `Case` with one alternative per constructor, in declaration order.
- Bool literals become `if`.
- Int and Char literals become an `if (eq ..)` chain over the distinct literals in first-occurrence order, ending in the default matrix.
- Pattern variables are bound with `let` at the leaf. If a pattern binds a name twice, the last occurrence in source order wins, matching the surface evaluator.
- Arm bodies reached from more than one leaf are bound once as join points `$j<n>`. A join point is a lambda over the arm's variables, or over a `Unit` parameter when the arm binds none, and each leaf calls it. Arms reached from a single leaf are inlined.
- Output is deterministic: fresh names, constructor order and literal order are all fixed.
- Slice 7c guarantees exhaustiveness and declared constructors before elaboration. A violated invariant yields `ElabError::Unsupported` rather than a panic.

## Unsupported constructs

- **`load`** expressions are rejected as unsupported: `load` bodies are not
  type-checked by the surface typechecker (they carry an untyped body with a
  fresh, unconstrained type variable), so there is nothing well-typed for
  elaboration to translate.
- **Constrained bindings** (a `let` whose generalized scheme carries class
  constraints, such as `forall t0. Eq t0 => t0 -> t0 -> Bool`) are rejected
  as unsupported: they need dictionary passing (milestone M2.3), and
  elaboration never drops a constraint silently.
- **Equality on non-base types** is rejected as unsupported, since Core's
  `eq` primitive is restricted to base types only; this also covers equality
  on a type variable in an implicitly generalized program body.
- **`rec` of a non-function** is rejected as unsupported: the surface
  typechecker does permit a `rec` binding whose body does not elaborate to a
  function value, but Core's `LetRec` requires its bound value to already be
  a `Lam`, so such a `rec` cannot be elaborated.
- **A data type applied to too many type arguments** is rejected as
  unsupported. Under-application, such as `SumType(name, [])` from a type
  alias like `type MyList = List` or a higher-kinded argument, is accepted
  and left to Core Lint's kind checker.
- **A residual applied type variable** (an `f a` whose head was never
  generalized or instantiated) is rejected as unsupported. A bare residual
  variable used at a non-`*` kind defaults to `Unit` and Core Lint rejects
  it with `KindMismatch`.
