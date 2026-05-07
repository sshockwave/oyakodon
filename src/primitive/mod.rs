//! Primitive types and traits.
//! This is where all `unsafe` code lives,
//! and the rest of the codebase is built on top of it.
//!
//! # A Journey to Safe Self-Referential Types
//! TODO

mod aliasable;
mod bowl;
mod cell;
mod low_level;
mod stable_deref;

use self::seal::BoundedView;
pub use self::{
    aliasable::{Aliasable, DanglingDeref},
    bowl::{Anchor, Bowl, ProtectedForAll, Stamp},
    cell::Cell,
    low_level::*,
};

#[cfg(any(not(feature = "stable_deref"), doc))]
pub use self::stable_deref::{CloneStableDeref, StableDeref};
#[cfg(all(feature = "stable_deref", not(doc)))]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};

pub trait View<'x> {
    type Output;
}

mod seal {
    pub trait BoundedView<'x, 'ub, X = &'x &'ub ()>:
        super::View<'x, Output = Self::Target>
    {
        type Target;
    }
}
impl<'x, 'ub, T: ?Sized> BoundedView<'x, 'ub> for T
where
    T: View<'x>,
{
    type Target = Self::Output;
}
