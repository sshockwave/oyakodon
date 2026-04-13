use super::{BowlMut, Derive};
use ::alloc::boxed::Box;

#[repr(transparent)]
pub struct BowlBox<'a, T, F>(BowlMut<'a, Box<T>, F>)
where
    F: for<'b> Derive<&'b mut T>;

impl<'a, T, F> BowlBox<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    pub fn new(base: T, derive: F) -> Self {
        Self(BowlMut::new(Box::new(base), derive))
    }

    pub fn new_into(
        base: T,
        derive: impl for<'b> Derive<&'b mut T, Output = <F as Derive<&'b mut T>>::Output>,
    ) -> Self {
        Self(BowlMut::new_into(Box::new(base), derive))
    }

    pub fn into_inner(self) -> T {
        *self.0.into_inner()
    }

    pub fn from_fn<'b>(
        base: T,
        derive: &'b dyn for<'c> Fn(&'c mut T) -> <F as Derive<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self(BowlMut::new_into(Box::new(base), derive))
    }

    pub fn from_fn_mut<'b>(
        base: T,
        derive: &'b mut dyn for<'c> FnMut(&'c mut T) -> <F as Derive<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self(BowlMut::new_into(Box::new(base), derive))
    }

    pub fn from_fn_once(
        base: T,
        derive: Box<dyn for<'c> FnOnce(&'c mut T) -> <F as Derive<&'c mut T>>::Output>,
    ) -> Self {
        Self(BowlMut::new_into(Box::new(base), derive))
    }
}
