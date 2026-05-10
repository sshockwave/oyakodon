//! This is where all `unsafe` code lives,
//! and the rest of the codebase is built on top of it.
#![allow(unsafe_code)]

mod aliasable;
mod bowl;
mod exists;
pub(crate) mod polyfill;
mod stable_deref;
mod taker;

pub use self::{aliasable::*, bowl::*, exists::*, stable_deref::*, taker::*};

unsafe impl<T: StableDeref> StableDeref for crate::AliasableDeref<T> {}
unsafe impl<T: CloneStableDeref> CloneStableDeref for crate::AliasableDeref<T> {}
