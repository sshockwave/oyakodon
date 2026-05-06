use super::{Aliasable, CloneStableDeref};
use ::{
    core::{
        marker::PhantomData,
        mem::transmute,
        ops::{Deref, DerefMut},
    },
    maybe_dangling::MaybeDangling,
};

pub trait View<'x> {
    type Output;
}

pub trait ViewIn<'x, 'ub, X = &'x &'ub ()>: View<'x, Output = Self::Target> {
    type Target;
}
impl<'x, 'ub, T: ?Sized> ViewIn<'x, 'ub> for T
where
    T: View<'x>,
{
    type Target = Self::Output;
}

/// Stores an owner and a derived shared reference into it.
///
/// `P` is the owner container (e.g. [`Rc<String>`][std::rc::Rc]).
/// It must implement the [`Aliasable`] trait to ensure that
/// it does not mutate the owned value when the derived view is alive.
///
/// `F` is the higher-kinded view marker type that describes the signature of the derivation function.
/// You can write `F` like `dyn for<'x> View<'x, Output = Type<'x>>` to indicate that the view type is `Type<'x>`,
/// or you can manually implement the [`View`] trait.
/// Note that it is entirely different type for different `F`,
/// even if they produce the same output type.
/// So it is recommended to use a unified view type in the same project,
/// though you can fall back on converting between them with [`Self::cast_view`].
///
/// The `'ub` parameter is a placeholder lifetime that can be deduced automatically.
/// If you need to specify it explicitly,
/// perfer choosing the longest possible lifetime satisfying `T::Target: 'ub`
/// to minimize the number of distinct types.
/// For owned heap containers, this is usually `'static`.
/// You could consider lowering it afterwards with [`Self::cast_life`]
/// if you need to put a shorter lifetime in the view,
/// which makes it somewhat easier to satisfy the invariants held by [`Bowl`].
pub struct Bowl<'ub, P, F: View<'ub> + ?Sized> {
    // `owner` will be dropped after `view`.
    // Rust guarantees that fields are dropped in the order of declaration.
    // https://doc.rust-lang.org/reference/destructors.html#r-destructors.operation
    //
    // `view: MaybeDangling<_>` suppresses Tree Borrows reference protection.
    // When `Bowl` is passed by value to a function,
    // Tree Borrows would normally "protect" any references inside it for the entire call duration,
    // asserting that the memory they point to remains valid.
    // But `Bowl` owns both `view` (the borrow) and `owner` (the allocation),
    // so dropping `Bowl` inside the callee frees `owner` while `view` is still considered live.
    // `MaybeDangling` opts out of the `dereferenceable` assumption, suppressing the protector.
    //
    // `owner: Aliasable` suppresses Stacked Borrows Unique retag.
    // Box-like types assert Unique ownership over their allocation whenever they are moved
    // (e.g., as a function argument).
    // If `owner` were moved after `view` was computed,
    // the resulting Unique retag would invalidate `view`'s SharedReadWrite tag on the same allocation.
    view: MaybeDangling<F::Output>,
    owner: Handle<'ub, 'ub, P>,
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> View<'x, Output = &'x P::Target>>
where
    P: Aliasable + Deref,
    P::Target: 'ub,
{
    pub fn new(owner: P) -> Self {
        let view = unsafe { transmute::<&P::Target, &'ub P::Target>(&*owner) };
        Bowl {
            view: MaybeDangling::new(view),
            owner: Handle(owner, PhantomData),
        }
    }
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> View<'x, Output = &'x mut P::Target>>
where
    P: Aliasable + DerefMut,
    P::Target: 'ub,
{
    pub fn new_mut(mut owner: P) -> Self {
        let view = unsafe { transmute::<&mut P::Target, &'ub mut P::Target>(&mut *owner) };
        Bowl {
            view: MaybeDangling::new(view),
            owner: Handle(owner, PhantomData),
        }
    }
}

#[derive(Copy)]
pub struct Isomorphic<'ub, F: View<'ub> + ?Sized>(F::Output);

impl<'ub, F> Clone for Isomorphic<'ub, F>
where
    F: View<'ub> + ?Sized,
    F::Output: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
{
    pub fn borrow<'a>(
        &'a self,
    ) -> Isomorphic<
        'ub,
        dyn for<'x> View<'x, Output = (&'a <F as ViewIn<'x, 'ub>>::Target, &'a Handle<'x, 'ub, P>)>
            + 'static,
    > {
        Isomorphic((&*self.view, &self.owner))
    }

    pub fn borrow_mut<'a>(
        &'a mut self,
    ) -> Isomorphic<
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a mut <F as ViewIn<'x, 'ub>>::Target,
                    &'a Handle<'x, 'ub, P>,
                ),
            > + 'static,
    > {
        Isomorphic((&mut *self.view, &self.owner))
    }
}

pub struct IsoStamp<'life, 'ub>(PhantomData<(&'life (), &'ub ())>);

impl<'a, 'ub, F> Isomorphic<'ub, F>
where
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
{
    pub fn map<R>(
        self,
        f: impl for<'x> FnOnce(<F as ViewIn<'x, 'ub>>::Target, IsoStamp<'x, 'ub>) -> R,
    ) -> R {
        f(self.0, IsoStamp(PhantomData))
    }
}

impl<'life, 'ub> IsoStamp<'life, 'ub> {
    pub fn stamp<'long, F>(&self, view: <F as ViewIn<'life, 'long>>::Target) -> Isomorphic<'ub, F>
    where
        F: ?Sized + for<'x> ViewIn<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view = unsafe {
            transmute::<<F as ViewIn<'life, 'long>>::Target, <F as View<'ub>>::Output>(view)
        };
        Isomorphic(view)
    }
}

pub struct Stamp<'brand, 'life, 'ub>(PhantomData<(&'brand (), &'life (), &'ub ())>);

impl<'brand, 'life, 'ub> Stamp<'brand, 'life, 'ub> {
    pub fn stamp<'long, F>(&self, view: <F as View<'life>>::Output) -> Derived<'brand, 'ub, F>
    where
        F: ?Sized + for<'x> ViewIn<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view =
            unsafe { transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view) };
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
        F: ?Sized + View<'ub>,
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

pub struct Handle<'life, 'ub, P>(P, PhantomData<(&'life (), &'ub ())>);

impl<'life, 'ub, P> Clone for Handle<'life, 'ub, P>
where
    P: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<'life, 'ub, P> Handle<'life, 'ub, P> {
    pub fn into_inner(self) -> P {
        self.0
    }

    pub fn fill<'long, F>(self, view: <F as View<'life>>::Output) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> ViewIn<'x, 'long>,
        'long: 'ub + 'life,
    {
        Slot(self.0, PhantomData).fill(Stamp(PhantomData).stamp(view))
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
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
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
