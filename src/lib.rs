//! Oyakodon is a library for almost[^almost] zero-cost self-referential structs in Rust.
//! It aims to provide a primitive, flexible, and safe API for this purpose,
//! without any assumptions about the container type or the view type.
//! This is a technical documentation for the library.
//! Users can simply refer to [crates.io](https://crates.io/crates/oyakodon) for a quick start guide.
//!
//! [^almost]: "Almost" because the API is zero-cost,
//! but the some niche compiler optimizations are turned off for safety.
//!
//! # What are self-referential structs?
//! There are many cases where we want to bundle a reference together with the owner of the data it points to.
//! For example, a zero-copy parser might store some [`Cow`] inside the parsed result
//! instead of creating new owned strings.
//! ```rust,ignore
//! fn read_and_parse(file: &str) -> (String, Vec<&str>) {
//!     let s = std::fs::read_to_string(file).unwrap();
//!     let parsed = s.split_whitespace().collect();
//!     (s, parsed)
//! }
//! ```
//! String `s` is moved out of the function, too,
//! so the reference in `parsed` is still valid,
//! but Rust's borrow checker cannot verify that
//! and will report a [E0515] error.
//! This is because each reference must have a determined lifetime,
//! and the lifetime it assigns to `parsed` is only valid before `s` is moved.
//!
//! [`Cow`]: std::borrow::Cow
//! [E0515]: https://doc.rust-lang.org/error_codes/E0515.html
//!
//! In general, we want to create a struct
//! that allows one or more fields to borrow from other fields in the same struct:
//! ```rust,ignore
//! struct SelfRef {
//!    owner: String,
//!    view: Vec<&'owner str>,
//! }
//! ```
//! The `'owner` here is not a real lifetime that can be used in Rust,
//! but it represents the fact that `view` borrows from `owner`.
//! [`oyakodon`][crate], like many other crates,
//! provides a way to construct such self-referential containers without using unsafe code directly.
//! What makes [`oyakodon`][crate] different is that
//! it does not require the user to use macros or implement unsafe traits.
//! It also allows arbitrary view types and
//! is more powerful[^powerful] than [covariance]-based solutions without sacrificing safety.
//!
//! [variance]: https://doc.rust-lang.org/nomicon/subtyping.html#variance
//! [^powerful]: See `covariant.rs` in the `examples/` for a constructive proof.
//!
//! # Safety Overivew
//! The idea of typing is simple.
//! We don't know when will the owner be dropped,
//! so we require users to proof that their code works for every possible `'owner`
//! using higher-ranked trait bounds ([HRTB]s).
//! This is an invariant that we maintain throughout the codebase.
//!
//! [HRTB]: https://doc.rust-lang.org/nomicon/hrtb.html
//!
//! `unsafe` is avoided wherever possible to minimize the review surface.
//! We employ the standard [Miri] tool to run tests for better memory checks.
//! The issue related to LLVM `noalias` found in other solutions are mitigated using [`MaybeDangling`].
//! [`BowlMut`] does not actually need that because we do not allow access to the pointer that marked `noalias`,
//! but we still need that to eliminate Miri `Unique` tagging errors.
//! We also use [`MaybeDangling`] to remove the `dereferenceable` attribute for views.
//!
//! [Miri]: https://github.com/rust-lang/miri/
//! [`MaybeDangling`]: maybe_dangling::MaybeDangling
//!
//! # About AI
//! The tests are vibed while not the rest.
//! AI-generated code are explicitly marked with `Co-Authored-By` in commit messages.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::type_complexity)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod bowl_box;
mod bowl_mut;
mod bowl_ref;
pub mod primitive {
    mod aliasable;
    mod bowl;
    pub mod stable_deref;

    pub use aliasable::{Aliasable, DanglingDeref};
    pub use bowl::{Bowl, Derived, Handle, Session, Slot, Stamp, View};

    #[cfg(all(feature = "stable_deref", not(doc)))]
    pub use ::stable_deref_trait::{CloneStableDeref, StableDeref};
    #[cfg(any(not(feature = "stable_deref"), doc))]
    pub use stable_deref::{CloneStableDeref, StableDeref};
}

use ::core::{future::Future, marker::PhantomData, option::Option, result::Result};
#[cfg(feature = "alloc")]
pub use bowl_box::BowlBox;
pub use bowl_mut::BowlMut;
pub use bowl_ref::BowlRef;

/// Marker trait for representing the type of view.
/// Its lifetime is dependent on the owner type,
/// so it is actually a higher-kinded type
/// that takes a lifetime parameter.
///
/// The purpose of type parameter `T` instead of a simple lifetime `'a` is twofold:
/// 1. It allows us to define the `View` trait for all functions
///    without triggering [E0207](https://doc.rust-lang.org/error_codes/E0207.html).
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
///
/// The `X` type parameter is a hack for adding bounds in HRTB,
/// e.g. in [`BowlRef::spawn`].
pub trait Derive<T, X = ()>: View<T> {
    fn call(self, input: T) -> Self::Output;
}

impl<T, F, R> View<T> for F
where
    F: FnOnce(T) -> R + ?Sized,
{
    type Output = R;
}

impl<T, F, R, X> Derive<T, X> for F
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
