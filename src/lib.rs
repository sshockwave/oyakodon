#![no_std]
#![allow(private_bounds, private_interfaces)]
extern crate alloc;

use ::{
    alloc::boxed::Box,
    core::{
        marker::PhantomData,
        mem::{ManuallyDrop, drop, transmute},
        ops::Drop,
    },
};

pub trait Derive: Sized {
    type Base;
    type Derived<'a>
    where
        Self::Base: 'a;
    fn derive(self, base: &mut Self::Base) -> Self::Derived<'_>;
}

pub struct Oyakodon<'a, F: Derive>
where
    F::Base: 'a,
{
    base: *mut F::Base,
    derived: ManuallyDrop<F::Derived<'a>>,
}

pub trait Don {
    type Value<'a>
    where
        Self: 'a;
    fn get(&self) -> &Self::Value<'_>;
    fn get_mut(&mut self) -> &mut Self::Value<'_>;
}

/// The primary constructor. All other `from_*` functions are convenience wrappers around this.
pub fn from_derive_into<'a, F: Derive>(
    base: F::Base,
    derive: impl Derive<Base = F::Base, Derived<'a> = F::Derived<'a>>,
) -> Oyakodon<'a, F> {
    let base = Box::into_raw(Box::new(base));
    // SAFETY: The lifetime `'a` passed to `derive()` might differ from the actual borrow,
    // but the HRTB requires `derive()` to work uniformly for any lifetime,
    // so `derive()` cannot exploit the length of `'a`.
    // Thus we can assume that `derive()` had received the real lifetime of `*base`.
    // Now `derived` is annotated with a fake lifetime `'a`
    // and the safety of reading `derived` is handed off to getters.
    let derived = derive.derive(unsafe { &mut *base });
    Oyakodon {
        base,
        derived: ManuallyDrop::new(derived),
    }
}

pub fn from_derive<'a, F: Derive>(base: F::Base, derive: F) -> Oyakodon<'a, F> {
    from_derive_into(base, derive)
}

pub fn from_fn<'a, T, F>(base: T, derive: F) -> Oyakodon<'a, DeriveFunction<F, T>>
where
    for<'b> F: OnInput<&'b mut T>,
{
    from_derive(base, DeriveFunction(derive, PhantomData))
}

pub fn from_fn_into<'a, F: Derive>(
    base: F::Base,
    derive: impl for<'b> OnInput<&'b mut F::Base, Output = F::Derived<'b>>,
) -> Oyakodon<'a, F> {
    from_derive_into(base, DeriveFunction(derive, PhantomData))
}

pub fn from_boxed_into<'a, F: Derive>(
    base: F::Base,
    derive: Box<dyn for<'b> FnOnce(&'b mut F::Base) -> F::Derived<'b>>,
) -> Oyakodon<'a, F> {
    from_derive_into(base, DeriveFunction(derive, PhantomData))
}

pub fn from_dyn_into<'a, 'b, F: Derive + 'b>(
    base: F::Base,
    derive: &'b dyn for<'c> Fn(&'c mut F::Base) -> F::Derived<'c>,
) -> Oyakodon<'a, F> {
    from_derive_into(base, DeriveFunction(derive, PhantomData))
}

pub fn from_dyn_mut_into<'a, 'b, F: Derive + 'b>(
    base: F::Base,
    derive: &'b mut dyn for<'c> FnMut(&'c mut F::Base) -> F::Derived<'c>,
) -> Oyakodon<'a, F> {
    from_derive_into(base, DeriveFunction(derive, PhantomData))
}

