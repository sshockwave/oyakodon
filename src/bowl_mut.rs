/// The safety arguments in this file are mostly the same as `BowlRef`, so they are not repeated here.
// These two implementations are not merged using macros because they break rust-analyzer.
// We also cannot use a common trait
// because an explicit reference `&'a T` in HRTB `for<'a> F: Ref<&'a T>`
// provide an implicit bound of on `'a` to have shorter lifetime than `T`
// while `for<'a> F: Ref<'a, T>` requires `T` to be static to satisfy `'a: 'static`.
// See https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats
// for an in-depth discussion on this issue.
// Internally, we implement `BowlMut` as a wrapper around `BowlRef`
// so the code size increase is not significant anyway.
use super::*;
use ::{
    core::{
        fmt::Debug,
        future::Future,
        mem::transmute,
        ops::{Deref, DerefMut},
        result::Result,
    },
    maybe_dangling::MaybeDangling,
};

/// We create this internal type to re-use some implementations from [`BowlRef`].
#[repr(transparent)]
struct MutToRef<F: ?Sized>(F);
impl<'a, T, F> View<&'a T> for MutToRef<F>
where
    T: ?Sized,
    F: ?Sized + for<'b> View<&'b mut T>,
{
    type Output = <F as View<&'a mut T>>::Output;
}

/// Stores a pointer to an owner and a derived mutable reference into it.
///
/// The difference from [`BowlRef`] is that `derive` receives `&mut T::Target`.
/// Because `&mut` implies exclusive access, the owner is not accessible before the view is dropped.
/// This gives a set of traits different from [`BowlRef`].
/// If you would like to have access to the owner after deriving the view,
/// please store it in the view.
///
/// # Examples
///
/// ```
/// use oyakodon::BowlMut;
///
/// fn parse_words(s: &mut String) -> Vec<&str> {
///     s.split_whitespace().collect()
/// }
///
/// let mut bowl = BowlMut::new(Box::new("hello world".to_owned()), parse_words);
/// bowl.get_mut()[0] = "hi";
/// assert_eq!(*bowl.get(), vec!["hi", "world"]);
/// ```
pub struct BowlMut<'a, T: Deref, F: ?Sized>(BowlRef<'a, T, MutToRef<F>>)
where
    F: for<'b> View<&'b mut T::Target>;

impl<'a, T, F> BowlMut<'a, T, F>
where
    T: StableDeref + DerefMut,
    F: for<'b> Derive<&'b mut T::Target>,
{
    pub fn new(owner: T, derive: F) -> Self {
        Self::from_derive(owner, derive)
    }
}

impl<'a, T, F> BowlMut<'a, T, F>
where
    T: StableDeref + DerefMut,
    F: for<'b> View<&'b mut T::Target> + ?Sized,
{
    /// See [`BowlRef::from_derive`]. The only difference is that `derive` receives `&mut T::Target`.
    pub fn from_derive(
        owner: T,
        derive: impl for<'b> Derive<&'b mut T::Target, Output = <F as View<&'b mut T::Target>>::Output>,
    ) -> Self {
        let mut owner = MaybeDangling::new(owner);
        // SAFETY: The difference of `BowlMut` and `BowlRef` is
        // only the mutability of the owner when deriving the view.
        // That restricts access to the owner after the view is alive,
        // but it does not change anything here.
        let view =
            derive.call(unsafe { transmute::<&mut T::Target, &'a mut T::Target>(&mut **owner) });
        // SAFETY: Same as `BowlRef::from_derive()`.
        Self(unsafe { BowlRef::new_unchecked(owner, MaybeDangling::new(view)) })
    }

    /// See [`BowlRef::from_fn`].
    pub fn from_fn<'b>(
        owner: T,
        derive: &'b dyn for<'c> Fn(&'c mut T::Target) -> <F as View<&'c mut T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(owner, derive)
    }

    /// See [`BowlRef::from_fn_mut`].
    pub fn from_fn_mut<'b>(
        owner: T,
        derive: &'b mut dyn for<'c> FnMut(
            &'c mut T::Target,
        ) -> <F as View<&'c mut T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(owner, derive)
    }

    /// See [`BowlRef::from_fn_once`].
    #[cfg(feature = "alloc")]
    pub fn from_fn_once(
        owner: T,
        derive: ::alloc::boxed::Box<
            dyn for<'c> FnOnce(&'c mut T::Target) -> <F as View<&'c mut T::Target>>::Output,
        >,
    ) -> Self {
        Self::from_derive(owner, derive)
    }

    /// See [`BowlRef::map`].
    pub fn map<G>(self, f: G) -> BowlMut<'a, T, Map<T::Target, F, G>>
    where
        G: for<'b> Derive<<F as View<&'b mut T::Target>>::Output>,
    {
        BowlMut(self.0.map(f).cast())
    }
}

