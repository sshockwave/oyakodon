use crate::DerefMove;
use ::core::{
    mem::replace,
    ops::{Deref, DerefMut},
    option::Option,
};

#[derive(Eq, Ord)]
pub struct Taker<'a, T>(&'a mut Option<T>);

impl<'a, T> Taker<'a, T> {
    /// # Safety
    /// The `value` must be `Some`,
    /// and we maintain an invariant that
    /// `value` will always be `Some` during the lifetime of `self`.
    pub unsafe fn new(value: &'a mut Option<T>) -> Self {
        Self(value)
    }
}

impl<'a, T> Deref for Taker<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // It is verified that this will compile to unchecked dereference with `-O`.
        let value = self.0.as_ref();
        // SAFETY: The invariant guarantees that `value` is `Some`.
        unsafe { value.unwrap_unchecked() }
    }
}

impl<'a, T> DerefMut for Taker<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let value = self.0.as_mut();
        // SAFETY: The invariant guarantees that `value` is `Some`.
        unsafe { value.unwrap_unchecked() }
    }
}

impl<'a, T> DerefMove for Taker<'a, T> {
    fn deref_move(self) -> Self::Target {
        let value = replace(self.0, None);
        // SAFETY: Guaranteed by the invariant.
        unsafe { value.unwrap_unchecked() }
    }
}
