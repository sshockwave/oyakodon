// The safety arguments in this file are mostly the same as `BowlRef`, so they are not repeated here.
// These two implementations are not merged using macros because they break rust-analyzer.
// We also cannot use a common trait
// because an explicit reference `&'a T` in HRTB `for<'a> F: Ref<&'a T>`
// provide an implicit bound of on `'a` to have shorter lifetime than `T`
// while `for<'a> F: Ref<'a, T>` requires `T` to be static to satisfy `'a: 'static`.
// See https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats
// for an in-depth discussion on this issue.
// The code size increase is not significant anyway.

use super::{Derive, StableDeref};
#[cfg(feature = "alloc")]
use ::alloc::boxed::Box;
use ::core::{
    marker::PhantomData,
    mem::transmute,
    ops::{Deref, DerefMut},
};

pub struct BowlMut<'a, T: Deref, F: for<'b> Derive<&'b mut T::Target>> {
    derived: <F as Derive<&'a mut T::Target>>::Output,
    base: T,
}

#[cfg(feature = "alloc")]
impl<'a, T, F> BowlMut<'a, Box<T>, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    pub fn new(base: T, derive: F) -> Self {
        Self::from_ptr(Box::new(base), derive)
    }
    pub fn new_into(
        base: T,
        derive: impl for<'b> Derive<&'b mut T, Output = <F as Derive<&'b mut T>>::Output>,
    ) -> Self {
        Self::from_ptr_into(Box::new(base), derive)
    }
    pub fn into_inner(self) -> T {
        *self.into_ptr()
    }

    #[cfg(feature = "alloc")]
    pub fn from_fn<'b>(
        base: T,
        derive: &'b dyn for<'c> Fn(&'c mut T) -> <F as Derive<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_ptr_into(Box::new(base), derive)
    }

    #[cfg(feature = "alloc")]
    pub fn from_fn_mut<'b>(
        base: T,
        derive: &'b mut dyn for<'c> FnMut(&'c mut T) -> <F as Derive<&'c mut T>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_ptr_into(Box::new(base), derive)
    }

    #[cfg(feature = "alloc")]
    pub fn from_fn_once(
        base: T,
        derive: Box<dyn for<'c> FnOnce(&'c mut T) -> <F as Derive<&'c mut T>>::Output>,
    ) -> Self {
        Self::from_ptr_into(Box::new(base), derive)
    }
}

impl<'a, T, F> BowlMut<'a, T, F>
where
    T: StableDeref + DerefMut,
    F: for<'b> Derive<&'b mut T::Target>,
{
    pub fn from_ptr_into(
        mut base: T,
        derive: impl for<'b> Derive<
            &'b mut T::Target,
            Output = <F as Derive<&'b mut T::Target>>::Output,
        >,
    ) -> Self {
        let derived = derive.call(unsafe { transmute(&mut *base) });
        BowlMut { base, derived }
    }

    pub fn from_ptr(base: T, derive: F) -> Self {
        Self::from_ptr_into(base, derive)
    }

    pub fn map_into<'b, G, H>(self, f: H) -> BowlMut<'b, T, G>
    where
        for<'c> H: Derive<<F as Derive<&'c mut T::Target>>::Output>,
        for<'c> G: Derive<
                &'c mut T::Target,
                Output = <H as Derive<<F as Derive<&'c mut T::Target>>::Output>>::Output,
            >,
    {
        let Self { base, derived } = self;
        BowlMut::<'_, T, G> {
            base,
            derived: f.call(derived),
        }
        .cast()
    }

    pub fn map<G>(self, f: G) -> BowlMut<'a, T, Map<T::Target, F, G>>
    where
        G: for<'b> Derive<<F as Derive<&'b mut T::Target>>::Output>,
    {
        self.map_into(f)
    }
}

impl<'a, T, F> BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> Derive<&'b mut T::Target>,
{
    pub fn cast_ref<'b, G>(&self) -> &BowlMut<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T::Target, Output = <F as Derive<&'c mut T::Target>>::Output>,
    {
        unsafe { transmute(self) }
    }
    pub fn cast_mut<'b, G>(&mut self) -> &mut BowlMut<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T::Target, Output = <F as Derive<&'c mut T::Target>>::Output>,
    {
        unsafe { transmute(self) }
    }
    pub fn cast<'b, G>(self) -> BowlMut<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T::Target, Output = <F as Derive<&'c mut T::Target>>::Output>,
    {
        unsafe { (&raw const self).cast::<BowlMut<'_, _, _>>().read() }
    }

    pub fn get(&self) -> &<F as Derive<&'_ mut T::Target>>::Output {
        &self.cast_ref::<F>().derived
    }
    pub fn get_mut(&mut self) -> &mut <F as Derive<&'_ mut T::Target>>::Output {
        &mut self.cast_mut::<F>().derived
    }

    pub fn into_ptr(self) -> T {
        let Self { base, derived: _ } = self;
        base
    }
}

// This is possible if `Derived=Rc<RefCell<&mut T::Target>>`.
impl<'a, T, F> Clone for BowlMut<'a, T, F>
where
    T: super::CloneStableDeref,
    F: for<'b> Derive<&'b mut T::Target>,
    for<'b> <F as Derive<&'b mut T::Target>>::Output: Clone,
{
    fn clone(&self) -> Self {
        let base = self.base.clone();
        let derived = self.derived.clone();
        Self { base, derived }
    }
}

unsafe impl<'a, T, F> Send for BowlMut<'a, T, F>
where
    T: StableDeref + Send,
    F: for<'b> Derive<&'b mut T::Target>,
    for<'b> <F as Derive<&'b mut T::Target>>::Output: Send,
{
}
unsafe impl<'a, T, F> Sync for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> Derive<&'b mut T::Target>,
    for<'b> <F as Derive<&'b mut T::Target>>::Output: Sync,
{
}

#[cfg(feature = "gat")]
impl<'a, T, F> super::Bowl for BowlMut<'a, T, F>
where
    T: Deref,
    F: for<'b> Derive<&'b mut T::Target>,
{
    type Value<'b>
        = <F as Derive<&'b mut T::Target>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        BowlMut::get(self)
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        BowlMut::get_mut(self)
    }
}

pub struct Map<T: ?Sized, F, G>(G, PhantomData<(F, T)>);
impl<'a, T: ?Sized, F, G> Derive<&'a mut T> for Map<T, F, G>
where
    F: for<'b> Derive<&'b mut T>,
    G: for<'b> Derive<<F as Derive<&'b mut T>>::Output>,
{
    type Output = <G as Derive<<F as Derive<&'a mut T>>::Output>>::Output;
}
