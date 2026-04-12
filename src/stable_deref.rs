use ::{alloc::boxed::Box, core::ops::Deref};

// SAFETY: See https://docs.rs/stable_deref_trait/latest/stable_deref_trait/trait.StableDeref.html
pub unsafe trait StableDeref: Deref {}
unsafe impl<'a, T: ?Sized> StableDeref for &'a T {}
unsafe impl<'a, T: ?Sized> StableDeref for &'a mut T {}
unsafe impl<T: ?Sized> StableDeref for Box<T> {}
