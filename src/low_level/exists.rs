use crate::{
    polyfill::{transmute_since_1_66, PhantomInvariantLifetime},
    primitive::{BoundedView, View},
};
use ::core::marker::PhantomData;

/// Maintains an invariant that
/// the view is valid for at least one of the lifetimes `'x` shorter than `'ub`.
/// When [`F::Output`] contains implicit lifetime bounds like `&'short &'x &'long ()`,
/// the invariant only requires `'x` to make the expression well-formed,
/// i.e. `'x` longer than `'short` and shorter than `'long`.
///
/// [`F::Output`]: View::Output
pub struct Exists<'ub, F: View<'ub> + ?Sized>(F::Output);

impl Exists<'_, dyn for<'x> View<'x, Output = ()>> {
    pub const fn new() -> Self {
        Exists(())
    }
}

impl<'ub, F> Exists<'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn borrow<'a, R>(
        &'a self,
        f: impl for<'x> FnOnce(&'a <F as View<'x>>::Output, Stamp<'x, 'ub>) -> R,
    ) -> R {
        f(&self.0, Stamp(PhantomData))
    }

    pub fn borrow_mut<'a, R>(
        &'a mut self,
        f: impl for<'x> FnOnce(&'a mut <F as View<'x>>::Output, Stamp<'x, 'ub>) -> R,
    ) -> R {
        f(&mut self.0, Stamp(PhantomData))
    }

    pub fn map<R>(self, f: impl for<'x> FnOnce(<F as View<'x>>::Output, Stamp<'x, 'ub>) -> R) -> R {
        f(self.0, Stamp(PhantomData))
    }
}

#[derive(Clone, Copy)]
pub struct Stamp<'life, 'ub, X: ?Sized = &'life &'ub ()>(
    PhantomData<(PhantomInvariantLifetime<'life>, &'ub (), X)>,
);

impl<'life, 'ub, X: ?Sized> Stamp<'life, 'ub, X> {
    pub const fn cast<Y: ?Sized>(&self) -> Stamp<'life, 'ub, Y> {
        Stamp(PhantomData)
    }

    /// For [`stamp`] to be sound,
    /// it shall allow relaxing the lifetime range to a wider one
    /// but must not allow shrinking the lifetime range in any way.
    /// In logical terms, for example, `x == 5` implies `0 <= x < 10`,
    /// `2 <= x < 8` implies `0 <= x < 10`,
    /// and `0 <= x < 10` does not necessarily imply `2 <= x < 8`.
    ///
    /// [`stamp`] could have been used to shrink the lifetime range due to [#84591]:
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
    /// and also cannot decrease the upper bound
    /// because [`Stamp`] is invariant over `'life`:
    /// ```compile_fail
    /// use oyakodon::primitive::{Stamp, View};
    /// fn get_stamp<'long, 'life, 'ub>(stamp: Stamp<'life, 'ub>) {
    ///     stamp.stamp::<dyn for<'short> View<'short, Output = &'short &'long ()>>(&&());
    /// }
    /// ```
    /// Thus [`stamp`] does not have soundness issues currently,
    /// but it might change in future Rust versions.
    ///
    /// [`stamp`]: Self::stamp
    /// [#84591]: https://github.com/rust-lang/rust/issues/84591
    pub fn stamp<'long, F>(&self, view: <F as View<'life>>::Output) -> Exists<'ub, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view = unsafe {
            transmute_since_1_66::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view)
        };
        Exists(view)
    }
}
