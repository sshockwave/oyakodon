#![allow(unsafe_code)]

mod aliasable;
mod bowl;
mod exists;
mod taker;

pub use self::{aliasable::*, bowl::*, exists::*, taker::*};

#[cfg(any(not(feature = "stable_deref"), doc))]
mod stable_deref;
#[cfg(any(not(feature = "stable_deref"), doc))]
pub use self::stable_deref::*;
#[cfg(all(feature = "stable_deref", not(doc)))]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
