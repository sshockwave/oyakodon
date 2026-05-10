/// # Safety
/// In addition to [`StableDeref`][crate::StableDeref],
/// the implementor must guarantee that the pointer to be `Deref`ed
/// is not attributed with `Unique` in Miri or `noalias` in LLVM IR.
pub unsafe trait Aliasable: ::core::ops::Deref {}

unsafe impl<T: ?Sized> Aliasable for &T {}
unsafe impl<T: super::StableDeref> Aliasable for crate::AliasableDeref<T> {}

#[cfg(feature = "alloc")]
mod has_alloc {
    use super::Aliasable;
    unsafe impl<T: ?Sized> Aliasable for ::alloc::rc::Rc<T> {}
    #[cfg(target_has_atomic = "ptr")]
    unsafe impl<T: ?Sized> Aliasable for ::alloc::sync::Arc<T> {}
    unsafe impl<T: ?Sized> Aliasable for crate::AliasableBox<T> {}
    unsafe impl<T> Aliasable for crate::AliasableVec<T> {}
    unsafe impl Aliasable for crate::AliasableString {}
}
