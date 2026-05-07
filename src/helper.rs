use crate::primitive::{Anchor, Bowl, ForAll, View};
use ::core::{clone::Clone, fmt, marker::Copy, mem::drop};

pub trait ViewIn<'x, 'ub, X = &'x &'ub ()>: View<'x, Output = Self::Target> {
    type Target;
}
impl<'x, 'ub, T: ?Sized> ViewIn<'x, 'ub> for T
where
    T: View<'x>,
{
    type Target = Self::Output;
}

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
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
{
    pub fn into_owner_value(self) -> T {
        *::aliasable::boxed::AliasableBox::into_unique(self.into_owner())
    }
}

impl<'ub, P, F> Bowl<'ub, P, F>
where
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
{
    pub fn with<'a, R>(
        &'a self,
        f: impl for<'life> FnOnce(&'a <F as ViewIn<'life, 'ub>>::Target, &'a Anchor<'life, 'ub, P>) -> R,
    ) -> R {
        self.borrow().map(|(view, anchor), _| f(view, anchor))
    }

    pub fn with_mut<'a, R>(
        &'a mut self,
        f: impl for<'life> FnOnce(
            &'a mut <F as ViewIn<'life, 'ub>>::Target,
            &'a Anchor<'life, 'ub, P>,
        ) -> R,
    ) -> R {
        self.borrow_mut().map(|(view, anchor), _| f(view, anchor))
    }

    /// Transforms the current view using `f`, encoding the composition as a generated view type.
    pub fn map_view<G>(
        self,
        f: G,
    ) -> Bowl<
        'ub,
        P,
        dyn for<'x> View<'x, Output = <G as Derive<<F as ViewIn<'x, 'ub>>::Target>>::Output>
            + 'static,
    >
    where
        G: for<'x> Derive<<F as ViewIn<'x, 'ub>>::Target>,
    {
        self.map(|view, slot| {
            let view = view.map(&slot, |view, stamp| stamp.stamp(f.call(view)));
            slot.unseal().fill(view)
        })
    }

    /// Changes the lifetime placeholder `'ub` without modifying the value.
    /// This will reduce the requirements of operations on the view,
    /// at the cost of shortening the maximum possible lifetime of the view.
    pub fn cast_life<'short>(self) -> Bowl<'short, P, F>
    where
        'ub: 'short,
    {
        self.map(|view, slot| {
            let view = view.map(&slot, |view, stamp| stamp.stamp(view));
            slot.unseal().fill(view)
        })
    }

    /// Changes the view marker type `F` to any `G` that produces identical output types.
    /// It is recommended to use `dyn for<'x> View<'x, Output = Type<'x>>` as the view type indicator (i.e. HKT),
    /// or a unified view type in the same project.
    pub fn cast_view<G: ?Sized + for<'x> ViewIn<'x, 'ub, Target = <F as View<'x>>::Output>>(
        self,
    ) -> Bowl<'ub, P, G> {
        self.map(|view, slot| {
            let view = view.map(&slot, |view, stamp| stamp.stamp(view));
            slot.unseal().fill(view)
        })
    }

    /// Combines [`Self::cast_life`] and [`Self::cast_view`].
    pub fn cast<'short, G: ?Sized + for<'x> ViewIn<'x, 'ub, Target = <F as View<'x>>::Output>>(
        self,
    ) -> Bowl<'short, P, G>
    where
        'ub: 'short,
    {
        self.cast_view().cast_life()
    }

    /// Drops the view and returns the owner.
    pub fn into_owner(self) -> P {
        self.map(|view, slot| {
            drop(view); // Same as `into_view`
            slot.unseal().into_inner()
        })
    }

    /// Drops the owner and returns the view.
    ///
    /// The bound of this function requires the view cannot borrow from `*owner`.
    /// When the view does borrow from the owner, use [`Self::with`] instead.
    pub fn into_view<S>(self) -> S
    where
        for<'x> F: ViewIn<'x, 'ub, Target = S>,
    {
        self.map(|view, slot| {
            let view = view.map(&slot, |view, _| view);
            // `view` must be dropped even if `owner`'s drop panics.
            // Miri reports that this is not guaranteed
            // if `owner` is dropped implicitly at the end of the function,
            // because the `view` is in a transition state
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
            drop(slot);
            view
        })
    }

    /// Returns both the owner and the view as a tuple.
    ///
    /// Carries the same `for<'x>` constraint as [`into_view`][Self::into_view]:
    /// the view type must be lifetime-independent of the owner.
    pub fn into_parts<S>(self) -> (P, S)
    where
        for<'x> F: ViewIn<'x, 'ub, Target = S>,
    {
        self.map(|view, slot| {
            let view = view.map(&slot, |view, _| view);
            (slot.unseal().into_inner(), view)
        })
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
        self.map(|view, slot| {
            let view = view.map(&slot, |view, stamp| match Result::into(view) {
                Ok(ok) => Ok(stamp.stamp(ok)),
                Err(err) => Err(stamp.stamp(err)),
            });
            let slot = slot.unseal();
            match view {
                Ok(ok) => Ok(slot.fill(ok)),
                Err(err) => Err(slot.fill(err)),
            }
        })
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
    T: super::primitive::CloneStableDeref,
    F: for<'x> ViewIn<'x, 'ub> + ?Sized,
    for<'x> <F as ViewIn<'x, 'ub>>::Target: Clone,
{
    fn clone(&self) -> Self {
        self.with(|view, anchor| anchor.clone().bind(view.clone()))
    }
}

impl<'ub, T, F> fmt::Debug for Bowl<'ub, T, F>
where
    F: for<'x> ViewIn<'x, 'ub> + ?Sized,
    for<'x> <F as ViewIn<'x, 'ub>>::Target: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg_struct = f.debug_struct("Bowl");
        self.with(|view, _| {
            dbg_struct.field("view", view);
        });
        dbg_struct.finish_non_exhaustive()
    }
}

impl<'ub, F> Default for ForAll<'ub, F>
where
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
    for<'x> <F as ViewIn<'x, 'ub>>::Target: Default,
{
    fn default() -> Self {
        // There should be a problem when converting `'static` to `'ub`,
        // because we required `'ub: 'life` in [`Stamp::stamp`],
        // but it does work currently and I don't understand why.
        ForAll::new().map(|(), stamp| stamp.stamp(Default::default()))
    }
}

impl<'ub, F> Copy for ForAll<'ub, F>
where
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
    for<'x> <F as ViewIn<'x, 'ub>>::Target: Copy,
{
}

impl<'ub, F> Clone for ForAll<'ub, F>
where
    F: ?Sized + for<'x> ViewIn<'x, 'ub>,
    for<'x> <F as ViewIn<'x, 'ub>>::Target: Clone,
{
    fn clone(&self) -> Self {
        self.borrow(|view, stamp| stamp.stamp(view.clone()))
    }
}
