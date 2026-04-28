#[cfg(feature = "alloc")]
use crate::primitive::MutView;
use crate::primitive::{Bowl, View};
use ::core::fmt;

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
impl<'ub, T: 'ub> Bowl<'ub, ::aliasable::boxed::AliasableBox<T>, MutView<T>> {
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
    /// Transforms the current view using `f`, encoding the composition as a generated view type.
    pub fn map_view<G>(
        self,
        f: G,
    ) -> Bowl<'ub, P, dyn for<'x> Fn(&'x ()) -> <G as Derive<<F as ViewIn<'x, 'ub>>::Target>>::Output>
    where
        G: for<'x> Derive<<F as ViewIn<'x, 'ub>>::Target>,
    {
        self.map(|session| {
            let (view, slot) = session.open(|view, stamp| stamp.stamp(f.call(view)));
            slot.fill(view)
        })
    }

    /// Changes the lifetime placeholder `'ub` without modifying the value.
    /// This will reduce the requirements of operations on the view,
    /// at the cost of shortening the maximum possible lifetime of the view.
    pub fn cast_life<'short>(self) -> Bowl<'short, P, F>
    where
        'ub: 'short,
    {
        self.map(|session| {
            let (view, slot) = session.open(|view, stamp| stamp.stamp(view));
            slot.fill(view)
        })
    }

    /// Changes the view marker type `F` to any `G` that produces identical output types.
    /// It is recommended to use `dyn for<'x> Fn(&'x ()) -> Type<'x>` as the view type indicator (i.e. HKT),
    /// or a unified view type in the same project.
    pub fn cast_view<G: ?Sized + for<'x> ViewIn<'x, 'ub, Target = <F as View<'x>>::Output>>(
        self,
    ) -> Bowl<'ub, P, G> {
        self.map(|session| {
            let (view, slot) = session.open(|view, stamp| stamp.stamp(view));
            slot.fill(view)
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
        self.map(|session| session.open(|_, _| ()).1.into_inner())
    }

    /// Drops the owner and returns the view.
    ///
    /// The bound of this function requires the view cannot borrow from `*owner`.
    /// When the view does borrow from the owner, use [`Self::with`] instead.
    pub fn into_view<S>(self) -> S
    where
        for<'x> F: ViewIn<'x, 'ub, Target = S>,
    {
        self.map(|session| session.open(|view, _| view).0)
    }

    /// Returns both the owner and the view as a tuple.
    ///
    /// Carries the same `for<'x>` constraint as [`into_view`][Self::into_view]:
    /// the view type must be lifetime-independent of the owner.
    pub fn into_parts<S>(self) -> (P, S)
    where
        for<'x> F: ViewIn<'x, 'ub, Target = S>,
    {
        self.map(|session| {
            let t = session.open(|view, _| view);
            (t.1.into_inner(), t.0)
        })
    }

    /// Unwraps an [`Outcome`] view, branching into `Ok` or `Err`.
    /// Both branches retain the owner.
    pub fn into_result(
        self,
    ) -> ::core::result::Result<
        Bowl<'ub, P, dyn for<'x> Fn(&'x ()) -> <<F as View<'x>>::Output as Result>::Ok>,
        Bowl<'ub, P, dyn for<'x> Fn(&'x ()) -> <<F as View<'x>>::Output as Result>::Err>,
    >
    where
        for<'x> <F as View<'x>>::Output: Result,
    {
        self.map(|session| {
            let (view, slot) = session.open(|view, stamp| match Result::into(view) {
                Ok(ok) => Ok(stamp.stamp(ok)),
                Err(err) => Err(stamp.stamp(err)),
            });
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
        self.with(|view, handle| handle.clone().fill(view.clone()))
    }
}

impl<'ub, T, F> fmt::Debug for Bowl<'ub, T, F>
where
    F: for<'x> ViewIn<'x, 'ub> + ?Sized,
    for<'x> <F as ViewIn<'x, 'ub>>::Target: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg_struct = f.debug_struct("BowlRef");
        self.with(|view, _| {
            dbg_struct.field("view", view);
        });
        dbg_struct.finish_non_exhaustive()
    }
}
