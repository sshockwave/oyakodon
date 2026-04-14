use super::{Derive, StableDeref, View};
use ::{
    core::{
        cmp::{Eq, PartialEq},
        convert::{AsMut, AsRef},
        hash::{Hash, Hasher},
        marker::PhantomData,
        mem::{forget, transmute},
        ops::Deref,
    },
    maybe_dangling::MaybeDangling,
};

pub struct BowlRef<'a, T: Deref, F: for<'b> View<&'b T::Target> + ?Sized> {
    // `base` will be dropped after `derived`.
    // Rust guarantees that fields are dropped in the order of declaration.
    // https://doc.rust-lang.org/reference/destructors.html#r-destructors.operation
    //
    // Both fields are wrapped in `MaybeDangling` for distinct reasons:
    //
    // `derived: MaybeDangling<_>` suppresses Tree Borrows reference protection.
    // When `BowlRef` is passed by value to a function,
    // Tree Borrows would normally "protect" any references inside it for the entire call duration,
    // asserting that the memory they point to remains valid.
    // But `BowlRef` owns both `derived` (the borrow) and `base` (the allocation),
    // so dropping `BowlRef` inside the callee frees `base` while `derived` is still considered live.
    // `MaybeDangling` opts out of the `dereferenceable` assumption, suppressing the protector.
    //
    // `base: MaybeDangling<_>` suppresses Stacked Borrows Unique retag.
    // Box-like types assert Unique ownership over their allocation whenever they are moved
    // (e.g., as a function argument).
    // If `base` were moved after `derived` was computed,
    // the resulting Unique retag would invalidate `derived`'s SharedReadWrite tag on the same allocation.
    // Wrapping `base` in `MaybeDangling` (a union) before calling `derive`
    // means no further Box move occurs after `derived` is created.
    derived: MaybeDangling<<F as View<&'a T::Target>>::Output>,
    base: MaybeDangling<T>,
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: StableDeref,
    F: for<'b> Derive<&'b T::Target>,
{
    pub fn new(base: T, derive: F) -> Self {
        Self::from_derive(base, derive)
    }
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: StableDeref,
    F: for<'b> View<&'b T::Target> + ?Sized,
{
    /// The primary constructor. All other constructors are convenience wrappers around this.
    pub fn from_derive(
        base: T,
        derive: impl for<'b> Derive<&'b T::Target, Output = <F as View<&'b T::Target>>::Output>,
    ) -> Self {
        let base = MaybeDangling::new(base);
        // SAFETY: The lifetime `'a` passed to `derive()` might differ from the actual borrow,
        // but the HRTB requires `derive()` to work uniformly for any lifetime,
        // so `derive()` cannot exploit the length of `'a`.
        // Thus we can assume that `derive()` had received the real lifetime of `*base`.
        // Now `derived` is annotated with a fake lifetime `'a`
        // and the safety of reading `derived` is handed off to getters.
        // We ensure that the base is never accessed until `derived` is dropped
        // to satisfy the possible LLVM `noalias` attribute on `base`.
        let derived = derive.call(unsafe { transmute(&**base) });
        BowlRef {
            base,
            derived: MaybeDangling::new(derived),
        }
    }
    pub fn from_fn<'b>(
        base: T,
        derive: &'b dyn for<'c> Fn(&'c T::Target) -> <F as View<&'c T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(base, derive)
    }

    pub fn from_fn_mut<'b>(
        base: T,
        derive: &'b mut dyn for<'c> FnMut(&'c T::Target) -> <F as View<&'c T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_derive(base, derive)
    }

    #[cfg(feature = "alloc")]
    pub fn from_fn_once(
        base: T,
        derive: ::alloc::boxed::Box<
            dyn for<'c> FnOnce(&'c T::Target) -> <F as View<&'c T::Target>>::Output,
        >,
    ) -> Self {
        Self::from_derive(base, derive)
    }

    pub fn map_into<'b, G: ?Sized, H>(self, f: H) -> BowlRef<'b, T, G>
    where
        for<'c> H: Derive<<F as View<&'c T::Target>>::Output>,
        for<'c> G:
            View<&'c T::Target, Output = <H as View<<F as View<&'c T::Target>>::Output>>::Output>,
    {
        let Self { base, derived } = self;
        // SAFETY: The HRTB on this method maintains the HRTB invariant on `derive()`.
        BowlRef::<'_, T, G> {
            base,
            derived: MaybeDangling::new(f.call(MaybeDangling::into_inner(derived))),
        }
        .cast()
    }

    pub fn map<G>(self, f: G) -> BowlRef<'a, T, Map<T::Target, F, G>>
    where
        G: for<'b> Derive<<F as View<&'b T::Target>>::Output>,
    {
        self.map_into(f)
    }
}

impl<'a, 'b, T, F, G> AsRef<BowlRef<'b, T, G>> for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c T::Target> + ?Sized,
    G: for<'c> View<&'c T::Target, Output = <F as View<&'c T::Target>>::Output> + ?Sized,
{
    fn as_ref(&self) -> &BowlRef<'b, T, G> {
        // SAFETY: We maintain an HRTB invariant on `derive()`
        // to make sure any `'c` that does not outlive `*base`
        // corresponds to a valid `F::Output<'c>`.
        // Since `*base` is heap-allocated and outlives `self`
        // any borrow `'b` of `self` satisfies the HRTB.
        // This is not a contradiction to the possible invariance of `F::Output`.
        // Think of it as if `derive()` was called with the lifetime of the borrow,
        // and we merely used `'a` as a placeholder.
        // The memory layout is the same because lifetime information is erased in runtime.
        unsafe { transmute(self) }
    }
}

