# Module: parlang-core lint (Core Lint)

## Role

Core Lint is a **pure, bidirectional type checker** over the explicitly-typed
Core `Term` IR introduced across slices 6a-6d. It performs **no inference and
no unification**: every `Term` node either carries enough type annotation to
synthesize its type directly (`Lam`, `LetRec`, `Con`'s explicit type
arguments), or its type is wholly determined by the synthesized types of its
children (`App`, `Let`, `If`/arithmetic/`eq` primitives, `TyLam`/`TyApp`,
`Case`). This is possible only because slice 6 made Core explicitly typed
(ADR 0007); Lint would otherwise have to re-derive the information inference
already computed.

See `docs/adr/0007-core-explicit-typing-and-lint.md` for the design rationale
and `docs/adr/0005-core-type-enum.md` for the underlying `Type` representation
Lint type-checks against.

## Public API

### `LintEnv`

The environment Lint checks a term against. It carries:

- A **data/kind environment**: registered `DataType` declarations (name, type
  parameters, constructors with their field types), built via
  `LintEnv::new().with_data(dt)`.
- A **term context**: a stack of `(name, Type)` bindings, extended for
  `Lam`/`Let`/`LetRec`/`Case`-alt binders during checking, and extendable by
  callers via `with_term(name, ty)` to seed free variables (e.g. prelude
  bindings referenced by an open term).
- An **in-scope type-variable stack**: the names bound by enclosing `TyLam`s
  (or, within a data declaration's field types, the data type's own
  parameters).

`LintEnv::new()` is empty; `prelude_env()` returns a `LintEnv` with the single
`List a = Nil | Cons a (List a)` data type registered, used by the standing
guard described below.

### `lint(term: &Term, env: &LintEnv) -> Result<Type, LintError>`

Checks `term` against `env`, returning its synthesized `Type` on success or
the first `LintError` encountered. Internally this clones `env` and runs the
bidirectional `synth` judgement described below.

## Typing rules

Each `Term` node is handled by exactly one rule; there is no fallback or
default case.

- **`Var`**: looked up in the term context (innermost binding wins); unbound
  is `UnboundTermVar`.
- **`Lit`**: synthesizes `Type::Base(lit.base_type())` directly.
- **`Lam(x, ty, body)`**: `ty` must be well-formed (`well_formed`, see below);
  `body` is synthesized with `x: ty` pushed onto the term context; the result
  is `Fun(ty, body_ty)`.
- **`App(f, a)`**: `f` must synthesize a `Fun(dom, cod)`; `a`'s synthesized
  type must be alpha-equivalent to `dom` (`ArgTypeMismatch` otherwise); result
  is `cod`.
- **`Let(x, v, b)`**: **unannotated** — `v`'s type is synthesized and pushed
  as `x`'s binding before synthesizing `b`; the result is `b`'s type. Unlike
  `Lam`, there is no annotation to check against; `Let` is purely an
  (unconstrained) binder introduction, matching the "Let is unannotated"
  design recorded in ADR 0007.
- **`LetRec(x, ann, v, b)`**: `ann` must be well-formed; `v` must be a `Lam`
  (`LetRecValueNotLam` otherwise); `x: ann` is pushed BEFORE synthesizing `v`,
  so the recursive call inside `v`'s body resolves; `v`'s synthesized type
  must be alpha-equivalent to `ann` (`LetRecAnnotationMismatch` otherwise);
  then `b` is synthesized with `x: ann` still in scope.
- **`TyLam(a, t)`**: `a` must not already be an in-scope type variable
  (`ShadowedTypeVar` otherwise); `a` is pushed onto the tyvar stack, `t` is
  synthesized, and the result is `Forall(a, t_ty)`.
- **`TyApp(t, ty)`**: `t` must synthesize a `Forall(a, body)`
  (`NotAForall` otherwise); `ty` must be well-formed; the result is
  `body.subst(a, ty)` (capture-avoiding substitution, ADR 0005's named-binder
  `subst`, not a De Bruijn shift).
