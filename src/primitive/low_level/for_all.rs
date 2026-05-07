use super::{BoundedView, View};
use ::core::{marker::PhantomData, mem::transmute};

/// Maintains an invariant that
/// the view is valid for at least one of the lifetimes `'x` shorter than `'ub`.
/// If the view contain a lower bound to `'x`,
/// e.g. `View<'x, Output = &'a &'x ()>`,
/// the invariant only needs to hold for `'x` that makes the expression well-formed.
pub struct ForAll<'ub, F: View<'ub> + ?Sized>(F::Output);

impl ForAll<'static, dyn for<'x> View<'x, Output = ()>> {
    pub fn new() -> Self {
        ForAll(())
    }
}

impl<'ub, F> ForAll<'ub, F>
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

pub struct Stamp<'life, 'ub>(PhantomData<(&'life (), &'ub ())>);

impl<'life, 'ub> Stamp<'life, 'ub> {
    pub fn stamp<'long, F>(&self, view: <F as View<'life>>::Output) -> ForAll<'ub, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        let view =
            unsafe { transmute::<<F as View<'life>>::Output, <F as View<'ub>>::Output>(view) };
        ForAll(view)
    }
}
