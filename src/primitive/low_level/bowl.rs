use crate::primitive::{Aliasable, CloneStableDeref, DerefMove, ForAll, Owned, Stamp, Taker, View};
use ::{
    core::{
        clone::Clone,
        mem::{drop, transmute},
        ops::{Deref, DerefMut},
    },
    maybe_dangling::MaybeDangling,
};

bounded_view!(BoundedView);

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
pub struct Bowl<'ub, P, F: View<'ub> + ?Sized>(
    ForAll<
        'ub,
        dyn for<'x> View<
                'x,
                Output = BowlInner<Slot<'x, 'ub, Owned<P>>, MaybeDangling<<F as View<'x>>::Output>>,
            > + 'static,
    >,
);

struct BowlInner<O, V> {
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
    view: V,
    owner: O,
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> View<'x, Output = &'x P::Target>>
where
    P: Aliasable + Deref,
    P::Target: 'ub,
{
    pub fn new(owner: P) -> Self {
        let view = unsafe { transmute::<&P::Target, &'ub P::Target>(&*owner) };
        Self(ForAll::new().map(|(), stamp| {
            stamp.stamp(BowlInner {
                view: MaybeDangling::new(view),
                owner: Slot(stamp, Owned::new(owner)),
            })
        }))
    }
}

impl<'ub, P> Bowl<'ub, P, dyn for<'x> View<'x, Output = &'x mut P::Target>>
where
    P: Aliasable + DerefMut,
    P::Target: 'ub,
{
    pub fn new_mut(mut owner: P) -> Self {
        let view = unsafe { transmute::<&mut P::Target, &'ub mut P::Target>(&mut *owner) };
        Self(ForAll::new().map(|(), stamp| {
            stamp.stamp(BowlInner {
                view: MaybeDangling::new(view),
                owner: Slot(stamp, Owned::new(owner)),
            })
        }))
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
                    &'a Slot<'x, 'ub, Owned<P>>,
                ),
            > + 'static,
    > {
        self.0
            .borrow(|bowl, stamp| stamp.stamp((&*bowl.view, &bowl.owner)))
    }

    pub fn borrow_mut<'a>(
        &'a mut self,
    ) -> ForAll<
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a mut <F as BoundedView<'x, 'ub>>::Target,
                    &'a Slot<'x, 'ub, Owned<P>>,
                ),
            > + 'static,
    > {
        self.0
            .borrow_mut(|bowl, stamp| stamp.stamp((&mut *bowl.view, &bowl.owner)))
    }

    /// Internally this function uses [`Option`] to check
    /// whether the caller has dropped the owner during the call.
    /// This has some performance overhead,
    /// but the compiler should be able to optimize it to the same level as [drop flags].
    ///
    /// [drop flags]: https://doc.rust-lang.org/reference/destructors.html#drop-flags
    pub fn map<R>(
        self,
        f: impl for<'owner, 'life> FnOnce(
            <F as BoundedView<'life, 'ub>>::Target,
            Slot<'life, 'ub, Taker<'owner, P>>,
        ) -> R,
    ) -> R {
        self.0.map(|BowlInner { view, owner }, stamp| {
            let mut owner = Some(owner.into_owner());
            let taker = unsafe { Taker::new(&mut owner) };
            let result = f(MaybeDangling::into_inner(view), Slot(stamp, taker));
            drop(owner);
            result
        })
    }
}

pub struct Slot<'life, 'ub, O: ?Sized>(Stamp<'life, 'ub>, O);

impl<'life, 'ub, O> Slot<'life, 'ub, O>
where
    O: DerefMove,
    O::Target: Sized,
{
    pub fn fill<'long, F>(self, view: <F as View<'life>>::Output) -> Bowl<'ub, O::Target, F>
    where
        F: ?Sized + for<'x> BoundedView<'x, 'long>,
        'long: 'ub + 'life,
    {
        Bowl(self.0.stamp(BowlInner {
            view: MaybeDangling::new(view),
            owner: Slot(self.0, Owned::new(self.1.deref_move())),
        }))
    }

    pub fn into_owner(self) -> O::Target {
        self.1.deref_move()
    }
}

impl<'life, 'ub, O> Slot<'life, 'ub, O>
where
    O: DerefMove + ?Sized,
    O::Target: CloneStableDeref,
{
    pub fn spawn(&self) -> Slot<'life, 'ub, Owned<O::Target>> {
        Slot(self.0, Owned::new(self.1.clone()))
    }
}
