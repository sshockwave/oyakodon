use crate::primitive::{
    Aliasable, BoundedView, Bowl, CloneStableDeref, Exists, Owned, OwnerRef, Slot, Stamp, View,
};
use ::core::{clone::Clone, fmt, marker::Copy, mem::drop, ops::DerefMut};

pub trait Derive<T> {
    type Output;
    fn call(self, input: T) -> Self::Output;
}

impl<T, F, R> Derive<T> for F
where
    F: FnOnce(T) -> R,
{
    type Output = R;
    fn call(self, input: T) -> Self::Output {
        self(input)
    }
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> View<'x, Output = &'x P::Target>>
where
    P: Aliasable,
    P::Target: 'ub,
{
    pub fn new_ref(owner: P) -> Self {
        Bowl::new(owner).map(|view, slot, stamp| {
            let view = slot.spawn_ref(view);
            slot.fill(view, stamp)
        })
    }
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> View<'x, Output = &'x mut P::Target>>
where
    P: Aliasable + DerefMut,
    P::Target: 'ub,
{
    pub fn new_mut(owner: P) -> Self {
        Bowl::new(owner).map(|view, mut slot, stamp| {
            let view = slot.spawn_mut(view);
            slot.fill(view, stamp)
        })
    }
}

#[cfg(feature = "alloc")]
impl<'ub, T: 'ub>
    Bowl<
        'ub,
        ::aliasable::boxed::AliasableBox<T>,
        dyn for<'x> View<'x, Output = &'x mut T> + 'static,
    >
{
    pub fn new_box(owner: T) -> Self {
        Bowl::new_mut(::aliasable::boxed::AliasableBox::from_unique(
            ::alloc::boxed::Box::new(owner),
        ))
    }
}

#[cfg(feature = "alloc")]
impl<'ub, T: 'ub, F> Bowl<'ub, ::aliasable::boxed::AliasableBox<T>, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn into_owner_value(self) -> T {
        *::aliasable::boxed::AliasableBox::into_unique(self.into_owner())
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    pub fn with<'a, R>(
        &'a self,
        f: impl for<'life> FnOnce(
            &'a <F as View<'life>>::Output,
            &'a Slot<'life, Owned<P>>,
            Stamp<'life, 'ub>,
        ) -> R,
    ) -> R {
        self.borrow()
            .map(|(view, slot), stamp| f(view, slot, stamp))
    }

    pub fn with_mut<'a, R>(
        &'a mut self,
        f: impl for<'life> FnOnce(
            &'a mut <F as View<'life>>::Output,
            &'a Slot<'life, Owned<P>>,
            Stamp<'life, 'ub>,
        ) -> R,
    ) -> R {
        self.borrow_mut()
            .map(|(view, slot), stamp| f(view, slot, stamp))
    }

    /// Transforms the current view using `f`, encoding the composition as a generated view type.
    pub fn map_view<G>(
        self,
        f: G,
    ) -> Bowl<
        'ub,
        P,
        dyn for<'x> View<'x, Output = <G as Derive<<F as BoundedView<'x, 'ub>>::Target>>::Output>
            + 'static,
    >
    where
        G: for<'x> Derive<<F as BoundedView<'x, 'ub>>::Target>,
    {
        self.map(|view, slot, stamp| slot.fill(f.call(view), stamp))
    }

    /// Changes the view marker type `F` to any `G` that produces identical output types.
    /// It is recommended to use `dyn for<'x> View<'x, Output = Type<'x>>` as the view type indicator (i.e. HKT),
    /// or a unified view type in the same project.
    pub fn cast_view<G: ?Sized + for<'x> BoundedView<'x, 'ub, Target = <F as View<'x>>::Output>>(
        self,
    ) -> Bowl<'ub, P, G> {
        self.map(|view, slot, stamp| slot.fill(view, stamp))
    }

    /// Drops the owner and returns the view.
    ///
    /// The bound of this function requires the view cannot borrow from `*owner`.
    /// When the view does borrow from the owner, use [`Self::with`] instead.
    pub fn into_view<S>(self) -> S
    where
        for<'x> F: BoundedView<'x, 'ub, Target = S>,
    {
        self.map(|view, _, _| view)
    }

    /// Drops the view and returns the owner.
    pub fn into_owner(self) -> P {
        self.map(|view, slot, _| {
            // `slot` must be dropped even if `view`'s drop panics.
            // Miri reports that this is not guaranteed
            // if `view` is dropped implicitly at the end of the function,
            // because the `slot` is in a transition state
            // where it is still valid but not fully owned by the caller.
            // Users can do this leak manually,
            // but this is not a concern,
            // because Rust consider memory leaks as a safe behavior.
            // They can reproduce this leak with safe code:
            // ```rust
            // struct PanicOnDrop;
            // impl Drop for PanicOnDrop {
            //     fn drop(&mut self) {
            //         panic!("Drop panicked!");
            //     }
            // }
            // struct LeakMe(Box<i64>);
            // fn leak_generator() -> LeakMe {
            //     let a = LeakMe(Box::new(1));
            //     let b = PanicOnDrop;
            //     // Implicitly:
            //     // 1. a_local is moved to the return slot
            //     // 2. b_local is dropped -> PANIC
            //     // 3. a_local is leaked because the return never completes
            //     a
            // }
            // fn main() {
            //     // We catch the panic so the program doesn't abort,
            //     // allowing Miri to show us the leak.
            //     let _ = std::panic::catch_unwind(|| {
            //         leak_generator();
            //     });
            // }
            // ```
            drop(view);
            slot.into_owner()
        })
    }

    /// Returns both the owner and the view as a tuple.
    ///
    /// Carries the same `for<'x>` constraint as [`into_view`][Self::into_view]:
    /// the view type must be lifetime-independent of the owner.
    pub fn into_parts<S>(self) -> (P, S)
    where
        for<'x> F: BoundedView<'x, 'ub, Target = S>,
    {
        self.map(|view, slot, _| (slot.into_owner(), view))
    }

    /// Unwraps a [`Result`][::core::result::Result] view, branching into `Ok` or `Err`.
    /// Both branches retain the owner.
    pub fn into_result(
        self,
    ) -> ::core::result::Result<
        Bowl<'ub, P, dyn for<'x> View<'x, Output = <<F as View<'x>>::Output as Result>::Ok>>,
        Bowl<'ub, P, dyn for<'x> View<'x, Output = <<F as View<'x>>::Output as Result>::Err>>,
    >
    where
        for<'x> <F as View<'x>>::Output: Result,
    {
        self.map(|view, slot, stamp| match Result::into(view) {
            Ok(ok) => Ok(slot.fill(ok, stamp)),
            Err(err) => Err(slot.fill(err, stamp)),
        })
    }
}

