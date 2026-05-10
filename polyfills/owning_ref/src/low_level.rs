#![allow(unsafe_code)]

use super::{CloneStableAddress, Erased, OwningHandle, OwningRef, OwningRefMut, StableAddress};
use ::{
    alloc::{rc::Rc, sync::Arc},
    core::{
        cell::{Ref, RefCell, RefMut},
        mem::transmute,
        ops::{Deref, DerefMut},
    },
    oyakodon::{AliasableDeref, Bowl},
};

pub unsafe trait IntoErased<'a> {
    type Erased;
    fn into_erased(self) -> Self::Erased;
}

impl<'t, O, T: 't + ?Sized> OwningRef<'t, O, T> {
    pub unsafe fn new_assert_stable_address(o: O) -> Self
    where
        O: Deref<Target = T>,
    {
        Self(Bowl::new(AliasableDeref::new(o)).map(|owner, slot, intro| {
            let view = slot.as_owner(&owner);
            let view = unsafe { transmute::<&T, &T>(view) };
            slot.fill((owner.into(), view), intro)
        }))
    }
}

impl<'t, O, T: 't + ?Sized> OwningRefMut<'t, O, T> {
    pub unsafe fn new_assert_stable_address(o: O) -> Self
    where
        O: DerefMut<Target = T>,
    {
        Self(
            Bowl::new(AliasableDeref::new(o)).map(|mut view, mut slot, intro| {
                let view = slot.as_owner_mut(&mut view);
                let view = unsafe { transmute::<&mut T, &mut T>(view) };
                slot.fill(view, intro)
            }),
        )
    }

    #[cfg(any())]
    #[deprecated(note = "unsafe function. can create aliased references")]
    pub unsafe fn map<F, U: 't + ?Sized>(self, f: F) -> OwningRef<'t, O, U>
    where
        O: StableAddress,
        F: FnOnce(&mut T) -> &U,
    {
        OwningRef(self.0.map(|view, slot, intro| slot.fill(f(view), intro)))
    }

    #[cfg(any())]
    #[deprecated(note = "unsafe function. can create aliased references")]
    pub unsafe fn try_map<F, U: 't + ?Sized, E>(self, f: F) -> Result<OwningRef<'t, O, U>, E>
    where
        O: StableAddress,
        F: FnOnce(&mut T) -> Result<&U, E>,
    {
        self.0.map(|view, slot, intro| match f(view) {
            Ok(view) => Ok(OwningRef(slot.fill(view, intro))),
            Err(e) => Err(e),
        })
    }
}

unsafe impl<O, H> StableAddress for OwningHandle<O, H>
where
    O: StableAddress,
    H: StableAddress,
{
}

pub trait ToHandle {
    type Handle: Deref;
    unsafe fn to_handle(x: *const Self) -> Self::Handle;
}

pub trait ToHandleMut {
    type HandleMut: DerefMut;
    unsafe fn to_handle_mut(x: *const Self) -> Self::HandleMut;
}

impl<O, H> OwningHandle<O, H>
where
    O: StableAddress,
    O::Target: ToHandle<Handle = H>,
    H: Deref,
{
    pub fn new(o: O) -> Self {
        OwningHandle::new_with_fn(o, |x| unsafe { O::Target::to_handle(x) })
    }
}

impl<O, H> OwningHandle<O, H>
where
    O: StableAddress,
    O::Target: ToHandleMut<HandleMut = H>,
    H: DerefMut,
{
    pub fn new_mut(o: O) -> Self {
        OwningHandle::new_with_fn(o, |x| unsafe { O::Target::to_handle_mut(x) })
    }
}

unsafe impl<'t, O, T: ?Sized> StableAddress for OwningRef<'t, O, T> {}
unsafe impl<'t, O, T: ?Sized> StableAddress for OwningRefMut<'t, O, T> {}
unsafe impl<'t, O, T: ?Sized> CloneStableAddress for OwningRef<'t, O, T> where O: CloneStableAddress {}

unsafe impl<'t, O, T: ?Sized> Send for OwningRef<'t, O, T>
where
    O: Send,
    for<'a> &'a T: Send,
{
}
unsafe impl<'t, O, T: ?Sized> Sync for OwningRef<'t, O, T>
where
    O: Sync,
    for<'a> &'a T: Sync,
{
}

unsafe impl<'t, O, T: ?Sized> Send for OwningRefMut<'t, O, T>
where
    O: Send,
    for<'a> &'a mut T: Send,
{
}
unsafe impl<'t, O, T: ?Sized> Sync for OwningRefMut<'t, O, T>
where
    O: Sync,
    for<'a> &'a mut T: Sync,
{
}

impl<T: 'static> ToHandle for RefCell<T> {
    type Handle = Ref<'static, T>;
    unsafe fn to_handle(x: *const Self) -> Self::Handle {
        unsafe { (*x).borrow() }
    }
}

impl<T: 'static> ToHandleMut for RefCell<T> {
    type HandleMut = RefMut<'static, T>;
    unsafe fn to_handle_mut(x: *const Self) -> Self::HandleMut {
        unsafe { (*x).borrow_mut() }
    }
}

unsafe impl<'a, T: 'a> IntoErased<'a> for Box<T> {
    type Erased = Box<dyn Erased + 'a>;
    fn into_erased(self) -> Self::Erased {
        self
    }
}
unsafe impl<'a, T: 'a> IntoErased<'a> for Rc<T> {
    type Erased = Rc<dyn Erased + 'a>;
    fn into_erased(self) -> Self::Erased {
        self
    }
}
unsafe impl<'a, T: 'a> IntoErased<'a> for Arc<T> {
    type Erased = Arc<dyn Erased + 'a>;
    fn into_erased(self) -> Self::Erased {
        self
    }
}
