// SAFETY: See https://docs.rs/stable_deref_trait/latest/stable_deref_trait/trait.StableDeref.html
pub unsafe trait StableDeref: ::core::ops::Deref {}
pub unsafe trait CloneStableDeref: StableDeref + Clone {}

unsafe impl<'a, T: ?Sized> StableDeref for &'a T {}
unsafe impl<'a, T: ?Sized> CloneStableDeref for &'a T {}

unsafe impl<'a, T: ?Sized> StableDeref for &'a mut T {}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> StableDeref for ::alloc::boxed::Box<T> {}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> StableDeref for ::alloc::rc::Rc<T> {}
#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> CloneStableDeref for ::alloc::rc::Rc<T> {}

#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
unsafe impl<T: ?Sized> StableDeref for ::alloc::sync::Arc<T> {}
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
unsafe impl<T: ?Sized> CloneStableDeref for ::alloc::sync::Arc<T> {}
