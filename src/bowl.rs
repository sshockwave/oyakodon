use core::{marker::PhantomData, mem::transmute};
use maybe_dangling::MaybeDangling;

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
    <F as View<'ub, 'ub>>::Output: IsType<V>,
{
    view: MaybeDangling<V>,
    owner: P,
    // Covariant over `'ub` is safe because it maintains the HRTB invariant.
    phantom: PhantomData<(&'ub (), F)>,
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
            owner,
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
            owner,
            phantom: PhantomData,
        }
    }
}

pub struct Token1<'brand, 'life, 'ub>(PhantomData<(&'brand (), &'life (), &'ub ())>);

impl<'brand, 'life, 'ub> Token1<'brand, 'life, 'ub> {
    pub fn make<F>(
        &self,
        view: <F as View<'life, 'ub>>::Output,
    ) -> Token2<'brand, 'ub, F, <F as View<'ub, 'ub>>::Output>
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
pub struct Token2<'brand, 'ub, F: ?Sized, V>(V, PhantomData<(&'brand (), &'ub (), F)>);

pub struct Token3<'brand, 'ub, P>(P, PhantomData<(&'brand (), &'ub ())>);

impl<'brand, 'ub, P> Token3<'brand, 'ub, P>
where
    P: AliasableDeref,
{
    pub fn consume<F>(
        self,
        view: Token2<'brand, 'ub, F, <F as View<'ub, 'ub>>::Output>,
    ) -> Bowl<'ub, P, <F as View<'ub, 'ub>>::Output, F>
    where
        F: for<'x> View<'x, 'ub>,
    {
        Bowl {
            view: MaybeDangling::new(view.0),
            owner: self.0,
            phantom: PhantomData,
        }
    }
}

impl<'brand, 'ub, P> Token3<'brand, 'ub, P> {
    pub fn into_inner(self) -> P {
        self.0
    }
}

// TODO: impl Clone Token3 for P: CloneStableRef

impl<'ub, P, V, F> Bowl<'ub, P, V, F>
where
    P: AliasableDeref,
    P::Target: 'ub,
    F: for<'x> View<'x, 'ub>,
    <F as View<'ub, 'ub>>::Output: IsType<V>,
{
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
        todo!()
    }
}
