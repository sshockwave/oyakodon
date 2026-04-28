use super::StableDeref;
use ::{
    core::ops::{Deref, DerefMut},
    maybe_dangling::MaybeDangling,
};

pub unsafe trait Aliasable: Deref {}

unsafe impl<T: ?Sized> Aliasable for &T {}
unsafe impl<T> Aliasable for DanglingDeref<T> where T: StableDeref {}

#[cfg(feature = "alloc")]
mod has_alloc {
    use super::Aliasable;
    unsafe impl<T: ?Sized> Aliasable for ::alloc::rc::Rc<T> {}
    #[cfg(target_has_atomic = "ptr")]
    unsafe impl<T: ?Sized> Aliasable for ::alloc::sync::Arc<T> {}
    unsafe impl<T: ?Sized> Aliasable for ::aliasable::boxed::AliasableBox<T> {}
    unsafe impl<T> Aliasable for ::aliasable::vec::AliasableVec<T> {}
}

pub struct DanglingDeref<T>(MaybeDangling<T>);

impl<T> DanglingDeref<T> {
    pub fn new(inner: T) -> Self {
        Self(MaybeDangling::new(inner))
    }

    pub fn into_inner(self) -> T {
        MaybeDangling::into_inner(self.0)
    }
}

impl<T: Deref> Deref for DanglingDeref<T> {
    type Target = T::Target;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: DerefMut> DerefMut for DanglingDeref<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}
