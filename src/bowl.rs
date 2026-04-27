use core::{marker::PhantomData, mem::transmute};
use maybe_dangling::MaybeDangling;

use crate::CloneStableDeref;

pub unsafe trait AliasableDeref {
    type Target: ?Sized;
    fn deref(&self) -> &Self::Target;
}

pub unsafe trait AliasableDerefMut: AliasableDeref {
    fn deref_mut(&mut self) -> &mut Self::Target;
}

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

pub struct Bowl<'ub, P, V, F: ?Sized>
where
    P: AliasableDeref,
    P::Target: 'ub,
    F: for<'x> View<'x, 'ub>,
    V: IsType<<F as View<'ub, 'ub>>::Output>,
{
    view: MaybeDangling<V>,
    // Covariant over `'ub` is safe because it maintains the HRTB invariant.
    owner: Token4<'ub, 'ub, P, &'ub &'ub ()>,
    phantom: PhantomData<F>,
}

pub struct RefView<'ub, T: ?Sized>(PhantomData<&'ub T>);
impl<'a, 'ub, T: ?Sized> View<'a, 'ub> for RefView<'ub, T> {
    type Output = &'a T;
}

pub struct MutView<'ub, T: ?Sized>(PhantomData<&'ub mut T>);
impl<'a, 'ub, T: ?Sized> View<'a, 'ub> for MutView<'ub, T> {
    type Output = &'a mut T;
}

impl<'ub, P> Bowl<'ub, P, &'ub P::Target, RefView<'ub, P::Target>>
where
    P: AliasableDeref,
{
    pub fn new(owner: P) -> Self {
        let view = owner.deref();
        let view = unsafe { transmute::<&P::Target, &P::Target>(view) };
        Self {
            view: MaybeDangling::new(view),
            owner: Token4(owner, PhantomData),
            phantom: PhantomData,
        }
    }
}

impl<'ub, P> Bowl<'ub, P, &'ub mut P::Target, MutView<'ub, P::Target>>
where
    P: AliasableDerefMut,
{
    pub fn new_mut(mut owner: P) -> Self {
        let view = owner.deref_mut();
        let view = unsafe { transmute::<&mut P::Target, &mut P::Target>(view) };
        Self {
            view: MaybeDangling::new(view),
            owner: Token4(owner, PhantomData),
            phantom: PhantomData,
        }
    }
}

pub struct Token1<'brand, 'life, 'ub>(PhantomData<(&'brand (), &'life (), &'ub ())>);

impl<'brand, 'life, 'ub> Token1<'brand, 'life, 'ub> {
    pub fn make<F>(&self, view: <F as View<'life, 'ub>>::Output) -> Token2<'brand, 'ub, F>
    where
        F: ?Sized + for<'x> View<'x, 'ub>,
    {
        let view = unsafe {
            transmute::<<F as View<'life, 'ub>>::Output, <F as View<'ub, 'ub>>::Output>(view)
        };
        Token2(view, PhantomData)
    }
}

#[derive(Clone, Copy)]
pub struct Token2<'brand, 'ub, F: View<'ub, 'ub> + ?Sized>(
    F::Output,
    PhantomData<(&'brand (), &'ub (), F)>,
);

pub struct Token3<'brand, 'ub, P>(P, PhantomData<(&'brand (), &'ub ())>);

impl<'brand, 'ub, P> Token3<'brand, 'ub, P>
where
    P: AliasableDeref,
{
    pub fn consume<F>(
        self,
        view: Token2<'brand, 'ub, F>,
    ) -> Bowl<'ub, P, <F as View<'ub, 'ub>>::Output, F>
    where
        F: for<'x> View<'x, 'ub>,
    {
        Bowl {
            view: MaybeDangling::new(view.0),
            owner: Token4(self.0, PhantomData),
            phantom: PhantomData,
        }
    }
}

impl<'brand, 'ub, P> Token3<'brand, 'ub, P> {
    pub fn into_inner(self) -> P {
        self.0
    }
}

impl<'brand, 'ub, P> Clone for Token3<'brand, 'ub, P>
where
    P: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

pub struct Token4<'life, 'ub, P, X>(P, PhantomData<(&'life (), &'ub (), X)>);

impl<'life, 'ub, P, X> Token4<'life, 'ub, P, X> {
    pub fn into_inner(self) -> P {
        self.0
    }
}

