//! Type-class dictionary insertion (M2.3).
//!
//! A pass that runs after type checking over a resolved `TypedProgram`.
//! Each class constraint becomes an explicit dictionary. The output reuses
//! existing `TypedExprKind` nodes (`Fun`, `App`, `Record`, `FieldAccess`).
//!
//! Contract for the evaluators (M2.3b) and Core elaboration (M2.3c):
//!
//! - A dictionary value is a `Record` node whose fields are the class
//!   methods in class declaration order. Its type is nominal:
//!   `$Dict_<Class> arg` (a `Type::SumType`). M2.3c registers a Core data
//!   type `$Dict_<Class> p` with one constructor whose fields are the
//!   methods in that order, with method type variables other than the
//!   class parameter bound by `Forall`. Classes have no superclasses yet;
//!   M2.5 adds superclass fields.
//! - An instance becomes a top-level `let` named `$dict_<Class>_<Tycon>`.
//!   An instance with a context is a function of its context dictionaries.
//!   An instance dictionary may reference itself (a method body using the
//!   same instance); M2.3b and M2.3c must bind it recursively.
//! - A constrained binding takes one dictionary parameter `$d<n>` per
//!   constraint, outside any `rec`, so recursive uses inside stay
//!   monomorphic. A class method occurrence is `FieldAccess(dict, method)`
//!   typed at the occurrence. `==` and `!=` at `Int`, `Bool`, `Char` or
//!   `Float` stay primitive; at any other operand type they call the `eq`
//!   field of the `Eq` dictionary, and `!=` negates it.
//! - A trailing body with residual constraints (`body_constraints`) is
//!   abstracted over their dictionaries in order.
//! - The `$` prefix of `$d`, `$dict_` and `$Dict_` names cannot clash with
//!   user identifiers.
//!
//! Constrained functions stay non-kernel: `kernel_check` rule R5 rejects
//! generalised functions. A later Core to Core specialiser (ADR 0010,
//! eligibility on specialised Core) may make them kernel-eligible.

mod erase;
mod evidence;
mod insert;
mod program;

pub use erase::{erase_program, EraseError};
pub use program::insert_dictionaries;
pub(crate) use program::method_renaming;

pub use evidence::{
    apply_all, dictionary_name, dictionary_type, dictionary_type_name, evidence_expr, solve,
    DictError, Evidence,
};
