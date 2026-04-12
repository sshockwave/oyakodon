// SAFETY: See https://docs.rs/stable_deref_trait/latest/stable_deref_trait/trait.StableDeref.html
pub unsafe trait StableDeref: ::core::ops::Deref {}
unsafe impl<'a, T: ?Sized> StableDeref for &'a T {}
unsafe impl<'a, T: ?Sized> StableDeref for &'a mut T {}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> StableDeref for ::alloc::boxed::Box<T> {}