impl<'life, 'ub, P, X> Clone for Token4<'life, 'ub, P, X>
where
    P: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<'life, 'ub, P, X> Token4<'life, 'ub, P, X>
where
    P: AliasableDeref,
{
    pub fn consume<F>(
        self,
        view: <F as View<'life, 'ub>>::Output,
    ) -> Bowl<'ub, P, <F as View<'ub, 'ub>>::Output, F>
    where
        F: for<'x> View<'x, 'ub>,
    {
        let view = unsafe {
            transmute::<<F as View<'life, 'ub>>::Output, <F as View<'ub, 'ub>>::Output>(view)
        };
        Bowl {
            view: MaybeDangling::new(view),
            owner: Token4(self.0, PhantomData),
            phantom: PhantomData,
        }
    }
}

type Token4Ref<'a, 'life, 'ub, P> = &'a Token4<'life, 'ub, P, &'a &'life ()>;
type ViewOut<'life, 'ub, F> = <F as View<'life, 'ub>>::Output;
type DeriveOut<G, T, S> = <G as Derive2<T, S>>::Output;
type WithRet<'a, 'life, 'ub, P, F, G> =
    <G as Derive2<&'a ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>>::Output;
type WithMutRet<'a, 'life, 'ub, P, F, G> =
    <G as Derive2<&'a mut ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>>::Output;

impl<'ub, P, V, F> Bowl<'ub, P, V, F>
where
    P: AliasableDeref,
    P::Target: 'ub,
    F: for<'x> View<'x, 'ub>,
    V: IsType<<F as View<'ub, 'ub>>::Output>,
    V: AsRef<<F as View<'ub, 'ub>>::Output>,
    V: AsMut<<F as View<'ub, 'ub>>::Output>,
{
    pub fn with<'a, G>(
        &'a self,
        g: G,
    ) -> DeriveOut<G, &'a <F as View<'ub, 'ub>>::Output, Token4Ref<'a, 'ub, 'ub, P>>
    where
        G: for<'life> Derive2<&'a ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>,
        for<'life> WithRet<'a, 'life, 'ub, P, F, G>: IsType<WithRet<'a, 'ub, 'ub, P, F, G>>,
    {
        g.derive(self.view.as_ref(), &self.owner)
    }

    pub fn with_mut<'a, G>(&'a mut self, g: G) -> WithMutRet<'a, 'ub, 'ub, P, F, G>
    where
        G: for<'life> Derive2<&'a mut ViewOut<'life, 'ub, F>, Token4Ref<'a, 'life, 'ub, P>>,
        for<'life> WithMutRet<'a, 'life, 'ub, P, F, G>: IsType<WithMutRet<'a, 'ub, 'ub, P, F, G>>,
    {
        g.derive(self.view.as_mut(), &self.owner)
    }

    pub fn map<G, H>(
        self,
        g: G,
        h: H,
    ) -> <H as Derive2<
        <G as Derive2<<F as View<'ub, 'ub>>::Output, Token1<'static, 'ub, 'ub>>>::Output,
        Token3<'static, 'ub, P>,
    >>::Output
    where
        G: for<'life, 'brand> Derive2<<F as View<'life, 'ub>>::Output, Token1<'brand, 'life, 'ub>>,
        for<'life, 'brand> <G as Derive2<<F as View<'life, 'ub>>::Output, Token1<'brand, 'life, 'ub>>>::Output:
            IsType<<G as Derive2<<F as View<'ub, 'ub>>::Output, Token1<'brand, 'ub, 'ub>>>::Output>,
        H: for<'brand> Derive2<
            <G as Derive2<<F as View<'ub, 'ub>>::Output, Token1<'brand, 'ub, 'ub>>>::Output,
            Token3<'brand, 'ub, P>,
        >,
        for<'brand> <H as Derive2<
            <G as Derive2<<F as View<'ub, 'ub>>::Output, Token1<'brand, 'ub, 'ub>>>::Output,
            Token3<'brand, 'ub, P>,
        >>::Output: IsType<
            <H as Derive2<
                <G as Derive2<<F as View<'ub, 'ub>>::Output, Token1<'static, 'ub, 'ub>>>::Output,
                Token3<'static, 'ub, P>,
            >>::Output,
        >,
    {
        h.derive(
            g.derive(
                MaybeDangling::into_inner(self.view).get(),
                Token1(PhantomData),
            ),
            Token3(self.owner.0, PhantomData),
        )
    }
}