impl<'a, F: Derive> Oyakodon<'a, F> {
    pub fn map<'b, G, H>(mut self, f: H) -> Oyakodon<'b, G>
    where
        for<'c> H: OnInput<F::Derived<'c>>,
        for<'c> G: Derive<Base = F::Base, Derived<'c> = <H as OnInput<F::Derived<'c>>>::Output>,
    {
        let base = self.base;
        let derived = &mut self.derived;
        // SAFETY: The memory of `derived` is freed by `forget(self)`,
        // and its resources are transferred to the new copy.
        let derived = unsafe { ManuallyDrop::take(derived) };
        core::mem::forget(self);
        let derived = f.call(derived);
        // SAFETY: Same as `cast()`.
        let derived = unsafe { (&raw const derived).cast::<G::Derived<'_>>().read() };
        Oyakodon {
            base,
            derived: ManuallyDrop::new(derived),
        }
    }

    pub fn cast_lifetime<'b>(self) -> Oyakodon<'b, F> {
        // SAFETY: The HRTB on `derive()` guarantees any `'c`
        // that does not outlive `*base` can produce a valid `F::Derived<'c>`.
        // Since `*base` is heap-allocated and outlives `self`
        // any borrow `'b` of `self` satisfies the HRTB.
        // This is not a contradiction to the possible invariance of `F::Derived`.
        // Think of it as if `derive()` was called with the lifetime of the borrow,
        // and we merely used `'a` as a placeholder.
        // The memory layout is the same because lifetime information is erased in runtime.
        unsafe { transmute(self) }
    }
    pub fn cast_lifetime_ref<'b>(&self) -> &Oyakodon<'b, F> {
        // SAFETY: Same as `cast_lifetime()`.
        unsafe { transmute(self) }
    }
    pub fn cast_lifetime_mut<'b>(&mut self) -> &mut Oyakodon<'b, F> {
        // SAFETY: Same as `cast_lifetime()`.
        unsafe { transmute(self) }
    }

    pub fn cast<'b, G>(self) -> Oyakodon<'b, G>
    where
        for<'c> G: Derive<Base = F::Base, Derived<'c> = F::Derived<'c>>,
    {
        // SAFETY: The HRTB of this function maintains the HRTB invariant of `derive()`.
        // The object cast implementation follows `ManuallyDrop::into_inner`
        // because the compiler can't figure out that their size are the same.
        unsafe { (&raw const self).cast::<Oyakodon<'_, _>>().read() }
    }

    // Convenience delegation so callers don't need to import the trait.
    pub fn get(&self) -> &F::Derived<'_> {
        Don::get(self)
    }
    pub fn get_mut(&mut self) -> &mut F::Derived<'_> {
        Don::get_mut(self)
    }

    pub fn into_inner(mut self) -> F::Base {
        let base = self.base;
        // SAFETY: This runs the deconstructor of `derived`,
        // and the memory is freed in `forget(self)`.
        unsafe { ManuallyDrop::drop(&mut self.derived) }
        core::mem::forget(self);
        // SAFETY: `*base` is not used elsewhere after `derived` is dropped.
        *unsafe { Box::from_raw(base) }
    }
}

impl<'a, F: Derive> Don for Oyakodon<'a, F> {
    type Value<'b>
        = F::Derived<'b>
    where
        Self: 'b;
    fn get(&self) -> &Self::Value<'_> {
        // SAFETY: Reading `derived` is safe only if
        // the lifetime passed to `derive()` is shorter than that of `*base`.
        // Ideally we would like to use the lifetime of the `self` instance
        // because that's the actual lifetime of `*base`,
        // but we don't know about that yet,
        // so using `'b` is the best we can do.
        &*self.cast_lifetime_ref().derived
    }
    fn get_mut(&mut self) -> &mut Self::Value<'_> {
        // SAFETY: Same as `get()`, but for mutable references.
        &mut *self.cast_lifetime_mut().derived
    }
}

impl<'a, F: Derive> Drop for Oyakodon<'a, F> {
    fn drop(&mut self) {
        // SAFETY: `derived` may hold references to `*base`, so it must be dropped first.
        unsafe { ManuallyDrop::drop(&mut self.derived) }
        // SAFETY: `*base` is heap-allocated and should be deallocated exactly once.
        drop(unsafe { Box::from_raw(self.base) });
    }
}

trait OnInput<T> {
    type Output;
    fn call(self, input: T) -> Self::Output;
}

impl<T, F, R> OnInput<T> for F
where
    F: FnOnce(T) -> R,
{
    type Output = R;
    fn call(self, input: T) -> Self::Output {
        self(input)
    }
}

struct DeriveFunction<F, T>(F, PhantomData<T>);

impl<T, F> Derive for DeriveFunction<F, T>
where
    for<'a> F: OnInput<&'a mut T>,
{
    type Base = T;
    type Derived<'a>
        = <F as OnInput<&'a mut T>>::Output
    where
        Self::Base: 'a;
    fn derive<'a>(self, base: &'a mut Self::Base) -> Self::Derived<'a> {
        self.0.call(base)
    }
}
