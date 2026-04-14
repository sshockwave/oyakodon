// The safety arguments in this file are mostly the same as `BowlRef`, so they are not repeated here.
// These two implementations are not merged using macros because they break rust-analyzer.
// We also cannot use a common trait
// because an explicit reference `&'a T` in HRTB `for<'a> F: Ref<&'a T>`
// provide an implicit bound of on `'a` to have shorter lifetime than `T`
// while `for<'a> F: Ref<'a, T>` requires `T` to be static to satisfy `'a: 'static`.
// See https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats
// for an in-depth discussion on this issue.
// The code size increase is not significant anyway.

use super::{Derive, StableDeref, View};
use ::{
    core::{
        marker::PhantomData,
        mem::{forget, transmute},
        ops::{Deref, DerefMut},
    },
    maybe_dangling::MaybeDangling,
};

pub struct BowlMut<'a, T: Deref, F: ?Sized>
where
    F: for<'b> View<&'b mut T::Target>,
{
    derived: MaybeDangling<<F as View<&'a mut T::Target>>::Output>,
    base: MaybeDangling<T>,
}

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
        let derived = derive.call(unsafe { transmute(&mut **base) });
        BowlMut {
            base,
            derived: MaybeDangling::new(derived),
        }
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
        let Self { base, derived } = self;
        BowlMut::<'_, T, G> {
            base,
            derived: MaybeDangling::new(f.call(MaybeDangling::into_inner(derived))),
        }
        .cast()
    }

    pub fn map<G>(self, f: G) -> BowlMut<'a, T, Map<T::Target, F, G>>
    where
        G: for<'b> Derive<<F as View<&'b mut T::Target>>::Output>,
    {
        self.map_into(f)
    }
}

impl<'a, 'b, T, F, G> AsRef<BowlMut<'b, T, G>> for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c mut T::Target> + ?Sized,
    G: for<'c> View<&'c mut T::Target, Output = <F as View<&'c mut T::Target>>::Output> + ?Sized,
{
    fn as_ref(&self) -> &BowlMut<'b, T, G> {
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F, G> AsMut<BowlMut<'b, T, G>> for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c mut T::Target> + ?Sized,
    G: for<'c> View<&'c mut T::Target, Output = <F as View<&'c mut T::Target>>::Output> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlMut<'b, T, G> {
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
        let result = unsafe { (&raw const self).cast::<BowlMut<'_, _, _>>().read() };
        forget(self);
        result
    }

    pub fn into_inner(self) -> T {
        let Self { base, derived: _ } = self;
        MaybeDangling::into_inner(base)
    }

    pub fn into_view<S>(self) -> S
    where
        for<'c> F: View<&'c mut T::Target, Output = S>,
    {
        MaybeDangling::into_inner(self.derived)
    }

    pub fn get(&self) -> &<F as View<&'_ mut T::Target>>::Output {
        let other: &BowlMut<_, F> = self.as_ref();
        &*other.derived
    }
    pub fn get_mut(&mut self) -> &mut <F as View<&'_ mut T::Target>>::Output {
        let other: &mut BowlMut<_, F> = self.as_mut();
        &mut *other.derived
    }
}

unsafe impl<'a, T, F> Sync for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b mut T::Target> + ?Sized,
    for<'b> <F as View<&'b mut T::Target>>::Output: Sync,
{
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
        BowlMut::get(self)
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        BowlMut::get_mut(self)
    }
}

pub struct Map<T: ?Sized, F: ?Sized, G: ?Sized>(PhantomData<T>, PhantomData<F>, G);
impl<'a, T: ?Sized, F, G> View<&'a mut T> for Map<T, F, G>
where
    F: for<'b> View<&'b mut T> + ?Sized,
    G: for<'b> View<<F as View<&'b mut T>>::Output> + ?Sized,
{
    type Output = <G as View<<F as View<&'a mut T>>::Output>>::Output;
}
