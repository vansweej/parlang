# ADR 0014: Dictionary Passing

## Status

Accepted

Builds on `docs/adr/0013-type-classes.md` (classes and instances),
`docs/adr/0010-kernel-eligibility-predicate.md` (the specialised-Core
contract) and `docs/adr/0009-surface-to-core-elaboration.md`.
Implements slices M2.3a, M2.3b, M2.3c1 and M2.3c2.

## Context

ADR 0013 made classes and instances type-check but rejected every
class program at run time. Milestone 2 needs them to run on the
surface evaluator, both REPLs, Core and the driver, with one shared
translation so that the front ends cannot drift apart.

## Decision

- `insert_dictionaries` (`crates/parlang/src/dictionary/`) rewrites
  the typed AST into dictionary-explicit form. It reuses the `Fun`,
  `App`, `Record` and `FieldAccess` nodes and adds no new AST node.
- A dictionary parameter is named `$d<n>`. Its type is the nominal
  type `$Dict_<Class>` applied to the class argument.
- A dictionary value is a record. Its `$super_<S>` superclass
  dictionary fields come first (ADR 0016), followed by the method
  closures in class declaration order.
- Each instance becomes a top-level `let $dict_<Class>_<Tycon>`. When
  the instance has a context, it is a function of its context
  dictionaries.
- The built-in `Eq` dictionaries are `$dict_Eq_Int`, `$dict_Eq_Bool`,
  `$dict_Eq_Char` and `$dict_Eq_Float`. `==` and `!=` at those types
  stay primitive. At other types they call the `eq` field of the `Eq`
  dictionary, and `!=` negates the result.
- A constrained `let` takes its dictionary parameters outside any
  `rec`. A program body with residual constraints is abstracted over
  its dictionaries.
- `erase_program` erases the result to the untyped AST for the
  surface tree-walker. It drops class declarations and binds each
  instance dictionary recursively:

```text
rec $dict_X -> fun $d0 -> ...          (with a context)
(rec $dict_X -> fun $unit -> record) () (without a context)
```

- `pipeline::surface_program` runs the erased translation when
  insertion changed something or a class is declared. Otherwise it
  runs the original program. It also keeps the original program when
  the translation contains `load`, so `==`-only programs that use
  `load` keep the direct path.
- On Core (`crates/parlang/src/elaborate/dictionary.rs`):
  - `$Dict_C p` is a data type with one constructor. Its fields are
    the superclass dictionaries `$Dict_S p` first, then the method
    types in class order.
  - Method type variables other than the class parameter are
    `Forall`-bound, so these are rank-2 fields.
  - `$Dict_Eq` is registered lazily. Built-in dictionaries are bound
    at the outermost level, and only when referenced, so class-free
    programs elaborate exactly as before.
  - An instance dictionary is a `TyLam` over the instance variables
    around a monomorphic `letrec`, or a unit thunk when the instance
    has no context.
  - Method selection is a `Case` on the dictionary followed by one
    `TyApp` per extra method variable. The type arguments are found
    by renaming the class signature apart, then matching it one way
    against the type at the use site.
- `typed_ast::is_class_program`, `contains_load` and
  `class_load_conflict` replace `requires_dictionary_passing` in file
  mode, both REPLs, `pipeline::elaborate_only` and the driver.

## Consequences

- Class programs run on every front end. `examples/classes.par`
  (value `10111`) runs on the surface evaluator and with `--core`, and
  it is part of the surface-vs-Core oracle.
- Classes and `load` are rejected uniformly with `type classes cannot
  be combined with load yet: ...`. The surface REPL also rejects a
  `load` line while constrained bindings exist.
- `EvalError::ClassesNotExecutable` stays for class or instance
  declarations inside untyped loaded files.
- `kernel-check` rejects constrained functions under R5. Their
  dictionaries are non-kernel data until a Core-to-Core specialiser
  exists (the ADR 0010 contract).
- The same milestone fixed a typechecker soundness bug:
  `infer_constructor` now threads the substitution, so
  `Cons 1 (Cons true Nil)` is a type error.
- Known limitations:
  - Class programs cannot be combined with `load` (`==`-only
    programs may use `load`).
  - In the REPL, a `load` line entered after a class line runs
    untyped. Method calls inside the untyped load body are unbound.
  - An instance dictionary cannot reference an instance declared
    after it.
  - A context-free instance whose method field eagerly evaluates a
    self-reference diverges on both evaluators.
  - Polymorphic recursion through an instance dictionary is
    unsupported on Core.
  - A body whose constructor-class variable stays residual (for example
    `return 1`) runs on the surface evaluator (as a function of its
    dictionary) but Core elaboration rejects it (residual higher-kinded
    type variable).
  - A body value with residual constraints prints as
    `<function $d0>`.
- Roadmap: M2.4a added constructor classes (`Functor` and `Monad` over
  `List` and `Option`); M2.4b added do-notation (ADR 0015); M2.5 added
  superclasses (ADR 0016). A built-in `Show` is deferred. A Core
  specialiser will make constrained functions kernel-eligible.

## References

- `crates/parlang/src/dictionary.rs` (module doc)
- `crates/parlang/src/dictionary/program.rs`
- `crates/parlang/src/dictionary/insert.rs`
- `crates/parlang/src/dictionary/evidence.rs`
- `crates/parlang/src/dictionary/erase.rs`
- `crates/parlang/src/elaborate/dictionary.rs`
- `crates/parlang/src/pipeline.rs`
- `crates/parlang/src/typed_ast.rs`
- `docs/adr/0013-type-classes.md`
- `docs/MODULE_ELABORATE.md` (Dictionaries)
