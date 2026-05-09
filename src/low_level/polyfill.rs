use ::core::mem::{self, MaybeUninit};

pub struct MaybeDangling<P>(MaybeUninit<P>);

impl<P> MaybeDangling<P> {
    /// Wraps a value in a `MaybeDangling`, allowing it to dangle.
    pub const fn new(x: P) -> Self
    where
        P: Sized,
    {
        MaybeDangling(MaybeUninit::new(x))
    }

    /// Returns a reference to the inner value.
    pub fn as_ref(&self) -> &P {
        // SAFETY: The value is always initialized.
        unsafe { self.0.assume_init_ref() }
    }

    /// Returns a mutable reference to the inner value.
    pub fn as_mut(&mut self) -> &mut P {
        // SAFETY: The value is always initialized.
        unsafe { self.0.assume_init_mut() }
    }

    /// Extracts the value from the `MaybeDangling` container.
    ///
    /// Note that this is UB if the inner value is currently dangling.
    pub fn into_inner(self) -> P
    where
        P: Sized,
    {
        let manual = mem::ManuallyDrop::new(self);
        // SAFETY: The value is always initialized,
        // and we are taking ownership of it,
        // so we should read it and forget the container.
        unsafe { manual.0.assume_init_read() }
    }
}

impl<P> Drop for MaybeDangling<P> {
    fn drop(&mut self) {
        // SAFETY: The value is always initialized,
        // and its destructor will not be called,
        // so we should drop it in place.
        unsafe { self.0.as_mut_ptr().drop_in_place() }
    }
}

#[rustversion::before(1.66)]
pub unsafe fn transmute_unchecked<Src, Dst>(src: Src) -> Dst {
    let dst: *const Src = &src;
    let dst = dst.cast();
    let dst = unsafe { ::core::ptr::read(dst) };
    ::core::mem::forget(src);
    dst
}
