//! Primitive types and traits.
//! This is where all `unsafe` code lives,
//! and the rest of the codebase is built on top of it.
//!
//! # A Journey to Safe Self-Referential Types
//! TODO

mod aliasable;
mod bowl;
pub mod stable_deref;

pub use aliasable::{Aliasable, DanglingDeref};
pub use bowl::{Bowl, Derived, Handle, Session, Slot, Stamp, View};

#[cfg(all(feature = "stable_deref", not(doc)))]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
#[cfg(any(not(feature = "stable_deref"), doc))]
pub use stable_deref::{CloneStableDeref, StableDeref};
