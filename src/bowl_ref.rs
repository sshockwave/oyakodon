use super::{Derive, StableDeref};
use ::core::{marker::PhantomData, mem::transmute, ops::Deref};

pub struct BowlRef<'a, T: Deref, F: for<'b> Derive<&'b T::Target>> {
    // `base` will be dropped after `derived`.
    // Rust guarantees that fields are dropped in the order of declaration.
    // https://doc.rust-lang.org/reference/destructors.html#r-destructors.operation
    derived: <F as Derive<&'a T::Target>>::Output,
    base: T,
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: StableDeref,
    F: for<'b> Derive<&'b T::Target>,
{
    /// The primary constructor. All other constructors are convenience wrappers around this.
    pub fn from_ptr_into(
        base: T,
        derive: impl for<'b> Derive<&'b T::Target, Output = <F as Derive<&'b T::Target>>::Output>,
    ) -> Self {
        // SAFETY: The lifetime `'a` passed to `derive()` might differ from the actual borrow,
        // but the HRTB requires `derive()` to work uniformly for any lifetime,
        // so `derive()` cannot exploit the length of `'a`.
        // Thus we can assume that `derive()` had received the real lifetime of `*base`.
        // Now `derived` is annotated with a fake lifetime `'a`
        // and the safety of reading `derived` is handed off to getters.
        let derived = derive.call(unsafe { transmute(&*base) });
        BowlRef { base, derived }
    }

    pub fn from_ptr(base: T, derive: F) -> Self {
        Self::from_ptr_into(base, derive)
    }

    pub fn from_fn<'b>(
        base: T,
        derive: &'b dyn for<'c> Fn(&'c T::Target) -> <F as Derive<&'c T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_ptr_into(base, derive)
    }

    pub fn from_fn_mut<'b>(
        base: T,
        derive: &'b mut dyn for<'c> FnMut(&'c T::Target) -> <F as Derive<&'c T::Target>>::Output,
    ) -> Self
    where
        F: 'b,
    {
        Self::from_ptr_into(base, derive)
    }

    #[cfg(feature = "alloc")]
    pub fn from_fn_once(
        base: T,
        derive: ::alloc::boxed::Box<
            dyn for<'c> FnOnce(&'c T::Target) -> <F as Derive<&'c T::Target>>::Output,
        >,
    ) -> Self {
        Self::from_ptr_into(base, derive)
    }

    pub fn map_into<'b, G, H>(self, f: H) -> BowlRef<'b, T, G>
    where
        for<'c> H: Derive<<F as Derive<&'c T::Target>>::Output>,
        for<'c> G: Derive<
                &'c T::Target,
                Output = <H as Derive<<F as Derive<&'c T::Target>>::Output>>::Output,
            >,
    {
        let Self { base, derived } = self;
        // SAFETY: The HRTB on this method maintains the HRTB invariant on `derive()`.
        BowlRef::<'_, T, G> {
            base,
            derived: f.call(derived),
        }
        .cast()
    }

    pub fn map<G>(self, f: G) -> BowlRef<'a, T, Map<T::Target, F, G>>
    where
        G: for<'b> Derive<<F as Derive<&'b T::Target>>::Output>,
    {
        self.map_into(f)
    }
}

impl<'a, T, F> BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> Derive<&'b T::Target>,
{
    pub fn cast_ref<'b, G>(&self) -> &BowlRef<'b, T, G>
    where
        for<'c> G: Derive<&'c T::Target, Output = <F as Derive<&'c T::Target>>::Output>,
    {
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
    pub fn cast_mut<'b, G>(&mut self) -> &mut BowlRef<'b, T, G>
    where
        for<'c> G: Derive<&'c T::Target, Output = <F as Derive<&'c T::Target>>::Output>,
    {
        // SAFETY: Same as `cast_ref()`.
        unsafe { transmute(self) }
    }
    pub fn cast<'b, G>(self) -> BowlRef<'b, T, G>
    where
        for<'c> G: Derive<&'c T::Target, Output = <F as Derive<&'c T::Target>>::Output>,
    {
        // SAFETY: The object cast implementation follows `ManuallyDrop::into_inner`
        // because the compiler can't figure out that their sizes are the same.
        unsafe { (&raw const self).cast::<BowlRef<'_, _, _>>().read() }
    }

    pub fn get(&self) -> &<F as Derive<&'_ T::Target>>::Output {
        // SAFETY: Reading `derived` is safe only if
        // the lifetime passed to `derive()` is shorter than that of `*base`.
        // Ideally we would like to use the lifetime of the `self` instance
        // because that's the actual lifetime of `*base`,
        // but we don't know about that yet,
        // so using `'b` is the best we can do.
        &self.cast_ref::<F>().derived
    }
    pub fn get_mut(&mut self) -> &mut <F as Derive<&'_ T::Target>>::Output {
        // SAFETY: Same as `get()`, but for mutable references.
        &mut self.cast_mut::<F>().derived
    }

    pub fn into_ptr(self) -> T {
        let Self { base, derived: _ } = self;
        // SAFETY: `*base` is not used elsewhere after `derived` is dropped.
        base
    }
}

impl<'a, T, F> Clone for BowlRef<'a, T, F>
where
    T: super::CloneStableDeref,
    F: for<'b> Derive<&'b T::Target>,
    for<'b> <F as Derive<&'b T::Target>>::Output: Clone,
{
    fn clone(&self) -> Self {
        let base = self.base.clone();
        // SAFETY: `StableDeref` should guarantee that `*base` outlives `base`,
        // so the new `derived` will be valid as long as we hold the new `base`.
        let derived = self.derived.clone();
        Self { base, derived }
    }
}

// SAFETY: `T::Target: Sync` is implied by `Derive::Output: Send` on demand
unsafe impl<'a, T, F> Send for BowlRef<'a, T, F>
where
    T: StableDeref + Send,
    F: for<'b> Derive<&'b T::Target>,
    for<'b> <F as Derive<&'b T::Target>>::Output: Send,
{
}
// SAFETY: We do not provide access to `&*base` since it can be stored in `derived`.
// That gives us the flexibility to omit `T: Sync`.
unsafe impl<'a, T, F> Sync for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> Derive<&'b T::Target>,
    for<'b> <F as Derive<&'b T::Target>>::Output: Sync,
{
}

#[cfg(feature = "gat")]
impl<'a, T, F> super::Bowl for BowlRef<'a, T, F>
where
    T: Deref,
    F: for<'b> Derive<&'b T::Target>,
{
    type Value<'b>
        = <F as Derive<&'b T::Target>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        BowlRef::get(self)
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        BowlRef::get_mut(self)
    }
}

pub struct Map<T: ?Sized, F, G>(G, PhantomData<(F, T)>);
impl<'a, T: ?Sized, F, G> Derive<&'a T> for Map<T, F, G>
where
    F: for<'b> Derive<&'b T>,
    G: for<'b> Derive<<F as Derive<&'b T>>::Output>,
{
    type Output = <G as Derive<<F as Derive<&'a T>>::Output>>::Output;
}
