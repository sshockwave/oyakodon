#![allow(unsafe_code)]

mod aliasable;
mod bowl;
mod exists;
mod taker;

pub use self::{aliasable::*, bowl::*, exists::*, taker::*};

#[cfg(not(feature = "stable_deref"))]
mod stable_deref;
#[cfg(not(feature = "stable_deref"))]
pub use self::stable_deref::*;
#[cfg(feature = "stable_deref")]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
