use ::core::ops::Deref;

/// Pointer that behaves correctly without knowing whether it is being aliased or not.
///
/// Safe Rust code tracks borrows
/// and knows when a unique pointer is shared by other references.
/// This information is lost in unsafe code.
/// By implementing this trait,
/// the pointer should be always safe to borrow from,
/// even if the Rust compiler does not know that
/// the pointer is already being aliased elsewhere.
///
/// # Safety
/// The implementor must guarantee that the pointer to be `Deref`ed
/// is not attributed with `Unique` in Miri or `noalias` in LLVM IR.
pub unsafe trait Aliasable: ::core::ops::Deref {}

unsafe impl<T: ?Sized> Aliasable for &T {}
unsafe impl<T: super::StableDeref> Aliasable for crate::AliasableDeref<T> {}

unsafe impl<T: Deref + ?Sized> Aliasable for crate::MaybeUnique<T, true> {}
unsafe impl<T: Aliasable + ?Sized> Aliasable for crate::MaybeUnique<T, false> {}

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