- **Primitives via `Con`** (reserved names `if`, `+`, `-`, `*`, `/`, `<`,
  `eq`; a non-reserved `Con` name goes through the data-constructor rule
  instead):
  - `if`: exactly 3 args; the condition must synthesize `Bool`
    (`PrimArgMismatch` otherwise); the two branches' synthesized types must be
    alpha-equivalent (`BranchTypeMismatch` otherwise); result is that type.
  - `+`/`-`/`*`/`/`: exactly 2 args, both must synthesize `Int`; result is
    `Int`.
  - `<`: exactly 2 args, both `Int` or both `Char`; result is `Bool`.
  - `eq`: exactly 2 args; each must synthesize a `Type::Base(_)`
    (`NonBaseEq` otherwise); the two base types must be equal
    (`EqTypeMismatch` otherwise); result is `Bool`. A reserved primitive given
    explicit type arguments is rejected with `ReservedPrimWithTypeArgs`.
- **Data `Con(name, ty_args, args)`** (non-reserved `name`): `name` must be a
  registered constructor (`UnknownConstructor` otherwise), owned by a
  registered data type (`UnknownType` otherwise); `ty_args` must match the
  data type's parameter count (`TypeArgCountMismatch` otherwise) and each must
  be well-formed; `args` must match the constructor's field count
  (`FieldCountMismatch` otherwise), and each argument's synthesized type must
  be alpha-equivalent to its field type with the data type's parameters
  substituted by `ty_args` (`ArgTypeMismatch` otherwise); the result is the
  data type's name applied (via `Type::App`) to `ty_args` in order.
- **`Case(scrutinee, alts)`** (ADR 0003's exhaustive tag-switch): `alts` must
  be non-empty (`EmptyCase` otherwise); the scrutinee's synthesized type must
  resolve (via its application spine) to a registered data type
  (`NotADataType` otherwise); every alt's constructor must belong to that data
  type (`ForeignAlt` otherwise), with no duplicate constructor names
  (`DuplicateAlt`), and EVERY constructor of the data type must have an
  alternative (`NonExhaustiveCase` listing the missing names) — consistent
  with ADR 0003's "no default alternative" design. Each alt's binders are
  **unannotated**: their types are derived positionally from the data type's
  field types, substituting the data type's parameters by the scrutinee's
  instantiated type arguments, and the binder count must match the field count
  (`BinderCountMismatch` otherwise). All alt bodies' synthesized types must be
  alpha-equivalent to each other (`BranchTypeMismatch` otherwise); that common
  type is the result.

### Well-formedness of types (`well_formed`)

A type is well-formed under an environment if every free type variable it
mentions is in scope (bound by an enclosing `TyLam`, or, when checking a data
declaration's own field types, one of that data type's own parameters;
otherwise `UnboundTypeVar`), and every type-constructor application has
exactly the arity the data environment records for it (`UnknownType` for an
unregistered name, `ArityMismatch` for a wrong argument count). A `Forall`
binder that shadows an already in-scope type variable is rejected
(`ShadowedTypeVar`), mirroring the same rule for `TyLam` terms.

## `LintError`

The full set of ways Lint can reject a term: `UnboundTermVar`,
`UnboundTypeVar`, `ShadowedTypeVar`, `UnknownType`, `UnknownConstructor`,
`NotAFunction`, `ArgTypeMismatch`, `TypeArgCountMismatch`,
`FieldCountMismatch`, `ArityMismatch`, `ReservedPrimWithTypeArgs`,
`PrimArgCount`, `PrimArgMismatch`, `NotAForall`, `NotADataType`,
`NonExhaustiveCase`, `DuplicateAlt`, `ForeignAlt`, `BinderCountMismatch`,
`EmptyCase`, `BranchTypeMismatch`, `LetRecValueNotLam`,
`LetRecAnnotationMismatch`, `NonBaseEq`, `EqTypeMismatch`. Each variant's
`Display` implementation renders a human-readable message including the
relevant type(s)/name(s).

## `prelude_env` and the linted `not`

`prelude_env()` returns a `LintEnv` with `List a = Nil | Cons a (List a)`
registered as a `DataType`, used by tests that lint `Case` expressions over
lists. The prelude function `not : Bool -> Bool` (built in `prelude.rs` as
`\b: Bool. if b then false else true`) lint-checks cleanly against the empty
`LintEnv` (it needs no data environment), confirming that explicitly-typed
prelude terms are Lint-clean.

## References

- `docs/adr/0007-core-explicit-typing-and-lint.md`
- `docs/adr/0005-core-type-enum.md`
- `docs/adr/0003-core-case-eliminator.md`
- `docs/MODULE_CORE_EVAL.md`
- `docs/MODULE_CORE_PRELUDE.md`
