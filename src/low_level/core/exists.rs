use crate::primitive::{BoundedView, View};
use ::core::{marker::PhantomData, mem::transmute};

/// Maintains an invariant that
/// the view is valid for at least one of the lifetimes `'x` shorter than `'ub`.
/// If the view contain a lower bound to `'x`,
/// e.g. `View<'x, Output = &'a &'x ()>`,
/// the invariant only needs to hold for `'x` that makes the expression well-formed.
pub struct Exists<'ub, F: View<'ub> + ?Sized>(F::Output);

impl Exists<'_, dyn for<'x> View<'x, Output = ()>> {
    pub fn new() -> Self {
        Exists(())
    }
}

impl<'ub, F> Exists<'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn borrow<'a, R>(
        &'a self,
        f: impl for<'x> FnOnce(&'a <F as BoundedView<'x, 'ub>>::Target, Stamp<'x, 'ub>) -> R,
    ) -> R {
        f(&self.0, Stamp(PhantomData))
    }

    pub fn borrow_mut<'a, R>(
        &'a mut self,
        f: impl for<'x> FnOnce(&'a mut <F as BoundedView<'x, 'ub>>::Target, Stamp<'x, 'ub>) -> R,
    ) -> R {
        f(&mut self.0, Stamp(PhantomData))
    }

    pub fn map<R>(
        self,
        f: impl for<'x> FnOnce(<F as BoundedView<'x, 'ub>>::Target, Stamp<'x, 'ub>) -> R,
    ) -> R {
        f(self.0, Stamp(PhantomData))
    }
}

#[derive(Clone, Copy)]
pub struct Stamp<'life, 'ub, X: ?Sized = &'life &'ub ()>(PhantomData<(&'life (), &'ub (), X)>);

impl<'life, 'ub, X: ?Sized> Stamp<'life, 'ub, X> {
    pub fn cast<Y: ?Sized>(&self) -> Stamp<'life, 'ub, Y> {
        Stamp(PhantomData)
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
    /// fn get_stamp<'short, 'life, 'ub>(stamp: Stamp<'life, 'ub>) {
    ///     stamp.stamp::<dyn for<'long> View<'long, Output = &'short &'long ()>>(&&());
    /// }
    /// ```
    ///
    /// [`stamp`]: Self::stamp
    /// [#84591]: https://github.com/rust-lang/rust/issues/84591
    pub fn stamp<'long, F>(&self, view: <F as View<'life>>::Output) -> Exists<'ub, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view =
            unsafe { transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view) };
        Exists(view)
    }
}
