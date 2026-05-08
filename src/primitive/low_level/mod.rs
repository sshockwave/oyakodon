mod aliasable;
mod for_all;
mod taker;

pub use self::{aliasable::*, for_all::*, taker::*};
use super::*;

#[cfg(any(not(feature = "stable_deref"), doc))]
mod stable_deref;
#[cfg(any(not(feature = "stable_deref"), doc))]
pub use self::stable_deref::*;
#[cfg(all(feature = "stable_deref", not(doc)))]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
