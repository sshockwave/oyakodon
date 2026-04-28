//! Primitive types and traits.
//! This is where all `unsafe` code lives,
//! and the rest of the codebase is built on top of it.
//!
//! # A Journey to Safe Self-Referential Types
//! TODO

mod aliasable;
mod bowl;
pub mod stable_deref;

pub use self::{
    aliasable::{Aliasable, DanglingDeref},
    bowl::{Bowl, Derived, Handle, MutView, RefView, Session, Slot, Stamp, View},
};

#[cfg(any(not(feature = "stable_deref"), doc))]
pub use self::stable_deref::{CloneStableDeref, StableDeref};
#[cfg(all(feature = "stable_deref", not(doc)))]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
