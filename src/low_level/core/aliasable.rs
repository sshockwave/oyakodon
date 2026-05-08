pub unsafe trait Aliasable: ::core::ops::Deref {}

unsafe impl<T: ?Sized> Aliasable for &T {}
unsafe impl<T: super::StableDeref> Aliasable for crate::primitive::DanglingDeref<T> {}

#[cfg(feature = "alloc")]
mod has_alloc {
    use super::Aliasable;
    unsafe impl<T: ?Sized> Aliasable for ::alloc::rc::Rc<T> {}
    #[cfg(target_has_atomic = "ptr")]
    unsafe impl<T: ?Sized> Aliasable for ::alloc::sync::Arc<T> {}
    unsafe impl<T: ?Sized> Aliasable for ::aliasable::boxed::AliasableBox<T> {}
    unsafe impl<T> Aliasable for ::aliasable::vec::AliasableVec<T> {}
    unsafe impl Aliasable for ::aliasable::string::AliasableString {}
}
