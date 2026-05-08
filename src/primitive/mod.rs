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

pub use self::{
    aliasable::{Aliasable, DanglingDeref},
    bowl::{Anchor, Bowl, Slot},
    cell::Cell,
    low_level::*,
};
use self::{bounded_view::BoundedView, deref_move::DerefMove};

#[cfg(any(not(feature = "stable_deref"), doc))]
pub use self::stable_deref::{CloneStableDeref, StableDeref};
#[cfg(all(feature = "stable_deref", not(doc)))]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};

pub trait View<'x> {
    type Output;
}

mod bounded_view {
    pub trait BoundedView<'x, 'ub, X = &'x &'ub ()>:
        super::View<'x, Output = Self::Target>
    {
        type Target;
    }
}
impl<'x, T: ?Sized> BoundedView<'x, '_> for T
where
    T: View<'x>,
{
    type Target = Self::Output;
}

mod deref_move {
    pub trait DerefMove: ::core::ops::DerefMut {
        fn deref_move(self) -> Self::Target;
    }
}
