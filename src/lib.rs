#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod bowl_box;
mod bowl_mut;
mod bowl_ref;
#[cfg(any(not(feature = "stable_deref"), doc))]
mod stable_deref;

use ::core::{future::Future, marker::PhantomData, option::Option, result::Result};
#[cfg(all(feature = "stable_deref", not(doc)))]
pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
#[cfg(feature = "alloc")]
pub use bowl_box::BowlBox;
pub use bowl_mut::BowlMut;
pub use bowl_ref::BowlRef;
#[cfg(any(not(feature = "stable_deref"), doc))]
pub use stable_deref::{CloneStableDeref, StableDeref};

/// Unified interface for all bowl types.
/// [`BowlRef`], [`BowlMut`], and [`BowlBox`] all implement this trait.
#[cfg(feature = "gat")]
pub trait Bowl {
    type Value<'a>
    where
        Self: 'a;
    fn get(&self) -> &Self::Value<'_>;
    fn get_mut(&mut self) -> &mut Self::Value<'_>;
}

/// Marker trait for indicating the type of view.
/// Its lifetime is dependent on the owner type,
/// so it is actually a higher-kinded type
/// that takes a lifetime parameter.
///
/// The purpose of type parameter `T` instead of a simple lifetime `'a` is twofold:
/// 1. It allows us to define the `View` trait for all functions
/// without triggering [E0207](https://doc.rust-lang.org/error_codes/E0207.html).
/// 2. Writing `&'a T` constrains the lifetime variable in HRTB to at most as long as `T`.
pub trait View<T: ?Sized> {
    type Output;
}

/// The consuming counterpart of [`View`].
///
/// Named functions automatically satisfy this via the blanket impl for [`FnOnce`].
/// For closures, prefer the `from_fn` constructors with a `&dyn Fn` argument,
/// since closures cannot express a generic return lifetime on stable Rust.
/// Alternatively, define a custom type and implement [`View`] and [`Derive`] manually
/// for zero-cost dispatch without the `dyn` indirection.
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

/// Phantom view type representing the composition of `F` followed by `G`.
///
/// Used as the view type parameter after calling [`BowlRef::map`], [`BowlMut::map`],
/// or [`BowlBox::map`]. `F` is applied first to produce an intermediate value,
/// which `G` then transforms into the final output.
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

/// Phantom view type that awaits the future produced by `F`.
///
/// Used as the view type parameter after calling [`BowlRef::into_async`],
/// [`BowlMut::into_async`], or [`BowlBox::into_async`].
/// The owner remains alive for the entire duration of the await.
pub struct Async<T: ?Sized, F: ?Sized>(PhantomData<T>, PhantomData<F>);

impl<'a, T: ?Sized, F> View<&'a T> for Async<T, F>
where
    F: for<'b> View<&'b T> + ?Sized,
    <F as View<&'a T>>::Output: Future,
{
    type Output = <<F as View<&'a T>>::Output as Future>::Output;
}

impl<'a, T: ?Sized, F> View<&'a mut T> for Async<T, F>
where
    F: for<'b> View<&'b mut T> + ?Sized,
    <F as View<&'a mut T>>::Output: Future,
{
    type Output = <<F as View<&'a mut T>>::Output as Future>::Output;
}

/// Abstracts over [`Result`] and [`Option`] for use with the `into_result` methods.
///
/// For [`Option<T>`], the error type is `()`, so [`None`] becomes [`Err(())`][Err].
pub trait Outcome {
    type Ok;
    type Err;
    fn get(self) -> Result<Self::Ok, Self::Err>;
}

impl<T> Outcome for Option<T> {
    type Ok = T;
    type Err = ();
    fn get(self) -> Result<Self::Ok, Self::Err> {
        self.ok_or(())
    }
}

impl<T, E> Outcome for Result<T, E> {
    type Ok = T;
    type Err = E;
    fn get(self) -> Result<T, E> {
        self
    }
}

/// Phantom view type for the `Ok` branch produced by `into_result`.
///
/// The contained view is [`Outcome::Ok`] of the original view type.
pub struct Success<T: ?Sized, F: ?Sized>(PhantomData<T>, PhantomData<F>);

impl<'a, T: ?Sized, F> View<&'a T> for Success<T, F>
where
    F: for<'b> View<&'b T> + ?Sized,
    <F as View<&'a T>>::Output: Outcome,
{
    type Output = <<F as View<&'a T>>::Output as Outcome>::Ok;
}

impl<'a, T: ?Sized, F> View<&'a mut T> for Success<T, F>
where
    F: for<'b> View<&'b mut T> + ?Sized,
    <F as View<&'a mut T>>::Output: Outcome,
{
    type Output = <<F as View<&'a mut T>>::Output as Outcome>::Ok;
}

/// Phantom view type for the `Err` branch produced by `into_result`.
pub struct Failure<T: ?Sized, F: ?Sized>(PhantomData<T>, PhantomData<F>);

impl<'a, T: ?Sized, F> View<&'a T> for Failure<T, F>
where
    F: for<'b> View<&'b T> + ?Sized,
    <F as View<&'a T>>::Output: Outcome,
{
    type Output = <<F as View<&'a T>>::Output as Outcome>::Err;
}

impl<'a, T: ?Sized, F> View<&'a mut T> for Failure<T, F>
where
    F: for<'b> View<&'b mut T> + ?Sized,
    <F as View<&'a mut T>>::Output: Outcome,
{
    type Output = <<F as View<&'a mut T>>::Output as Outcome>::Err;
}
