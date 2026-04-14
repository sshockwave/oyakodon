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
use super::{BowlRef, Derive, Map, StableDeref, View};
use ::{
    core::{
        mem::transmute,
        ops::{Deref, DerefMut},
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

pub struct BowlMut<'a, T: Deref, F: ?Sized>(BowlRef<'a, T, MutToRef<F>>)
where
    F: for<'b> View<&'b mut T::Target>;

impl<'a, T, F> BowlMut<'a, T, F>
where
    T: StableDeref + DerefMut,
    F: for<'b> Derive<&'b mut T::Target>,
{
    pub fn new(base: T, derive: F) -> Self {
        Self::from_derive(base, derive)
    }
}

impl<'a, T, F> BowlMut<'a, T, F>
where
    T: StableDeref + DerefMut,
    F: for<'b> View<&'b mut T::Target> + ?Sized,
{
    pub fn from_derive(
        base: T,
        derive: impl for<'b> Derive<&'b mut T::Target, Output = <F as View<&'b mut T::Target>>::Output>,
    ) -> Self {
        let mut base = MaybeDangling::new(base);
        // SAFETY: The difference of `BowlMut` and `BowlRef` is
        // only the mutability of the base when deriving the view.
        // That restricts access to the base after the view is alive,
        // but it does not change anything here.
        let derived = derive.call(unsafe { transmute(&mut **base) });
        Self(BowlRef {
            base,
            derived: MaybeDangling::new(derived),
        })
    }

    pub fn from_fn<'b>(
        base: T,
        derive: &'b dyn for<'c> Fn(&'c mut T::Target) -> <F as View<&'c mut T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(base, derive)
    }

    pub fn from_fn_mut<'b>(
        base: T,
        derive: &'b mut dyn for<'c> FnMut(
            &'c mut T::Target,
        ) -> <F as View<&'c mut T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(base, derive)
    }

    #[cfg(feature = "alloc")]
    pub fn from_fn_once(
        base: T,
        derive: ::alloc::boxed::Box<
            dyn for<'c> FnOnce(&'c mut T::Target) -> <F as View<&'c mut T::Target>>::Output,
        >,
    ) -> Self {
        Self::from_derive(base, derive)
    }

    pub fn map_into<'b, G: ?Sized, H>(self, f: H) -> BowlMut<'b, T, G>
    where
        for<'c> H: Derive<<F as View<&'c mut T::Target>>::Output>,
        for<'c> G: View<
                &'c mut T::Target,
                Output = <H as View<<F as View<&'c mut T::Target>>::Output>>::Output,
            >,
    {
        BowlMut(self.0.map_into(f))
    }

    pub fn map<G>(self, f: G) -> BowlMut<'a, T, Map<T::Target, F, G>>
    where
        G: for<'b> Derive<<F as View<&'b mut T::Target>>::Output>,
    {
        self.map_into(f)
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
    pub fn cast<'b, G: ?Sized>(self) -> BowlMut<'b, T, G>
    where
        for<'c> G: View<&'c mut T::Target, Output = <F as View<&'c mut T::Target>>::Output>,
    {
        BowlMut(self.0.cast())
    }

    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }

    pub fn into_view<S>(self) -> S
    where
        for<'c> F: View<&'c mut T::Target, Output = S>,
    {
        self.0.into_view()
    }

    pub fn get(&self) -> &<F as View<&'_ mut T::Target>>::Output {
        self.0.get()
    }
    pub fn get_mut(&mut self) -> &mut <F as View<&'_ mut T::Target>>::Output {
        self.0.get_mut()
    }
}

#[cfg(feature = "gat")]
impl<'a, T, F> super::Bowl for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b mut T::Target> + ?Sized,
{
    type Value<'b>
        = <F as View<&'b mut T::Target>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        self.0.get()
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        self.0.get_mut()
    }
}
