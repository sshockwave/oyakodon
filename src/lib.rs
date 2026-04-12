#![no_std]
extern crate alloc;

use ::{
    alloc::boxed::Box,
    core::{marker::PhantomData, mem::transmute},
};

pub struct BowlMut<'a, T, F: for<'b> Derive<&'b mut T>> {
    // `base` will be dropped after `derived`.
    // Rust guarantees that fields are dropped in the order of declaration.
    // https://doc.rust-lang.org/reference/destructors.html#r-destructors.operation
    derived: <F as Derive<&'a mut T>>::Output,
    base: Box<T>,
}

#[cfg(feature = "gat")]
pub trait Bowl {
    type Value<'a>
    where
        Self: 'a;
    fn get(&self) -> &Self::Value<'_>;
    fn get_mut(&mut self) -> &mut Self::Value<'_>;
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

impl<'a, T, F> BowlMut<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    /// The primary constructor. All other `from_*` functions are convenience wrappers around this.
    pub fn new_into(
        base: T,
        derive: impl for<'b> Derive<&'b mut T, Output = <F as Derive<&'b mut T>>::Output>,
    ) -> Self {
        let mut base = Box::new(base);
        // SAFETY: The lifetime `'a` passed to `derive()` might differ from the actual borrow,
        // but the HRTB requires `derive()` to work uniformly for any lifetime,
        // so `derive()` cannot exploit the length of `'a`.
        // Thus we can assume that `derive()` had received the real lifetime of `*base`.
        // Now `derived` is annotated with a fake lifetime `'a`
        // and the safety of reading `derived` is handed off to getters.
        let derived = derive.call(unsafe { transmute(&mut *base) });
        BowlMut { base, derived }
    }

    pub fn new(base: T, derive: F) -> Self {
        Self::new_into(base, derive)
    }

    pub fn map_into<'b, G, H>(self, f: H) -> BowlMut<'b, T, G>
    where
        for<'c> H: Derive<<F as Derive<&'c mut T>>::Output>,
        for<'c> G:
            Derive<&'c mut T, Output = <H as Derive<<F as Derive<&'c mut T>>::Output>>::Output>,
    {
        let Self { base, derived } = self;
        // SAFETY: The HRTB on this method maintains the HRTB invariant on `derive()`.
        BowlMut::<'_, T, G> {
            base,
            derived: f.call(derived),
        }
        .cast()
    }

    pub fn map<G>(self, f: G) -> BowlMut<'a, T, Map<T, F, G>>
    where
        G: for<'b> Derive<<F as Derive<&'b mut T>>::Output>,
    {
        self.map_into(f)
    }

    pub fn cast_ref<'b, G>(&self) -> &BowlMut<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T, Output = <F as Derive<&'c mut T>>::Output>,
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
    pub fn cast_mut<'b, G>(&mut self) -> &mut BowlMut<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T, Output = <F as Derive<&'c mut T>>::Output>,
    {
        // SAFETY: Same as `cast_ref()`.
        unsafe { transmute(self) }
    }
    pub fn cast<'b, G>(self) -> BowlMut<'b, T, G>
    where
        for<'c> G: Derive<&'c mut T, Output = <F as Derive<&'c mut T>>::Output>,
    {
        // SAFETY: The object cast implementation follows `ManuallyDrop::into_inner`
        // because the compiler can't figure out that their sizes are the same.
        unsafe { (&raw const self).cast::<BowlMut<'_, _, _>>().read() }
    }

    pub fn get(&self) -> &<F as Derive<&'_ mut T>>::Output {
        // SAFETY: Reading `derived` is safe only if
        // the lifetime passed to `derive()` is shorter than that of `*base`.
        // Ideally we would like to use the lifetime of the `self` instance
        // because that's the actual lifetime of `*base`,
        // but we don't know about that yet,
        // so using `'b` is the best we can do.
        &self.cast_ref::<F>().derived
    }
    pub fn get_mut(&mut self) -> &mut <F as Derive<&'_ mut T>>::Output {
        // SAFETY: Same as `get()`, but for mutable references.
        &mut self.cast_mut::<F>().derived
    }

    pub fn into_inner(self) -> T {
        let Self { base, derived: _ } = self;
        // SAFETY: `*base` is not used elsewhere after `derived` is dropped.
        *base
    }
}

#[cfg(feature = "gat")]
impl<'a, T, F> Bowl for BowlMut<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    type Value<'b>
        = <F as Derive<&'b mut T>>::Output
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        BowlMut::get(self)
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        BowlMut::get_mut(self)
    }
}

pub struct Map<T, F, G>(G, PhantomData<(T, F)>);
impl<'a, T, F, G> Derive<&'a mut T> for Map<T, F, G>
where
    F: for<'b> Derive<&'b mut T>,
    G: for<'b> Derive<<F as Derive<&'b mut T>>::Output>,
{
    type Output = <G as Derive<<F as Derive<&'a mut T>>::Output>>::Output;
    fn call(self, _: &'a mut T) -> Self::Output {
        unreachable!()
    }
}

pub struct DynFnOnce<T, F>(Box<dyn for<'a> FnOnce(&'a mut T) -> <F as Derive<&'a mut T>>::Output>)
where
    F: for<'a> Derive<&'a mut T>;
impl<'a, T, F> Derive<&'a mut T> for DynFnOnce<T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    type Output = <F as Derive<&'a mut T>>::Output;
    fn call(self, input: &'a mut T) -> Self::Output {
        self.0(input)
    }
}
impl<T, F> DynFnOnce<T, F>
where
    F: for<'a> Derive<&'a mut T>,
{
    pub fn new(
        derive: Box<dyn for<'a> FnOnce(&'a mut T) -> <F as Derive<&'a mut T>>::Output>,
    ) -> Self {
        Self(derive)
    }
}

pub struct DynFn<'a, T, F: 'a>(&'a dyn for<'b> Fn(&'b mut T) -> <F as Derive<&'b mut T>>::Output)
where
    F: for<'b> Derive<&'b mut T>;
impl<'a, 'b, T, F> Derive<&'b mut T> for DynFn<'a, T, F>
where
    F: for<'c> Derive<&'c mut T>,
{
    type Output = <F as Derive<&'b mut T>>::Output;
    fn call(self, input: &'b mut T) -> Self::Output {
        self.0(input)
    }
}
impl<'a, T, F: 'a> DynFn<'a, T, F>
where
    F: for<'c> Derive<&'c mut T>,
{
    pub fn new(derive: &'a dyn for<'b> Fn(&'b mut T) -> <F as Derive<&'b mut T>>::Output) -> Self {
        Self(derive)
    }
}

pub struct DynFnMut<'a, T, F: 'a>(
    &'a mut dyn for<'b> FnMut(&'b mut T) -> <F as Derive<&'b mut T>>::Output,
)
where
    F: for<'b> Derive<&'b mut T>;
impl<'a, 'b, T, F> Derive<&'b mut T> for DynFnMut<'a, T, F>
where
    F: for<'c> Derive<&'c mut T>,
{
    type Output = <F as Derive<&'b mut T>>::Output;
    fn call(self, input: &'b mut T) -> Self::Output {
        self.0(input)
    }
}
impl<'a, T, F: 'a> DynFnMut<'a, T, F>
where
    F: for<'b> Derive<&'b mut T>,
{
    pub fn new(
        derive: &'a mut dyn for<'b> FnMut(&'b mut T) -> <F as Derive<&'b mut T>>::Output,
    ) -> Self {
        Self(derive)
    }
}
