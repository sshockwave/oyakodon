use super::{Aliasable, BoundedView, CloneStableDeref, ForAll, View};
use ::{
    core::{
        clone::Clone,
        marker::PhantomData,
        mem::transmute,
        ops::{Deref, DerefMut},
    },
    maybe_dangling::MaybeDangling,
};

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
    owner: Anchor<'ub, 'ub, P>,
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
            owner: Anchor(owner, PhantomData),
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
            owner: Anchor(owner, PhantomData),
        }
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn borrow<'a>(
        &'a self,
    ) -> ForAll<
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a <F as BoundedView<'x, 'ub>>::Target,
                    &'a Anchor<'x, 'ub, P>,
                ),
            > + 'static,
    > {
        let result = ForAll::new().map(|(), stamp| stamp.stamp((&*self.view, &self.owner)));
        // SAFETY: This function is for backwards compatibility and will be removed in the future.
        unsafe {
            transmute::<
                ForAll<
                    'ub,
                    dyn for<'x> View<
                        'x,
                        Output = (&'a <F as View<'ub>>::Output, &'a Anchor<'ub, 'ub, P>),
                    >,
                >,
                _,
            >(result)
        }
    }

    pub fn borrow_mut<'a>(
        &'a mut self,
    ) -> ForAll<
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a mut <F as BoundedView<'x, 'ub>>::Target,
                    &'a Anchor<'x, 'ub, P>,
                ),
            > + 'static,
    > {
        let result = ForAll::new().map(|(), stamp| stamp.stamp((&*self.view, &mut self.owner)));
        // SAFETY: This function is for backwards compatibility and will be removed in the future.
        unsafe {
            transmute::<
                ForAll<
                    'ub,
                    dyn for<'x> View<
                        'x,
                        Output = (&'a <F as View<'ub>>::Output, &'a mut Anchor<'ub, 'ub, P>),
                    >,
                >,
                _,
            >(result)
        }
    }
}

/// [`stamp`] could have been unsound due to [#84591]:
/// ```
/// use oyakodon::primitive::View;
/// fn requires_all<F: ?Sized + for<'x> View<'x>>() {}
/// fn get_lifetime<'lower_bound>() {
///     requires_all::<dyn for<'x> View<'x, Output = &'static &'x ()>>();
/// }
/// ```
/// But curiously, Rust is able to reject this case:
/// ```compile_fail
/// use oyakodon::primitive::View;
/// pub struct Requires<'life>(&'life ());
/// impl<'life> Requires<'life> {
///     pub fn all<F: ?Sized + View<'life>>(_: F::Output) {}
/// }
/// fn get_lifetime<'lower_bound, 'life, 'ub>() {
///     Requires::<'life>::all::<dyn for<'x> View<'x, Output = &'lower_bound &'x ()>>(&&());
/// }
/// ```
/// So [`stamp`] cannot be exploited to raise the lower bound of `'life`:
/// ```compile_fail
/// use oyakodon::primitive::{Stamp, View};
/// fn get_stamp<'short, 'brand, 'life, 'ub>(stamp: Stamp<'brand, 'life, 'ub>) {
///     stamp.stamp::<dyn for<'long> View<'long, Output = &'short &'long ()>>(&&());
/// }
/// ```
///
/// [`stamp`]: Self::stamp
/// [#84591]: https://github.com/rust-lang/rust/issues/84591
pub struct Stamp<'brand, 'life, 'ub>(PhantomData<(&'brand (), &'life (), &'ub ())>);

impl<'brand, 'life, 'ub> Stamp<'brand, 'life, 'ub> {
    pub fn stamp<'long, F>(
        &self,
        view: <F as View<'life>>::Output,
    ) -> ProtectedForAll<'brand, 'ub, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view =
            unsafe { transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view) };
        unsafe { ProtectedForAll::new_unchecked(view) }
    }
}

pub struct ProtectedForAll<'brand, 'ub, F: View<'ub> + ?Sized>(
    MaybeDangling<F::Output>,
    PhantomData<(&'brand (), F)>,
);

