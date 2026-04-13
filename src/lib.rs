#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

pub mod bowl_mut;
pub mod bowl_ref;
#[cfg(not(feature = "stable_deref"))]
mod stable_deref;

#[cfg(feature = "stable_deref")]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
pub use bowl_mut::BowlMut;
pub use bowl_ref::BowlRef;
#[cfg(not(feature = "stable_deref"))]
pub use stable_deref::{CloneStableDeref, StableDeref};

#[cfg(feature = "gat")]
pub trait Bowl {
    type Value<'a>
    where
        Self: 'a;
    fn get(&self) -> &Self::Value<'_>;
    fn get_mut(&mut self) -> &mut Self::Value<'_>;
}

pub trait Derive<T> {
    type Output;
    #[allow(unused_variables)]
    fn call(self, input: T) -> Self::Output
    where
        Self: Sized,
    {
        unreachable!(
            "Using the default implementation of Derive::call() indicates that this type is used for type inference only and should never be called."
        )
    }
}

impl<T, F, R> Derive<T> for F
where
    F: FnOnce(T) -> R,
{
    type Output = R;
    fn call(self, input: T) -> Self::Output {
        self(input)
    }
}
