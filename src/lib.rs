#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod bowl_box;
mod bowl_mut;
mod bowl_ref;
#[cfg(not(feature = "stable_deref"))]
mod stable_deref;

use ::core::marker::PhantomData;
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

pub struct Map<T: ?Sized, F: ?Sized, G: ?Sized>(PhantomData<T>, PhantomData<F>, PhantomData<G>);

impl<'a, T: ?Sized, F, G> View<&'a T> for Map<T, F, G>
where
    F: for<'b> View<&'b T> + ?Sized,
    G: for<'b> View<<F as View<&'b T>>::Output> + ?Sized,
{
    type Output = <G as View<<F as View<&'a T>>::Output>>::Output;
}

impl<'a, T: ?Sized, F, G> View<&'a mut T> for Map<T, F, G>
where
    F: for<'b> View<&'b mut T> + ?Sized,
    G: for<'b> View<<F as View<&'b mut T>>::Output> + ?Sized,
{
    type Output = <G as View<<F as View<&'a mut T>>::Output>>::Output;
}
