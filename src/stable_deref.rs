/// SAFETY: In addition to the requirements in [`stable_deref_trait::StableDeref`],
/// implementors must also guarantee that
/// the `noalias` attribute can be erased by [`MaybeDangling`]
/// if it's used by the pointer.
///
/// [`MaybeDangling`]: std::mem::MaybeDangling
pub unsafe trait StableDeref: ::core::ops::Deref {}
pub unsafe trait CloneStableDeref: StableDeref + Clone {}

unsafe impl<'a, T: ?Sized> StableDeref for &'a T {}
unsafe impl<'a, T: ?Sized> CloneStableDeref for &'a T {}

unsafe impl<'a, T: ?Sized> StableDeref for &'a mut T {}

#[cfg(feature = "alloc")]
mod has_alloc {
    use super::*;
    use ::alloc::*;

    unsafe impl<T: ?Sized> StableDeref for boxed::Box<T> {}

    unsafe impl<T: ?Sized> StableDeref for rc::Rc<T> {}
    unsafe impl<T: ?Sized> CloneStableDeref for rc::Rc<T> {}

    #[cfg(target_has_atomic = "ptr")]
    unsafe impl<T: ?Sized> StableDeref for sync::Arc<T> {}
    #[cfg(target_has_atomic = "ptr")]
    unsafe impl<T: ?Sized> CloneStableDeref for sync::Arc<T> {}
}
