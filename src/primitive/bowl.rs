use super::{Aliasable, CloneStableDeref};
use ::core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use maybe_dangling::MaybeDangling;

pub trait View<'x> {
    type ImplyBound<X>;
    type Output;
}

impl<'x, F: ?Sized, R> View<'x> for F
where
    F: FnOnce(&'x ()) -> R,
{
    type ImplyBound<X> = ();
    type Output = R;
}

pub struct Bowl<'ub, P, F: View<'ub> + ?Sized> {
    view: MaybeDangling<F::Output>,
    owner: Handle<'ub, 'ub, P, &'ub &'ub ()>,
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> Fn(&'x ()) -> &'x P::Target>
where
    P: Aliasable + Deref,
{
    pub fn new(owner: P) -> Self {
        let derived = Stamp(PhantomData).stamp(owner.deref());
        Slot(owner, PhantomData).fill(derived)
    }
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> Fn(&'x ()) -> &'x mut P::Target>
where
    P: Aliasable + DerefMut,
{
    pub fn new_mut(mut owner: P) -> Self {
        let derived = Stamp(PhantomData).stamp(owner.deref_mut());
        Slot(owner, PhantomData).fill(derived)
    }
}

pub struct Stamp<'brand, 'life, 'ub>(PhantomData<(&'brand (), &'life (), &'ub ())>);

impl<'brand, 'life, 'ub> Stamp<'brand, 'life, 'ub> {
    pub fn stamp<F>(&self, view: <F as View<'life>>::Output) -> Derived<'brand, 'ub, F>
    where
        F: ?Sized + for<'x> View<'x, ImplyBound<&'x &'ub ()> = ()>,
    {
        let view = unsafe {
            ::core::mem::transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view)
        };
        Derived(view, PhantomData)
    }
}

#[derive(Clone, Copy)]
pub struct Derived<'brand, 'ub, F: View<'ub> + ?Sized>(
    F::Output,
    PhantomData<(&'brand (), &'ub (), F)>,
);

pub struct Slot<'brand, 'ub, P>(P, PhantomData<(&'brand (), &'ub ())>);

impl<'brand, 'ub, P> Slot<'brand, 'ub, P> {
    pub fn fill<F>(self, view: Derived<'brand, 'ub, F>) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> View<'x, ImplyBound<&'x &'ub ()> = ()>,
    {
        Bowl {
            view: MaybeDangling::new(view.0),
            owner: Handle(self.0, PhantomData),
        }
    }

    pub fn into_inner(self) -> P {
        self.0
    }
}

impl<'brand, 'ub, P> Clone for Slot<'brand, 'ub, P>
where
    P: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

pub struct Handle<'life, 'ub, P, X>(P, PhantomData<(&'life (), &'ub (), X)>);

impl<'life, 'ub, P, X> Clone for Handle<'life, 'ub, P, X>
where
    P: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<'life, 'ub, P, X> Handle<'life, 'ub, P, X> {
    pub fn into_inner(self) -> P {
        self.0
    }

    pub fn fill<F>(self, view: <F as View<'life>>::Output) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> View<'x, ImplyBound<&'x &'ub ()> = ()>,
    {
        Slot(self.0, PhantomData).fill(Stamp(PhantomData).stamp(view))
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> View<'x, ImplyBound<&'x &'ub ()> = ()>,
{
    pub fn with<'a, R>(
        &'a self,
        f: impl for<'life> FnOnce(
            &'a <F as View<'life>>::Output,
            &'a Handle<'life, 'ub, P, &'a &'life ()>,
        ) -> R,
    ) -> R {
        f(&*self.view, &self.owner)
    }

    pub fn with_mut<'a, R>(
        &'a mut self,
        f: impl for<'life> FnOnce(
            &'a mut <F as View<'life>>::Output,
            &'a Handle<'life, 'ub, P, &'a &'life ()>,
        ) -> R,
    ) -> R {
        f(&mut *self.view, &self.owner)
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + View<'ub>,
{
    pub fn map<R>(self, f: impl for<'brand> FnOnce(Session<'brand, 'ub, P, F>) -> R) -> R {
        f(Session(self, PhantomData))
    }
}

pub struct Session<'brand, 'ub, P, F>(Bowl<'ub, P, F>, PhantomData<&'brand ()>)
where
    F: View<'ub> + ?Sized;

impl<'brand, 'ub, P, F> Session<'brand, 'ub, P, F>
where
    F: ?Sized + for<'x> View<'x, ImplyBound<&'x &'ub ()> = ()>,
{
    pub fn open<R>(
        self,
        f: impl for<'life> FnOnce(<F as View<'life>>::Output, Stamp<'brand, 'life, 'ub>) -> R,
    ) -> (R, Slot<'brand, 'ub, P>) {
        (
            f(MaybeDangling::into_inner(self.0.view), Stamp(PhantomData)),
            Slot(self.0.owner.0, PhantomData),
        )
    }
}
