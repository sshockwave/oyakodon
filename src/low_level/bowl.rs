use crate::{
    polyfill::MaybeDangling, Aliasable, BoundedView, CloneStableDeref, DerefMove, Exists, Intro,
    Owned, Taker, View,
};
use ::core::{
    clone::Clone,
    marker::PhantomData,
    mem::{drop, transmute},
    ops::{Deref, DerefMut},
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
pub struct Bowl<'ub, P, F: ?Sized + for<'x> BoundedView<'x, 'ub>>(
    Exists<
        'ub,
        dyn for<'life> View<
                'life,
                Output = BowlInner<
                    Slot<'life, Owned<P>>,
                    MaybeDangling<<F as BoundedView<'life, 'ub>>::Target>,
                >,
            > + 'static,
    >,
);

struct BowlInner<O: ?Sized, V> {
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

/// [`Access`] is a zero-sized token to access the owner
/// so you don't have to store the owner reference in the view type.
pub struct Access<'life, const UNIQUE: bool>(PhantomData<&'life ()>);

impl<'life> From<Access<'life, true>> for Access<'life, false> {
    fn from(_token: Access<'life, true>) -> Self {
        Self(PhantomData)
    }
}

impl<P> Bowl<'_, P, dyn for<'x> View<'x, Output = Access<'x, true>>> {
    pub fn new(owner: P) -> Self {
        Self(Exists::new().map(|(), intro| {
            intro.pack(BowlInner {
                view: MaybeDangling::new(Access(PhantomData)),
                owner: Slot(PhantomData, Owned::new(owner)),
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
    ) -> Exists<
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a <F as BoundedView<'x, 'ub>>::Target,
                    &'a Slot<'x, Owned<P>>,
                ),
            > + 'static,
    > {
        self.0
            .borrow(|bowl, intro| intro.pack((bowl.view.as_ref(), &bowl.owner)))
    }

    pub fn borrow_mut<'a>(
        &'a mut self,
    ) -> Exists<
        'ub,
        dyn for<'x> View<
                'x,
                Output = (
                    &'a mut <F as BoundedView<'x, 'ub>>::Target,
                    &'a mut Slot<'x, Owned<P>>,
                ),
            > + 'static,
    > {
        self.0
            .borrow_mut(|bowl, intro| intro.pack((bowl.view.as_mut(), &mut bowl.owner)))
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
            <F as View<'life>>::Output,
            Slot<'life, Taker<'owner, P>>,
            Intro<'life, 'ub>,
        ) -> R,
    ) -> R {
        self.0.map(|BowlInner { view, owner }, intro| {
            let mut owner = Some(owner.into_owner());
            let taker = unsafe { Taker::new(&mut owner) };
            let result = f(
                MaybeDangling::into_inner(view),
                Slot(PhantomData, taker),
                intro,
            );
            drop(owner);
            result
        })
    }
}

pub struct Slot<'life, O: ?Sized>(PhantomData<&'life ()>, O);

mod with_new_bounded_view {
    use super::*;
    bounded_view!(
        pub trait BoundedView {}
    );

    impl<'life, O> Slot<'life, O>
    where
        O: DerefMove,
        O::Target: Sized,
    {
        pub fn fill<'long, 'ub, F, X: ?Sized>(
            self,
            view: <F as View<'life>>::Output,
            intro: Intro<'life, 'ub, X>,
        ) -> Bowl<'ub, O::Target, F>
        where
            F: ?Sized + for<'x> BoundedView<'x, 'long>,
            'long: 'ub + 'life,
        {
            Bowl(intro.pack(BowlInner {
                view: MaybeDangling::new(view),
                owner: Slot(self.0, Owned::new(self.1.deref_move())),
            }))
        }
    }
}

impl<O> Slot<'_, O>
where
    O: DerefMove,
    O::Target: Sized,
{
    pub fn into_owner(self) -> O::Target {
        self.1.deref_move()
    }
}

impl<'life, O> Slot<'life, O>
where
    O: Deref + ?Sized,
    O::Target: CloneStableDeref,
{
    pub fn spawn(&self) -> Slot<'life, Owned<O::Target>> {
        Slot(self.0, Owned::new(self.1.clone()))
    }
}

impl<'life, O> Slot<'life, O> {
    pub fn borrow(&self, _token: Access<'life, false>) -> &O::Target
    where
        O: Deref,
        O::Target: Aliasable,
    {
        // SAFETY: The owner cannot be mutated
        // and thus maintains the invariant of `StableDeref`.
        &self.1
    }

    pub fn deref<'a, const U: bool>(
        &'a self,
        _token: &'a Access<'life, U>,
    ) -> &'a <O::Target as Deref>::Target
    where
        O: Deref,
        O::Target: Deref,
    {
        &self.1
    }

    pub fn deref_mut<'a>(
        &'a mut self,
        // `mut` is not needed
        // because shared tokens must be used on owners
        // that dereference to the same target,
        // so it must implement `CloneStableDeref`
        // and never `DerefMut`.
        _token: &'a Access<'life, true>,
    ) -> &'a mut <O::Target as Deref>::Target
    where
        O: DerefMut,
        O::Target: DerefMut,
    {
        &mut self.1
    }

    pub fn spawn_ref<const U: bool>(
        &self,
        token: Access<'life, U>,
    ) -> &'life <O::Target as Deref>::Target
    where
        O: Deref,
        O::Target: Aliasable,
    {
        let view = self.deref(&token);
        unsafe {
            transmute::<&<O::Target as Deref>::Target, &'life <O::Target as Deref>::Target>(view)
        }
    }

    pub fn spawn_mut(
        &mut self,
        token: Access<'life, true>,
    ) -> &'life mut <O::Target as Deref>::Target
    where
        O: DerefMut,
        O::Target: Aliasable + DerefMut,
    {
        let view = self.deref_mut(&token);
        unsafe {
            transmute::<&mut <O::Target as Deref>::Target, &'life mut <O::Target as Deref>::Target>(
                view,
            )
        }
    }
}