impl<'a, 'b, T, F, G> AsMut<BowlRef<'b, T, G>> for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'c> View<&'c T::Target> + ?Sized,
    G: for<'c> View<&'c T::Target, Output = <F as View<&'c T::Target>>::Output> + ?Sized,
{
    fn as_mut(&mut self) -> &mut BowlRef<'b, T, G> {
        // SAFETY: Same as `as_ref()`.
        unsafe { transmute(self) }
    }
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b T::Target> + ?Sized,
{
    pub fn cast<'b, G: ?Sized>(self) -> BowlRef<'b, T, G>
    where
        for<'c> G: View<&'c T::Target, Output = <F as View<&'c T::Target>>::Output>,
    {
        // SAFETY: The object cast implementation follows `ManuallyDrop::into_inner`
        // because the compiler can't figure out that their sizes are the same.
        // Extra care needs to be taken that the resources in `self` shouldn't be freed.
        let result = unsafe { (&raw const self).cast::<BowlRef<'_, _, _>>().read() };
        forget(self);
        result
    }

    pub fn into_inner(self) -> T {
        let Self { base, derived } = self;
        // `base` must be dropped even if `derived`'s drop panics.
        // Miri reports that this is not guaranteed
        // if `derived` is dropped implicitly at the end of the function.
        drop(derived);
        // SAFETY: `*base` is not used elsewhere after `derived` is dropped.
        MaybeDangling::into_inner(base)
    }

    pub fn into_view<S>(self) -> S
    where
        for<'c> F: View<&'c T::Target, Output = S>,
    {
        // SAFETY: The HRTB requires `F::Output` to not depend on `base`.
        MaybeDangling::into_inner(self.derived)
    }

    pub fn get(&self) -> &<F as View<&'_ T::Target>>::Output {
        // SAFETY: Reading `derived` is safe only if
        // the lifetime passed to `derive()` is shorter than that of `*base`.
        // Ideally we would like to use the lifetime of the `self` instance
        // because that's the actual lifetime of `*base`,
        // but we don't know about that yet,
        // so using `'b` is the best we can do.
        let other: &BowlRef<_, F> = self.as_ref();
        &*other.derived
    }
    pub fn get_mut(&mut self) -> &mut <F as View<&'_ T::Target>>::Output {
        // SAFETY: Same as `get()`, but for mutable references.
        let other: &mut BowlRef<_, F> = self.as_mut();
        &mut *other.derived
    }
}

impl<'a, T, F> Clone for BowlRef<'a, T, F>
where
    T: super::CloneStableDeref,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Clone,
{
    fn clone(&self) -> Self {
        let base = self.base.clone();
        // SAFETY: `StableDeref` should guarantee that `*base` outlives `base`,
        // so the new `derived` will be valid as long as we hold the new `base`.
        let derived = self.derived.clone();
        Self { base, derived }
    }
}

// SAFETY: We do not provide access to `&*base` since it can be stored in `derived`.
// That gives us the flexibility to omit `T: Sync`.
unsafe impl<'a, T, F> Sync for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Sync,
{
}

#[cfg(feature = "gat")]
impl<'a, T, F> super::Bowl for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> View<&'b T::Target> + ?Sized,
{
    type Value<'b>
        = <F as View<&'b T::Target>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        BowlRef::get(self)
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        BowlRef::get_mut(self)
    }
}

pub struct Map<T: ?Sized, F: ?Sized, G: ?Sized>(PhantomData<F>, PhantomData<T>, G);
impl<'a, T: ?Sized, F, G> View<&'a T> for Map<T, F, G>
where
    F: for<'b> View<&'b T> + ?Sized,
    G: for<'b> View<<F as View<&'b T>>::Output> + ?Sized,
{
    type Output = <G as View<<F as View<&'a T>>::Output>>::Output;
}

// These traits are specific to `BowlRef`
// because they require access to `base`,
// which is not available in `BowlMut`.
impl<'a, 'b, T, F, G> PartialEq<BowlRef<'b, T, G>> for BowlRef<'a, T, F>
where
    T: Deref + PartialEq,
    F: for<'c> View<&'c T::Target> + ?Sized,
    G: for<'c> View<&'c T::Target> + ?Sized,
    for<'c> <F as View<&'c T::Target>>::Output: PartialEq<<G as View<&'c T::Target>>::Output>,
{
    fn eq(&self, other: &BowlRef<'b, T, G>) -> bool {
        // SAFETY: Accessing `base` is safe because `derived` does not have exlusive access to `base`.
        (*self.base).eq(&*other.base) && self.get().eq(other.get())
    }
}

impl<'a, T, F> Eq for BowlRef<'a, T, F>
where
    T: Deref + Eq,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Eq,
{
}

impl<'a, T, F> Hash for BowlRef<'a, T, F>
where
    T: Deref + Hash,
    F: for<'b> View<&'b T::Target> + ?Sized,
    for<'b> <F as View<&'b T::Target>>::Output: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: Same as `PartialEq::eq()`.
        self.base.hash(state);
        self.get().hash(state);
    }
}
