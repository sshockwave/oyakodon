#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod bowl_box;
pub mod bowl_mut;
pub mod bowl_ref;
#[cfg(not(feature = "stable_deref"))]
mod stable_deref;

#[cfg(feature = "stable_deref")]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
#[cfg(feature = "alloc")]
pub use bowl_box::BowlBox;
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

pub trait View<T> {
    type Output;
}

pub trait Derive<T>: View<T> {
    fn call(self, input: T) -> Self::Output;
}

impl<T, F, R> View<T> for F
where
    F: FnOnce(T) -> R + ?Sized,
{
    type Output = R;
}

impl<T, F, R> Derive<T> for F
where
    F: FnOnce(T) -> R,
{
    fn call(self, input: T) -> Self::Output {
        self(input)
    }
}
