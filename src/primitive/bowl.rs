use super::{Aliasable, CloneStableDeref};
use ::core::{
    marker::PhantomData,
    mem::transmute,
    ops::{Deref, DerefMut},
};
use maybe_dangling::MaybeDangling;

pub trait View<'x, 'ub, __ImplyBound = &'x &'ub ()> {
    type Output;
}

pub trait Derive2<T, S> {
    type Output;
    fn derive(self, a: T, b: S) -> Self::Output;
}

impl<T, S, R, F> Derive2<T, S> for F
where
    F: FnOnce(T, S) -> R,
{
    type Output = R;
    fn derive(self, a: T, b: S) -> R {
        self(a, b)
    }
}

pub trait IsType<T> {
    fn get(self) -> T;
}
impl<T> IsType<T> for T {
    fn get(self) -> T {
        self
    }
}

pub struct Bowl<'ub, P, F: View<'ub, 'ub> + ?Sized> {
    view: MaybeDangling<F::Output>,
    owner: Handle<'ub, 'ub, P, &'ub &'ub ()>,
}

pub struct RefView<'ub, T: ?Sized>(PhantomData<&'ub T>);
impl<'a, 'ub, T: ?Sized> View<'a, 'ub> for RefView<'ub, T> {
    type Output = &'a T;
}

pub struct MutView<'ub, T: ?Sized>(PhantomData<&'ub mut T>);
impl<'a, 'ub, T: ?Sized> View<'a, 'ub> for MutView<'ub, T> {
    type Output = &'a mut T;
}

impl<'ub, P> Bowl<'ub, P, RefView<'ub, P::Target>>
where
    P: Aliasable + Deref,
{
    pub fn new(owner: P) -> Self {
        let view = owner.deref();
        let view = unsafe { transmute::<&P::Target, &P::Target>(view) };
        Self {
            view: MaybeDangling::new(view),
            owner: Handle(owner, PhantomData),
        }
    }
}

impl<'ub, P> Bowl<'ub, P, MutView<'ub, P::Target>>
where
    P: Aliasable + DerefMut,
{
    pub fn new_mut(mut owner: P) -> Self {
        let view = owner.deref_mut();
        let view = unsafe { transmute::<&mut P::Target, &mut P::Target>(view) };
        Self {
            view: MaybeDangling::new(view),
            owner: Handle(owner, PhantomData),
        }
    }
}

pub struct Stamp<'brand, 'life, 'ub>(PhantomData<(&'brand (), &'life (), &'ub ())>);

impl<'brand, 'life, 'ub> Stamp<'brand, 'life, 'ub> {
    pub fn make<F>(&self, view: <F as View<'life, 'ub>>::Output) -> Derived<'brand, 'ub, F>
    where
        F: ?Sized + for<'x> View<'x, 'ub>,
    {
        let view = unsafe {
            transmute::<<F as View<'life, 'ub>>::Output, <F as View<'ub, 'ub>>::Output>(view)
        };
        Derived(view, PhantomData)
    }
}

#[derive(Clone, Copy)]
pub struct Derived<'brand, 'ub, F: View<'ub, 'ub> + ?Sized>(
    F::Output,
    PhantomData<(&'brand (), &'ub (), F)>,
);

pub struct Slot<'brand, 'ub, P>(P, PhantomData<(&'brand (), &'ub ())>);

impl<'brand, 'ub, P> Slot<'brand, 'ub, P> {
    pub fn consume<F>(self, view: Derived<'brand, 'ub, F>) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> View<'x, 'ub>,
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

    pub fn consume<F>(self, view: <F as View<'life, 'ub>>::Output) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> View<'x, 'ub>,
    {
        let view = unsafe {
            transmute::<<F as View<'life, 'ub>>::Output, <F as View<'ub, 'ub>>::Output>(view)
        };
        Bowl {
            view: MaybeDangling::new(view),
            owner: Handle(self.0, PhantomData),
        }
    }
}

type Token4Ref<'a, 'life, 'ub, P> = &'a Handle<'life, 'ub, P, &'a &'life ()>;
type ViewOut<'life, 'ub, F> = <F as View<'life, 'ub>>::Output;
type DeriveOut<G, T, S> = <G as Derive2<T, S>>::Output;
type WithRet<'a, 'life, 'ub, P, F, G> =
    DeriveOut<G, &'a ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>;

type WithMutRet<'a, 'life, 'ub, P, F, G> =
    DeriveOut<G, &'a mut ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>;

type MapGRet<'life, 'ub, 'brand, G, F> =
    DeriveOut<G, ViewOut<'life, 'ub, F>, Stamp<'brand, 'life, 'ub>>;
type MapHRet<'ub, 'brand, P, F, G, H> =
    DeriveOut<H, MapGRet<'ub, 'ub, 'brand, G, F>, Slot<'brand, 'ub, P>>;

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> View<'x, 'ub>,
{
    pub fn with<'a, G>(
        &'a self,
        g: G,
    ) -> DeriveOut<G, &'a <F as View<'ub, 'ub>>::Output, Token4Ref<'a, 'ub, 'ub, P>>
    where
        G: for<'life> Derive2<&'a ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>,
        for<'life> WithRet<'a, 'life, 'ub, P, F, G>: IsType<WithRet<'a, 'ub, 'ub, P, F, G>>,
    {
        g.derive(&*self.view, &self.owner)
    }

    pub fn with_mut<'a, G>(&'a mut self, g: G) -> WithMutRet<'a, 'ub, 'ub, P, F, G>
    where
        G: for<'life> Derive2<&'a mut ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>,
        for<'life> WithMutRet<'a, 'life, 'ub, P, F, G>: IsType<WithMutRet<'a, 'ub, 'ub, P, F, G>>,
    {
        g.derive(&mut *self.view, &self.owner)
    }

    pub fn map<G, H>(self, g: G, h: H) -> MapHRet<'ub, 'static, P, F, G, H>
    where
        G: for<'life, 'brand> Derive2<ViewOut<'life, 'ub, F>, Stamp<'brand, 'life, 'ub>>,
        for<'life, 'brand> MapGRet<'life, 'ub, 'brand, G, F>:
            IsType<MapGRet<'ub, 'ub, 'brand, G, F>>,
        H: for<'brand> Derive2<MapGRet<'ub, 'ub, 'brand, G, F>, Slot<'brand, 'ub, P>>,
        for<'brand> MapHRet<'ub, 'brand, P, F, G, H>: IsType<MapHRet<'ub, 'static, P, F, G, H>>,
    {
        h.derive(
            g.derive(MaybeDangling::into_inner(self.view), Stamp(PhantomData)),
            Slot(self.owner.0, PhantomData),
        )
    }
}
