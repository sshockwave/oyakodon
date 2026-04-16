/// # SAFETY
/// In addition to the requirements in [`stable_deref_trait::StableDeref`],
/// implementors must also guarantee that
/// the `noalias` attribute can be erased by [`MaybeDangling`]
/// if it's used by the pointer.
///
/// [`MaybeDangling`]: std::mem::MaybeDangling
pub unsafe trait StableDeref: ::core::ops::Deref {}
/// # SAFETY
/// See [`stable_deref_trait::CloneStableDeref`].
pub unsafe trait CloneStableDeref: StableDeref + Clone {}

unsafe impl<T: ?Sized> StableDeref for &T {}
unsafe impl<T: ?Sized> CloneStableDeref for &T {}

unsafe impl<T: ?Sized> StableDeref for &mut T {}

unsafe impl<'a, T: ?Sized> StableDeref for ::core::cell::Ref<'a, T> {}
unsafe impl<'a, T: ?Sized> StableDeref for ::core::cell::RefMut<'a, T> {}

#[cfg(feature = "alloc")]
mod has_alloc {
    use super::*;
    use ::alloc::*;

    unsafe impl<T: ?Sized> StableDeref for boxed::Box<T> {}
    unsafe impl<T> StableDeref for vec::Vec<T> {}
    unsafe impl StableDeref for string::String {}
    unsafe impl StableDeref for ffi::CString {}

    unsafe impl<'a, B: 'a + borrow::ToOwned + ?Sized> StableDeref for borrow::Cow<'a, B> where
        B::Owned: StableDeref
    {
    }

    unsafe impl<T: ?Sized> StableDeref for rc::Rc<T> {}
    unsafe impl<T: ?Sized> CloneStableDeref for rc::Rc<T> {}

    #[cfg(target_has_atomic = "ptr")]
    unsafe impl<T: ?Sized> StableDeref for sync::Arc<T> {}
    #[cfg(target_has_atomic = "ptr")]
    unsafe impl<T: ?Sized> CloneStableDeref for sync::Arc<T> {}
}

#[cfg(feature = "std")]
mod has_std {
    use super::*;
    use ::std::*;

    unsafe impl StableDeref for ffi::OsString {}
    unsafe impl StableDeref for path::PathBuf {}

    unsafe impl<'a, T: ?Sized> StableDeref for sync::MutexGuard<'a, T> {}
    unsafe impl<'a, T: ?Sized> StableDeref for sync::RwLockReadGuard<'a, T> {}
    unsafe impl<'a, T: ?Sized> StableDeref for sync::RwLockWriteGuard<'a, T> {}
}
