use super::{BowlMut, Derive, Map, View};
use ::{alloc::boxed::Box, core::mem::transmute};

#[repr(transparent)]
pub struct BowlBox<'a, T, F>(BowlMut<'a, Box<T>, F>)
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized;

impl<'a, T, F> BowlBox<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    pub fn new(base: T, derive: F) -> Self {
        Self(BowlMut::new(Box::new(base), derive))
    }
}

impl<'a, T, F> BowlBox<'a, T, F>
where
    F: ?Sized + for<'b> View<&'b mut T>,
{
    pub fn from_derive(
        base: T,
        derive: impl for<'b> Derive<&'b mut T, Output = <F as View<&'b mut T>>::Output>,
    ) -> Self {
        Self(BowlMut::from_derive(Box::new(base), derive))
    }

    pub fn into_inner(self) -> T {
        *self.0.into_inner()
    }

    pub fn from_fn<'b>(
        base: T,
        derive: &'b dyn for<'c> Fn(&'c mut T) -> <F as View<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self(BowlMut::from_derive(Box::new(base), derive))
    }

    pub fn from_fn_mut<'b>(
        base: T,
        derive: &'b mut dyn for<'c> FnMut(&'c mut T) -> <F as View<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self(BowlMut::from_derive(Box::new(base), derive))
    }

    pub fn from_fn_once(
        base: T,
        derive: Box<dyn for<'c> FnOnce(&'c mut T) -> <F as View<&'c mut T>>::Output>,
    ) -> Self {
        Self(BowlMut::from_derive(Box::new(base), derive))
    }
}

impl<'a, T, F> BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized,
{
    pub fn map_into<'b, G: ?Sized, H>(self, f: H) -> BowlBox<'b, T, G>
    where
        for<'c> H: Derive<<F as View<&'c mut T>>::Output>,
        for<'c> G: View<&'c mut T, Output = <H as View<<F as View<&'c mut T>>::Output>>::Output>,
    {
        BowlBox(self.0.map_into(f))
    }

    pub fn map<G>(self, f: G) -> BowlBox<'a, T, Map<T, F, G>>
    where
        G: for<'b> Derive<<F as View<&'b mut T>>::Output>,
    {
        self.map_into(f)
    }

    pub fn cast<'b, G: ?Sized>(self) -> BowlBox<'b, T, G>
    where
        for<'c> G: View<&'c mut T, Output = <F as View<&'c mut T>>::Output>,
    {
        BowlBox(self.0.cast())
    }

    pub fn into_view<S>(self) -> S
    where
        for<'c> F: View<&'c mut T, Output = S>,
    {
        self.0.into_view()
    }

    pub fn get(&self) -> &<F as View<&'_ mut T>>::Output {
        self.0.get()
    }
    pub fn get_mut(&mut self) -> &mut <F as View<&'_ mut T>>::Output {
        self.0.get_mut()
    }
}

impl<'a, 'b, T, F, G> AsRef<BowlBox<'b, T, G>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
    G: for<'c> View<&'c mut T, Output = <F as View<&'c mut T>>::Output> + ?Sized,
{
    fn as_ref(&self) -> &BowlBox<'b, T, G> {
        // SAFETY: `#[repr(transparent)]` delegates to the same safety contract as `BowlMut::as_ref()`
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F, G> AsMut<BowlBox<'b, T, G>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
    G: for<'c> View<&'c mut T, Output = <F as View<&'c mut T>>::Output> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlBox<'b, T, G> {
        // SAFETY: `#[repr(transparent)]` delegates to the same safety contract as `BowlMut::as_mut()`
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F, G> AsRef<BowlMut<'b, Box<T>, G>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
    G: for<'c> View<&'c mut T, Output = <F as View<&'c mut T>>::Output> + ?Sized,
{
    fn as_ref(&self) -> &BowlMut<'b, Box<T>, G> {
        self.0.as_ref()
    }
}

impl<'a, 'b, T, F, G> AsMut<BowlMut<'b, Box<T>, G>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
    G: for<'c> View<&'c mut T, Output = <F as View<&'c mut T>>::Output> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlMut<'b, Box<T>, G> {
        self.0.as_mut()
    }
}

impl<'a, T, F> From<BowlMut<'a, Box<T>, F>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized,
{
    fn from(value: BowlMut<'a, Box<T>, F>) -> Self {
        Self(value)
    }
}

impl<'a, T, F> From<BowlBox<'a, T, F>> for BowlMut<'a, Box<T>, F>
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized,
{
    fn from(value: BowlBox<'a, T, F>) -> Self {
        value.0
    }
}

#[cfg(feature = "gat")]
impl<'a, T, F> super::Bowl for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized,
{
    type Value<'b>
        = <F as View<&'b mut T>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        self.0.get()
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        self.0.get_mut()
    }
}
