use super::*;
use ::{
    alloc::boxed::Box,
    core::{fmt::Debug, future::Future, mem::transmute, result::Result},
};

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

    pub fn into_parts<S>(self) -> (T, S)
    where
        for<'c> F: View<&'c mut T, Output = S>,
    {
        let (base, view) = self.0.into_parts();
        (*base, view)
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

    pub fn cast_life<'b>(self) -> BowlBox<'b, T, F> {
        BowlBox(self.0.cast_life())
    }

    pub fn cast_view<'b, G: ?Sized>(self) -> BowlBox<'a, T, G>
    where
        for<'c> G: View<&'c mut T, Output = <F as View<&'c mut T>>::Output>,
    {
        BowlBox(self.0.cast_view())
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

    pub async fn into_async(self) -> BowlBox<'a, T, Async<T, F>>
    where
        for<'c> <F as View<&'c mut T>>::Output: Future,
    {
        BowlBox(self.0.into_async().await)
    }

    pub fn into_result(self) -> Result<BowlBox<'a, T, Success<T, F>>, BowlBox<'a, T, Failure<T, F>>>
    where
        for<'b> <F as View<&'b mut T>>::Output: Outcome,
    {
        self.0.into_result().map(BowlBox).map_err(BowlBox)
    }

    pub fn get(&self) -> &<F as View<&'_ mut T>>::Output {
        self.0.get()
    }
    pub fn get_mut(&mut self) -> &mut <F as View<&'_ mut T>>::Output {
        self.0.get_mut()
    }
}

impl<'a, 'b, T, F> AsRef<BowlBox<'b, T, F>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
{
    fn as_ref(&self) -> &BowlBox<'b, T, F> {
        // SAFETY: `#[repr(transparent)]` delegates to the same safety contract as `BowlMut::as_ref()`
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F> AsMut<BowlBox<'b, T, F>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlBox<'b, T, F> {
        // SAFETY: `#[repr(transparent)]` delegates to the same safety contract as `BowlMut::as_mut()`
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F> AsRef<BowlMut<'b, Box<T>, F>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
{
    fn as_ref(&self) -> &BowlMut<'b, Box<T>, F> {
        self.0.as_ref()
    }
}

impl<'a, 'b, T, F> AsMut<BowlMut<'b, Box<T>, F>> for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'c> View<&'c mut T> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlMut<'b, Box<T>, F> {
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

impl<'a, T, F> Debug for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized,
    for<'b> <F as View<&'b mut T>>::Output: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BowlBox")
            .field("derived", self.get())
            .finish_non_exhaustive()
    }
}