mod with_new_bounded_view {
    use super::*;
    crate::bounded_view!(
        pub trait BoundedView {}
    );

    impl<'ub, P, F> Bowl<'ub, P, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'ub>,
    {
        /// Changes the lifetime placeholder `'ub` without modifying the value.
        /// This will reduce the requirements of operations on the view,
        /// at the cost of shortening the maximum possible lifetime of the view.
        pub fn cast_life<'short>(self) -> Bowl<'short, P, F>
        where
            'ub: 'short,
        {
            self.map(|view, slot, stamp| slot.fill(view, stamp.cast::<()>()))
        }

        /// Combines [`Self::cast_life`] and [`Self::cast_view`].
        pub fn cast<
            'short,
            G: ?Sized + for<'x> BoundedView<'x, 'ub, Target = <F as View<'x>>::Output>,
        >(
            self,
        ) -> Bowl<'short, P, G>
        where
            'ub: 'short,
        {
            self.cast_view().cast_life()
        }
    }
}

pub trait Result {
    type Ok;
    type Err;
    fn into(self) -> ::core::result::Result<Self::Ok, Self::Err>;
}

impl<T, E> Result for ::core::result::Result<T, E> {
    type Ok = T;
    type Err = E;
    fn into(self) -> ::core::result::Result<T, E> {
        self
    }
}

impl<'ub, T, F> Clone for Bowl<'ub, T, F>
where
    T: crate::primitive::CloneStableDeref,
    F: for<'x> BoundedView<'x, 'ub> + ?Sized,
    for<'x> <F as BoundedView<'x, 'ub>>::Target: Clone,
{
    fn clone(&self) -> Self {
        self.with(|view, slot, stamp| slot.clone().fill(view.clone(), stamp))
    }
}

impl<'ub, T, F> fmt::Debug for Bowl<'ub, T, F>
where
    F: for<'x> BoundedView<'x, 'ub> + ?Sized,
    for<'x> <F as BoundedView<'x, 'ub>>::Target: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg_struct = f.debug_struct("Bowl");
        self.with(|view, _, _| {
            dbg_struct.field("view", view);
        });
        dbg_struct.finish_non_exhaustive()
    }
}

impl<'ub, F> Default for Exists<'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
    for<'x> <F as BoundedView<'x, 'ub>>::Target: Default,
{
    fn default() -> Self {
        Exists::new().map(|(), stamp| stamp.stamp(Default::default()))
    }
}

impl<'ub, F> Copy for Exists<'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
    for<'x> <F as BoundedView<'x, 'ub>>::Target: Copy,
{
}

impl<'ub, F> Clone for Exists<'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
    for<'x> <F as BoundedView<'x, 'ub>>::Target: Clone,
{
    fn clone(&self) -> Self {
        self.borrow(|view, stamp| stamp.stamp(view.clone()))
    }
}

impl<'ub, F> Exists<'ub, F>
where
    F: ?Sized + for<'x> BoundedView<'x, 'ub>,
{
    // TODO: better name
    pub fn cast<'short>(self) -> Exists<'short, F>
    where
        'ub: 'short,
    {
        self.map(|view, stamp| stamp.stamp(view))
    }
}

impl<P: CloneStableDeref> Clone for Slot<'_, Owned<P>> {
    fn clone(&self) -> Self {
        self.spawn()
    }
}

impl Clone for OwnerRef<'_, false> {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for OwnerRef<'_, false> {}
