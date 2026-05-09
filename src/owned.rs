use crate::DerefMove;
use ::core::ops::{Deref, DerefMut};

pub struct Owned<T: ?Sized>(T);

impl<T> Owned<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T: ?Sized> Deref for Owned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> DerefMove for Owned<T> {
    fn deref_move(self) -> Self::Target {
        self.0
    }
}