impl<'a, 'b, T, F> AsRef<BowlMut<'b, T, F>> for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c mut T::Target> + ?Sized,
{
    fn as_ref(&self) -> &BowlMut<'b, T, F> {
        // SAFETY: `#[repr(transparent)]` delegates to the same safety contract as `BowlRef::as_ref()`
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F> AsMut<BowlMut<'b, T, F>> for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c mut T::Target> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlMut<'b, T, F> {
        // SAFETY: `#[repr(transparent)]` delegates to the same safety contract as `BowlRef::as_mut()`
        unsafe { transmute(self) }
    }
}

impl<'a, T, F> BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b mut T::Target> + ?Sized,
{
    /// See [`BowlRef::cast_life`].
    pub fn cast_life<'b>(self) -> BowlMut<'b, T, F> {
        BowlMut(self.0.cast_life())
    }

    /// See [`BowlRef::cast_view`].
    pub fn cast_view<
        G: ?Sized + for<'b> View<&'b mut T::Target, Output = <F as View<&'b mut T::Target>>::Output>,
    >(
        self,
    ) -> BowlMut<'a, T, G> {
        BowlMut(self.0.cast_view())
    }

    /// See [`BowlRef::cast`].
    pub fn cast<
        'b,
        G: ?Sized + for<'c> View<&'c mut T::Target, Output = <F as View<&'c mut T::Target>>::Output>,
    >(
        self,
    ) -> BowlMut<'b, T, G> {
        BowlMut(self.0.cast())
    }

    /// See [`BowlRef::into_owner`].
    pub fn into_owner(self) -> T {
        self.0.into_owner()
    }

    /// See [`BowlRef::into_view`].
    pub fn into_view<S>(self) -> S
    where
        for<'c> F: View<&'c mut T::Target, Output = S>,
    {
        self.0.into_view()
    }

    /// See [`BowlRef::into_parts`].
    pub fn into_parts<S>(self) -> (T, S)
    where
        for<'c> F: View<&'c mut T::Target, Output = S>,
    {
        self.0.into_parts()
    }

    /// See [`BowlRef::into_async`].
    pub async fn into_async(self) -> BowlMut<'a, T, Async<T::Target, F>>
    where
        for<'c> <F as View<&'c mut T::Target>>::Output: Future,
    {
        BowlMut(self.0.into_async().await.cast_view())
    }

    /// See [`BowlRef::into_result`].
    pub fn into_result(
        self,
    ) -> Result<BowlMut<'a, T, Success<T::Target, F>>, BowlMut<'a, T, Failure<T::Target, F>>>
    where
        for<'b> <F as View<&'b mut T::Target>>::Output: Outcome,
    {
        use Result::*;
        match self.0.into_result() {
            Ok(v) => Ok(BowlMut(v.cast_view())),
            Err(e) => Err(BowlMut(e.cast_view())),
        }
    }

    pub fn spawn<'b, S>(
        &'b self,
        spawn: impl for<'c> Derive<&'b <F as View<&'c mut T::Target>>::Output, Output = S>,
    ) -> S {
        self.0.spawn(spawn)
    }

    pub fn spawn_mut<'b, S>(
        &'b mut self,
        spawn: impl for<'c> Derive<&'b mut <F as View<&'c mut T::Target>>::Output, Output = S>,
    ) -> S {
        self.0.spawn_mut(spawn)
    }
}

// SAFETY: We do not provide access to `&*owner` since it can be stored in `view`.
// That gives us the flexibility to omit `T: Sync`.
unsafe impl<'a, T, F> Sync for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b mut T::Target> + ?Sized,
    for<'b> <F as View<&'b mut T::Target>>::Output: Sync,
{
}

impl<'a, T, F> Debug for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b mut T::Target> + ?Sized,
    for<'b> <F as View<&'b mut T::Target>>::Output: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.spawn(|view: &<F as View<&mut T::Target>>::Output| {
            f.debug_struct("BowlMut")
                .field("view", view)
                .finish_non_exhaustive()
        })
    }
}

impl<'a, 'b, T, F, G> From<BowlRef<'a, T, F>> for BowlMut<'b, T, G>
where
    T: Deref,
    F: for<'c> View<&'c T::Target> + ?Sized,
    G: for<'c> View<&'c mut T::Target, Output = <F as View<&'c T::Target>>::Output> + ?Sized,
{
    fn from(bowl_ref: BowlRef<'a, T, F>) -> Self {
        // SAFETY: The difference of `BowlMut` and `BowlRef` is
        // only the mutability of the owner when deriving the view.
        // If we only had `&T::Target` when deriving the view,
        // we can certainly derive the same view with `&mut T::Target`.
        BowlMut(bowl_ref.cast())
    }
}