impl<'brand, 'ub, F> ProtectedForAll<'brand, 'ub, F>
where
    F: View<'ub> + ?Sized,
{
    pub unsafe fn new_unchecked(view: F::Output) -> Self {
        Self(MaybeDangling::new(view), PhantomData)
    }

    /// # Safety
    /// The corresponding owner must exist for the entire lifetime of the returned `P`
    /// until it is wrapped back into a `ProtectedForAll` again.
    unsafe fn into_inner(self) -> F::Output {
        MaybeDangling::into_inner(self.0)
    }
}

pub struct Slot<'brand, P>(P, PhantomData<&'brand ()>);

impl<'brand, P> Slot<'brand, P> {
    pub fn fill<'ub, F>(self, view: ProtectedForAll<'brand, 'ub, F>) -> Bowl<'ub, P, F>
    where
        F: ?Sized + View<'ub>,
    {
        Bowl {
            view: view.0,
            owner: Anchor(self.0, PhantomData),
        }
    }

    pub fn into_inner(self) -> P {
        self.0
    }
}

impl<'brand, 'ub, P> Clone for Slot<'brand, P>
where
    P: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

pub struct Anchor<'life, 'ub, P>(P, PhantomData<(&'life (), &'ub ())>);

impl<'life, 'ub, P> Clone for Anchor<'life, 'ub, P>
where
    P: CloneStableDeref,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<'life, 'ub, P> Anchor<'life, 'ub, P> {
    pub fn into_inner(self) -> P {
        self.0
    }

    pub fn bind<'long, F>(self, view: <F as View<'life>>::Output) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        Slot(self.0, PhantomData).fill(Stamp(PhantomData).stamp(view))
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + View<'ub>,
{
    pub fn map<R>(
        self,
        f: impl for<'bowl> FnOnce(ProtectedForAll<'bowl, 'ub, F>, ProtectedSlot<'bowl, P>) -> R,
    ) -> R {
        let view = MaybeDangling::into_inner(self.view);
        let view = unsafe { ProtectedForAll::new_unchecked(view) };
        f(view, ProtectedSlot(self.owner.0, PhantomData))
    }
}

pub struct ProtectedSlot<'bowl, P>(P, PhantomData<&'bowl ()>);

impl<'bowl, P> ProtectedSlot<'bowl, P> {
    pub fn unseal(self) -> Slot<'bowl, P> {
        Slot(self.0, PhantomData)
    }
}

impl<'bowl, 'ub, F> ProtectedForAll<'bowl, 'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn borrow<'a, P>(
        &'a self,
    ) -> ProtectedForAll<
        'bowl,
        'ub,
        dyn for<'x> View<'x, Output = &'a <F as BoundedView<'x, 'ub>>::Target> + 'static,
    > {
        unsafe { ProtectedForAll::new_unchecked(&*self.0) }
    }

    pub fn borrow_mut<'a, P>(
        &'a mut self,
    ) -> ProtectedForAll<
        'bowl,
        'ub,
        dyn for<'x> View<'x, Output = &'a mut <F as BoundedView<'x, 'ub>>::Target> + 'static,
    > {
        unsafe { ProtectedForAll::new_unchecked(&mut *self.0) }
    }

    pub fn zip<G>(
        self,
        other: ProtectedForAll<'bowl, 'ub, G>,
    ) -> ProtectedForAll<
        'bowl,
        'ub,
        dyn for<'x> View<
            'x,
            Output = (
                <F as BoundedView<'x, 'ub>>::Target,
                <G as BoundedView<'x, 'ub>>::Target,
            ),
        >,
    >
    where
        G: ?Sized + for<'x> BoundedView<'x, 'ub>,
    {
        unsafe { ProtectedForAll::new_unchecked((self.into_inner(), other.into_inner())) }
    }

    pub fn map<R, P>(
        self,
        _token: &ProtectedSlot<'bowl, P>,
        f: impl for<'x> FnOnce(<F as BoundedView<'x, 'ub>>::Target, Stamp<'bowl, 'x, 'ub>) -> R,
    ) -> R {
        f(unsafe { self.into_inner() }, Stamp(PhantomData))
    }
}
