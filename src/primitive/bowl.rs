use super::{Aliasable, BoundedView, CloneStableDeref, ForAll, View};
use ::{
    core::{
        clone::Clone,
        marker::PhantomData,
        mem::{drop, replace, transmute},
        ops::{Deref, DerefMut},
        option::Option,
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
            owner: Anchor(PhantomData, owner),
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
            owner: Anchor(PhantomData, owner),
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
/// fn get_stamp<'short, 'life, 'ub, P>(stamp: Stamp<'life, 'ub, P>) {
///     stamp.stamp::<dyn for<'long> View<'long, Output = &'short &'long ()>>(&&());
/// }
/// ```
///
/// [`stamp`]: Self::stamp
/// [#84591]: https://github.com/rust-lang/rust/issues/84591
pub struct Stamp<'life, 'ub, P>(&'life mut Option<P>, PhantomData<(&'life (), &'ub ())>);

impl<'life, 'ub, P> Stamp<'life, 'ub, P> {
    pub fn stamp<'long, F>(self, view: <F as View<'life>>::Output) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view =
            unsafe { transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view) };
        let owner = replace(self.0, None);
        // SAFETY: Same as `Self::into_owner`.
        let owner = unsafe { owner.unwrap_unchecked() };
        Bowl {
            view: MaybeDangling::new(view),
            owner: Anchor(PhantomData, owner),
        }
    }

    pub fn into_owner(self) -> P {
        let owner = replace(self.0, None);
        // SAFETY: `self` is consumed,
        // so the `owner` must not have been moved out.
        // All references to `owner` are also dropped.
        unsafe { owner.unwrap_unchecked() }
    }
}

impl<'life, 'ub, P: CloneStableDeref> Stamp<'life, 'ub, P> {
    pub fn spawn(&self) -> Anchor<'life, 'ub, P> {
        // Verified that this will compile to unchecked dereference with `-O`.
        let owner = self.0.as_ref();
        // SAFETY: `slot` is guaranteed to be valid for its entire lifetime,
        // so the `owner` must not have been moved out.
        let owner = unsafe { owner.unwrap_unchecked() };
        Anchor(PhantomData, owner.clone())
    }
}

pub struct Anchor<'life, 'ub, P: ?Sized>(PhantomData<(&'life (), &'ub ())>, P);

impl<P: CloneStableDeref + ?Sized> Clone for Anchor<'_, '_, P> {
    fn clone(&self) -> Self {
        Self(PhantomData, self.1.clone())
    }
}

impl<'life, 'ub, P> Anchor<'life, 'ub, P> {
    pub fn into_inner(self) -> P {
        self.1
    }

    pub fn bind<'long, F>(self, view: <F as View<'life>>::Output) -> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view =
            unsafe { transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view) };
        Bowl {
            view: MaybeDangling::new(view),
            owner: Anchor(PhantomData, self.1),
        }
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    /// Internally this function uses [`Option`] to check
    /// whether the caller has dropped the owner during the call.
    /// This has some performance overhead,
    /// but the compiler should be able to optimize it to the same level as [drop flags].
    ///
    /// [drop flags]: https://doc.rust-lang.org/reference/destructors.html#drop-flags
    pub fn map<R>(
        self,
        f: impl for<'life> FnOnce(<F as BoundedView<'life, 'ub>>::Target, Stamp<'life, 'ub, P>) -> R,
    ) -> R {
        let view = MaybeDangling::into_inner(self.view);
        let view = unsafe { transmute::<<F as View<'ub>>::Output, <F as View<'_>>::Output>(view) };
        let mut owner = Some(self.owner.into_inner());
        let result = f(view, Stamp(&mut owner, PhantomData));
        drop(owner);
        result
    }
}
