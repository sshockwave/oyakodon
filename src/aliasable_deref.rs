use crate::polyfill::MaybeDangling;
use ::core::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct AliasableDeref<T>(MaybeDangling<T>);

impl<T> AliasableDeref<T> {
    pub fn new(inner: T) -> Self {
        Self(MaybeDangling::new(inner))
    }

    pub fn get(&self) -> &T {
        self.0.as_ref()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.0.as_mut()
    }

    pub fn into_inner(self) -> T {
        MaybeDangling::into_inner(self.0)
    }
}

impl<T: Deref> Deref for AliasableDeref<T> {
    type Target = T::Target;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: DerefMut> DerefMut for AliasableDeref<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
