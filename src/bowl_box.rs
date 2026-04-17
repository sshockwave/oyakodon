use super::*;
use ::{
    alloc::boxed::Box,
    core::{fmt::Debug, future::Future, mem::transmute, result::Result},
};

/// Stores a value into the heap and a derived mutable reference into it.
///
/// This is a thin wrapper around [`BowlMut<Box<T>, F>`][BowlMut].
/// Conversions between `BowlBox<T, F>` and `BowlMut<Box<T>, F>` are provided via [`From`].
///
/// # Examples
///
/// ```
/// use oyakodon::BowlBox;
///
/// fn parse_words(s: &mut String) -> Vec<&str> {
///     s.split_whitespace().collect()
/// }
///
/// let mut bowl = BowlBox::new("hello world foo".to_owned(), parse_words);
/// bowl.get_mut()[2] = "bar";
/// assert_eq!(*bowl.get(), vec!["hello", "world", "bar"]);
///
/// assert_eq!(bowl.into_owner(), "hello world foo");
/// ```
#[repr(transparent)]
pub struct BowlBox<'a, T, F>(BowlMut<'a, Box<T>, F>)
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized;

impl<'a, T, F> BowlBox<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    pub fn new(owner: T, derive: F) -> Self {
        Self(BowlMut::new(Box::new(owner), derive))
    }
}

impl<'a, T, F> BowlBox<'a, T, F>
where
    F: ?Sized + for<'b> View<&'b mut T>,
{
    /// See [`BowlMut::from_derive`]. Boxes the owner automatically.
    pub fn from_derive(
        owner: T,
        derive: impl for<'b> Derive<&'b mut T, Output = <F as View<&'b mut T>>::Output>,
    ) -> Self {
        Self(BowlMut::from_derive(Box::new(owner), derive))
    }

    /// Drops the view and returns the unboxed owner.
    ///
    /// Unlike [`BowlMut::into_owner`], the [`Box`] is dereferenced so the value is returned by value.
    pub fn into_owner(self) -> T {
        *self.0.into_owner()
    }

    /// See [`BowlMut::from_fn`].
    pub fn from_fn<'b>(
        owner: T,
        derive: &'b dyn for<'c> Fn(&'c mut T) -> <F as View<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self(BowlMut::from_derive(Box::new(owner), derive))
    }

    /// See [`BowlMut::from_fn_mut`].
    pub fn from_fn_mut<'b>(
        owner: T,
        derive: &'b mut dyn for<'c> FnMut(&'c mut T) -> <F as View<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self(BowlMut::from_derive(Box::new(owner), derive))
    }

    /// See [`BowlMut::from_fn_once`].
    pub fn from_fn_once(
        owner: T,
        derive: Box<dyn for<'c> FnOnce(&'c mut T) -> <F as View<&'c mut T>>::Output>,
    ) -> Self {
        Self(BowlMut::from_derive(Box::new(owner), derive))
    }

    /// See [`BowlMut::into_parts`]. The owner is unboxed.
    pub fn into_parts<S>(self) -> (T, S)
    where
        for<'c> F: View<&'c mut T, Output = S>,
    {
        let (owner, view) = self.0.into_parts();
        (*owner, view)
    }
}

impl<'a, T, F> BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized,
{
    /// See [`BowlMut::map`].
    pub fn map<G>(self, f: G) -> BowlBox<'a, T, Map<T, F, G>>
    where
        G: for<'b> Derive<<F as View<&'b mut T>>::Output>,
    {
        BowlBox(self.0.map(f))
    }

    /// See [`BowlMut::cast_life`].
    pub fn cast_life<'b>(self) -> BowlBox<'b, T, F> {
        BowlBox(self.0.cast_life())
    }

    /// See [`BowlMut::cast_view`].
    pub fn cast_view<
        'b,
        G: ?Sized + for<'c> View<&'c mut T, Output = <F as View<&'c mut T>>::Output>,
    >(
        self,
    ) -> BowlBox<'a, T, G> {
        BowlBox(self.0.cast_view())
    }

    /// See [`BowlMut::cast`].
    pub fn cast<
        'b,
        G: ?Sized + for<'c> View<&'c mut T, Output = <F as View<&'c mut T>>::Output>,
    >(
        self,
    ) -> BowlBox<'b, T, G> {
        BowlBox(self.0.cast())
    }

    /// See [`BowlMut::into_view`].
    pub fn into_view<S>(self) -> S
    where
        for<'c> F: View<&'c mut T, Output = S>,
    {
        self.0.into_view()
    }

    /// See [`BowlMut::into_async`].
    pub async fn into_async(self) -> BowlBox<'a, T, Async<T, F>>
    where
        for<'c> <F as View<&'c mut T>>::Output: Future,
    {
        BowlBox(self.0.into_async().await)
    }

    /// See [`BowlMut::into_result`].
    pub fn into_result(self) -> Result<BowlBox<'a, T, Success<T, F>>, BowlBox<'a, T, Failure<T, F>>>
    where
        for<'b> <F as View<&'b mut T>>::Output: Outcome,
    {
        self.0.into_result().map(BowlBox).map_err(BowlBox)
    }

    pub fn spawn<'b, S>(
        &'b self,
        spawn: impl for<'c> Derive<&'b <F as View<&'c mut T>>::Output, Output = S>,
    ) -> S {
        self.0.spawn(spawn)
    }

    pub fn spawn_mut<'b, S>(
        &'b mut self,
        spawn: impl for<'c> Derive<&'b mut <F as View<&'c mut T>>::Output, Output = S>,
    ) -> S {
        self.0.spawn_mut(spawn)
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

impl<'a, T, F> Debug for BowlBox<'a, T, F>
where
    T: ?Sized,
    F: for<'b> View<&'b mut T> + ?Sized,
    for<'b> <F as View<&'b mut T>>::Output: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.spawn(|view: &<F as View<&mut T>>::Output| {
            f.debug_struct("BowlBox")
                .field("view", view)
                .finish_non_exhaustive()
        })
    }
}
