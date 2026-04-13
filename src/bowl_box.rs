use super::{BowlMut, Derive, bowl_mut::Map};
use ::{alloc::boxed::Box, core::mem::transmute};

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

    pub fn map_into<'b, G, H>(self, f: H) -> BowlBox<'b, T, G>
    where
        for<'c> H: Derive<<F as Derive<&'c mut T>>::Output>,
        for<'c> G:
            Derive<&'c mut T, Output = <H as Derive<<F as Derive<&'c mut T>>::Output>>::Output>,
    {
        BowlBox(self.0.map_into(f))
    }

    pub fn map<G>(self, f: G) -> BowlBox<'a, T, Map<T, F, G>>
    where
        G: for<'b> Derive<<F as Derive<&'b mut T>>::Output>,
    {
        self.map_into(f)
    }

    pub fn cast_ref<'b, G>(&self) -> &BowlBox<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T, Output = <F as Derive<&'c mut T>>::Output>,
    {
        // SAFETY: Same as BowlMut::cast_ref()
        unsafe { transmute(self) }
    }
    pub fn cast_mut<'b, G>(&mut self) -> &mut BowlBox<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T, Output = <F as Derive<&'c mut T>>::Output>,
    {
        // SAFETY: Same as BowlMut::cast_mut()
        unsafe { transmute(self) }
    }
    pub fn cast<'b, G>(self) -> BowlBox<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T, Output = <F as Derive<&'c mut T>>::Output>,
    {
        self.0.cast().into()
    }

    pub fn get(&self) -> &<F as Derive<&'_ mut T>>::Output {
        self.0.get()
    }
    pub fn get_mut(&mut self) -> &mut <F as Derive<&'_ mut T>>::Output {
        self.0.get_mut()
    }
}

impl<'a, T, F> Clone for BowlBox<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
    BowlMut<'a, Box<T>, F>: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, T, F> From<BowlMut<'a, Box<T>, F>> for BowlBox<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    fn from(value: BowlMut<'a, Box<T>, F>) -> Self {
        Self(value)
    }
}

impl<'a, T, F> From<BowlBox<'a, T, F>> for BowlMut<'a, Box<T>, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    fn from(value: BowlBox<'a, T, F>) -> Self {
        value.0
    }
}

#[cfg(feature = "gat")]
impl<'a, T, F> super::Bowl for BowlBox<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    type Value<'b>
        = <F as Derive<&'b mut T>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        self.0.get()
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        self.0.get_mut()
    }
}
