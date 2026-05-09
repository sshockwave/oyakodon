#![allow(unsafe_code)]

mod aliasable;
mod bowl;
mod exists;
pub mod polyfill;
mod taker;

pub use self::{aliasable::*, bowl::*, exists::*, taker::*};

#[cfg(not(feature = "stable_deref"))]
mod stable_deref;
#[cfg(not(feature = "stable_deref"))]
pub use self::stable_deref::*;
#[cfg(feature = "stable_deref")]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};

unsafe impl<T: StableDeref> StableDeref for crate::primitive::AliasableDeref<T> {}
unsafe impl<T: CloneStableDeref> CloneStableDeref for crate::primitive::AliasableDeref<T> {}
