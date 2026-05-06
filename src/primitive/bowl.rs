use super::{Aliasable, CloneStableDeref};
use ::{
    core::{
        clone::Clone,
        default::Default,
        marker::{Copy, PhantomData},
        mem::transmute,
        ops::{Deref, DerefMut},
    },
    maybe_dangling::MaybeDangling,
};

pub trait View<'x> {
    type Output;
}

pub trait BoundedView<'x, 'ub, X = &'x &'ub ()>: View<'x, Output = Self::Target> {
    type Target;
}
impl<'x, 'ub, T: ?Sized> BoundedView<'x, 'ub> for T
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

pub struct ForAll<'bowl, 'ub, F: View<'ub> + ?Sized>(F::Output, PhantomData<&'bowl ()>);

impl<'bowl, 'ub, F> ForAll<'bowl, 'ub, F>
where
    F: View<'ub> + ?Sized,
{
    /// # SAFETY
    /// The caller must ensure that the view is valid for all lifetimes `'x`
    /// that does not outlive `'ub`.
    /// If the view contain a lower bound to `'x`,
    /// e.g. `View<'x, Output = &'a &'x ()>`,
    /// the invariant only needs to hold for `'x` that makes the expression well-formed.
    pub unsafe fn new_unchecked(view: F::Output) -> Self {
        ForAll(view, PhantomData)
    }
}

impl<'bowl, 'ub, F> Default for ForAll<'bowl, 'ub, F>
where
    F: View<'ub> + ?Sized,
    F::Output: Default,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

impl<'bowl, 'ub, F> Copy for ForAll<'bowl, 'ub, F>
where
    F: View<'ub> + ?Sized,
    F::Output: Copy,
{
}

impl<'bowl, 'ub, F> Clone for ForAll<'bowl, 'ub, F>
where
    F: View<'ub> + ?Sized,
    F::Output: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn borrow<'a>(
        &'a self,
    ) -> ForAll<
        'a,
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a <F as BoundedView<'x, 'ub>>::Target,
                    &'a Anchor<'x, 'ub, P>,
                ),
            > + 'static,
    > {
        // We cannot borrow `self.view` and then add `self.owner` here
        // because the lower bound of `'x` from `self.view.borrow()` is not enough
        // to prove that `'x` outlives `'a`,
        // which is required by the return type signature.
        // SAFETY: The lifetime acts like a brand
        // that the `Anchor` will only be matched
        // with the view derived from the same `Bowl`.
        // It would be a violation of this invariant
        // if we add a `fn zip(ForAll<A>, ForAll<B>) -> ForAll<(A, B)>`.
        unsafe { ForAll::new_unchecked((&*self.view, &self.owner)) }
    }

    pub fn borrow_mut<'a>(
        &'a mut self,
    ) -> ForAll<
        'a,
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a mut <F as BoundedView<'x, 'ub>>::Target,
                    &'a Anchor<'x, 'ub, P>,
                ),
            > + 'static,
    > {
        // SAFETY: Same as `borrow`.
        unsafe { ForAll::new_unchecked((&mut *self.view, &self.owner)) }
    }
}

pub struct IsoStamp<'bowl, 'life, 'ub>(PhantomData<(&'bowl (), &'life (), &'ub (), fn(&'ub ()))>);

impl<'bowl, 'ub, F> ForAll<'bowl, 'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn borrow<'a>(
        &'a self,
    ) -> ForAll<
        'bowl,
        'ub,
        dyn for<'x> View<'x, Output = &'a <F as BoundedView<'x, 'ub>>::Target> + 'static,
    > {
        // SAFETY: The return type raises the lower bound of `'x`,
        // but `&'a self` implies `&'a F::Output`,
        // thus `'x` already satisfies the lower bound.
        unsafe { ForAll::new_unchecked(&self.0) }
    }

    pub fn borrow_mut<'a>(
        &'a mut self,
    ) -> ForAll<
        'bowl,
        'ub,
        dyn for<'x> View<'x, Output = &'a mut <F as BoundedView<'x, 'ub>>::Target> + 'static,
    > {
        // SAFETY: Same as `borrow`.
        unsafe { ForAll::new_unchecked(&mut self.0) }
    }

    pub fn map<R>(
        self,
        f: impl for<'x> FnOnce(<F as BoundedView<'x, 'ub>>::Target, IsoStamp<'bowl, 'x, 'ub>) -> R,
    ) -> R {
        f(self.0, IsoStamp(PhantomData))
    }
}

impl<'bowl, 'life, 'ub> IsoStamp<'bowl, 'life, 'ub> {
    pub fn stamp<'long, F>(
        &self,
        view: <F as BoundedView<'life, 'long>>::Target,
    ) -> ForAll<'bowl, 'ub, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view = unsafe {
            transmute::<<F as BoundedView<'life, 'long>>::Target, <F as View<'ub>>::Output>(view)
        };
        // SAFETY: Depends on `IsoStamp` always being used inside an function generic over the `'life`.
        unsafe { ForAll::new_unchecked(view) }
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
    pub fn stamp<'long, F>(&self, view: <F as View<'life>>::Output) -> Encased<'brand, 'ub, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view =
            unsafe { transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view) };
        Encased(view, PhantomData)
    }
}

#[derive(Clone, Copy)]
pub struct Encased<'brand, 'ub, F: View<'ub> + ?Sized>(
    F::Output,
    PhantomData<(&'brand (), &'ub (), F)>,
);

pub struct Slot<'brand, P>(P, PhantomData<&'brand ()>);

impl<'brand, P> Slot<'brand, P> {
    pub fn fill<'ub, F>(self, view: Encased<'brand, 'ub, F>) -> Bowl<'ub, P, F>
    where
        F: ?Sized + View<'ub>,
    {
        Bowl {
            view: MaybeDangling::new(view.0),
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
    pub fn map<R>(self, f: impl for<'brand> FnOnce(Scope<'brand, 'ub, P, F>) -> R) -> R {
        f(Scope(self, PhantomData))
    }
}

pub struct Scope<'brand, 'ub, P, F>(Bowl<'ub, P, F>, PhantomData<&'brand ()>)
where
    F: View<'ub> + ?Sized;

impl<'brand, 'ub, P, F> Scope<'brand, 'ub, P, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn open<R>(
        self,
        f: impl for<'life> FnOnce(<F as View<'life>>::Output, Stamp<'brand, 'life, 'ub>) -> R,
    ) -> (R, Slot<'brand, P>) {
        (
            f(MaybeDangling::into_inner(self.0.view), Stamp(PhantomData)),
            Slot(self.0.owner.0, PhantomData),
        )
    }
}
