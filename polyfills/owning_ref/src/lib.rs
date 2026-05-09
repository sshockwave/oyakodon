#![deny(unsafe_code)]
#![warn(unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod low_level;

pub use self::low_level::*;
pub use ::oyakodon::{CloneStableDeref as CloneStableAddress, StableDeref as StableAddress};
use ::{
    alloc::{rc::Rc, sync::Arc},
    core::{
        borrow::{Borrow, BorrowMut},
        cell::{Ref, RefMut},
        cmp::Ordering,
        fmt::{self, Debug},
        hash::{Hash, Hasher},
        ops::{Deref, DerefMut},
    },
    oyakodon::{Access, AliasableDeref, Bowl, Intro, Owned, Slot, View},
    std::sync::{MutexGuard, RwLockReadGuard, RwLockWriteGuard},
};

/// [`safer_owning_ref::OwningRef`] includes a lifetime parameter
/// to avoid extending the lifetime of a reference to an external object via [`Self::map`].
/// It would be unsound without this lifetime parameter:
/// ```
/// use ::owning_ref::OwningRef;
/// let x = OwningRef::new(Box::new(()));
/// let z: OwningRef<Box<()>, str>;
/// {
///     let s = "Hello World!".to_string();
///     let s_ref: &str = &s;
///     let y: OwningRef<Box<()>, &str> = x.map(|_| &s_ref);
///     z = y.map(|s: &&str| *s);
///     // s deallocated here
/// }
/// // Invalid access to deallocated memory!
/// // println!("{}", &*z);
/// ```
/// See [this issue](https://github.com/Kimundi/owning-ref-rs/pull/71) for more details.
pub struct OwningRef<'t, O, T: ?Sized>(
    Bowl<'t, AliasableDeref<O>, dyn for<'x> View<'x, Output = (Access<'x, false>, &'x T)>>,
);

pub struct OwningRefMut<'t, O, T: ?Sized>(
    Bowl<'t, AliasableDeref<O>, dyn for<'x> View<'x, Output = &'x mut T>>,
);

pub trait Erased {}
impl<T> Erased for T {}

/////////////////////////////////////////////////////////////////////////////
// OwningRef
/////////////////////////////////////////////////////////////////////////////

impl<'t, O, T: ?Sized> OwningRef<'t, O, T> {
    pub fn new(o: O) -> Self
    where
        O: StableAddress,
        O: Deref<Target = T>,
        T: 't,
    {
        Self(Bowl::new(AliasableDeref::new(o)).map(|owner, slot, intro| {
            let owner = owner.into();
            let view = slot.spawn_ref(owner);
            slot.fill((owner, view), intro)
        }))
    }

    pub fn map<F, U: 't + ?Sized>(self, f: F) -> OwningRef<'t, O, U>
    where
        O: StableAddress,
        F: FnOnce(&T) -> &U,
    {
        OwningRef(
            self.0
                .map(|(owner, view), slot, intro| slot.fill((owner, f(view)), intro)),
        )
    }

    // TODO: unsafe fn map_with_owner_direct

    pub fn map_with_owner<F, U: 't + ?Sized>(self, f: F) -> OwningRef<'t, O, U>
    where
        O: StableAddress + Deref,
        F: for<'a> FnOnce(&'a O::Target, &'a T) -> &'a U,
        O::Target: 't,
    {
        OwningRef(self.0.map(|(owner, view), slot, intro| {
            let view = f(slot.spawn_ref(owner), view);
            slot.fill((owner, view), intro)
        }))
    }

    pub fn try_map<F, U: 't + ?Sized, E>(self, f: F) -> Result<OwningRef<'t, O, U>, E>
    where
        O: StableAddress,
        F: FnOnce(&T) -> Result<&U, E>,
    {
        self.0.map(|(owner, view), slot, intro| match f(view) {
            Ok(view) => Ok(OwningRef(slot.fill((owner, view), intro))),
            Err(e) => Err(e),
        })
    }

    // TODO: unsafe fn try_map_with_owner_direct

    pub fn try_map_with_owner<F, U: 't + ?Sized, E>(self, f: F) -> Result<OwningRef<'t, O, U>, E>
    where
        O: StableAddress + Deref,
        F: for<'a> FnOnce(&'a O::Target, &'a T) -> Result<&'a U, E>,
        O::Target: 't,
    {
        self.0.map(
            |(owner, view), slot, intro| match f(slot.spawn_ref(owner), view) {
                Ok(view) => Ok(OwningRef(slot.fill((owner, view), intro))),
                Err(e) => Err(e),
            },
        )
    }

    // TODO: unsafe fn map_owner
    // TODO: fn map_owner_box
    // TODO: fn erase_owner

    fn as_owner(&self) -> &O
    where
        O: StableAddress,
    {
        fn get_owner<'x, 'a, 't, O: StableAddress, T: ?Sized>(
            ((owner, _), slot): (
                &'a (Access<'x, false>, &'x T),
                &'a Slot<'x, Owned<AliasableDeref<O>>>,
            ),
            _: Intro<'x, 't>,
        ) -> &'a AliasableDeref<O> {
            slot.borrow(*owner)
        }
        self.0.borrow().map(get_owner).get()
    }

    pub fn into_owner(self) -> O {
        self.0.into_owner().into_inner()
    }
}

impl<'t, O, T: ?Sized> OwningRefMut<'t, O, T> {
    pub fn new(o: O) -> Self
    where
        O: StableAddress,
        O: DerefMut<Target = T>,
        T: 't,
    {
        Self(Bowl::new_mut(AliasableDeref::new(o)))
    }

    pub fn map_mut<F, U: 't + ?Sized>(self, f: F) -> OwningRefMut<'t, O, U>
    where
        O: StableAddress,
        F: FnOnce(&mut T) -> &mut U,
    {
        OwningRefMut(self.0.map(|view, slot, intro| slot.fill(f(view), intro)))
    }

    pub fn try_map_mut<F, U: 't + ?Sized, E>(self, f: F) -> Result<OwningRefMut<'t, O, U>, E>
    where
        O: StableAddress,
        F: FnOnce(&mut T) -> Result<&mut U, E>,
    {
        self.0.map(|view, slot, intro| match f(view) {
            Ok(view) => Ok(OwningRefMut(slot.fill(view, intro))),
            Err(e) => Err(e),
        })
    }

    // TODO: unsafe fn map_owner
    // TODO: fn map_owner_box
    // TODO: fn erase_owner
    // TODO: unsafe fn as_owner
    // TODO: unsafe fn as_owner_mut

    pub fn into_owner(self) -> O {
        self.0.into_owner().into_inner()
    }
}

/////////////////////////////////////////////////////////////////////////////
// OwningHandle
/////////////////////////////////////////////////////////////////////////////

pub struct OwningHandle<O, H>
where
    O: StableAddress,
    H: Deref,
{
    handle: H,
    _owner: AliasableDeref<O>,
}

impl<O, H> Deref for OwningHandle<O, H>
where
    O: StableAddress,
    H: Deref,
{
    type Target = H::Target;
    fn deref(&self) -> &H::Target {
        self.handle.deref()
    }
}

impl<O, H> DerefMut for OwningHandle<O, H>
where
    O: StableAddress,
    H: DerefMut,
{
    fn deref_mut(&mut self) -> &mut H::Target {
        self.handle.deref_mut()
    }
}

impl<O, H> OwningHandle<O, H>
where
    O: StableAddress,
    H: Deref,
{
    pub fn new_with_fn<F>(o: O, f: F) -> Self
    where
        F: FnOnce(*const O::Target) -> H,
    {
        let o = AliasableDeref::new(o);
        let h: H = f(&*o as *const O::Target);

        OwningHandle {
            handle: h,
            _owner: o,
        }
    }
    pub fn try_new<F, E>(o: O, f: F) -> Result<Self, E>
    where
        F: FnOnce(*const O::Target) -> Result<H, E>,
    {
        let o = AliasableDeref::new(o);
        let h: H = f(&*o as *const O::Target)?;

        Ok(OwningHandle {
            handle: h,
            _owner: o,
        })
    }

    pub fn as_owner(&self) -> &O {
        self._owner.get()
    }

    pub fn into_owner(self) -> O {
        self._owner.into_inner()
    }
}

/////////////////////////////////////////////////////////////////////////////
// std traits
/////////////////////////////////////////////////////////////////////////////

impl<O, T: ?Sized> Deref for OwningRef<'_, O, T> {
    type Target = T;
    fn deref(&self) -> &T {
        fn get_view<'x, 'a, 't, O, T: ?Sized>(
            ((_, view), _): (
                &'a (Access<'x, false>, &'x T),
                &'a Slot<'x, Owned<AliasableDeref<O>>>,
            ),
            _: Intro<'x, 't>,
        ) -> &'a T {
            *view
        }
        self.0.borrow().map(get_view)
    }
}

impl<O, T: ?Sized> Deref for OwningRefMut<'_, O, T> {
    type Target = T;

    fn deref(&self) -> &T {
        fn get_view<'x, 'a, 't, O, T: ?Sized>(
            (view, _): (&'a &'x mut T, &'a Slot<'x, Owned<AliasableDeref<O>>>),
            _: Intro<'x, 't>,
        ) -> &'a T {
            *view
        }
        self.0.borrow().map(get_view)
    }
}

impl<'t, O, T: ?Sized> DerefMut for OwningRefMut<'t, O, T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        fn get_view<'x, 'a, 't, O, T: ?Sized>(
            (view, _): (
                &'a mut &'x mut T,
                &'a mut Slot<'x, Owned<AliasableDeref<O>>>,
            ),
            _: Intro<'x, 't>,
        ) -> &'a mut T {
            &mut **view
        }
        self.0.borrow_mut().map(get_view)
    }
}

impl<'t, O, T: ?Sized> AsRef<T> for OwningRef<'t, O, T> {
    fn as_ref(&self) -> &T {
        &*self
    }
}

impl<'t, O, T: ?Sized> AsRef<T> for OwningRefMut<'t, O, T> {
    fn as_ref(&self) -> &T {
        &*self
    }
}

impl<'t, O, T: ?Sized> AsMut<T> for OwningRefMut<'t, O, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut *self
    }
}

impl<'t, O, T: ?Sized> Borrow<T> for OwningRef<'t, O, T> {
    fn borrow(&self) -> &T {
        &*self
    }
}

impl<'t, O, T: ?Sized> Borrow<T> for OwningRefMut<'t, O, T> {
    fn borrow(&self) -> &T {
        &*self
    }
}

impl<'t, O, T: ?Sized> BorrowMut<T> for OwningRefMut<'t, O, T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut *self
    }
}

impl<'t, O, T: 't + ?Sized> From<O> for OwningRef<'t, O, T>
where
    O: StableAddress,
    O: Deref<Target = T>,
{
    fn from(owner: O) -> Self {
        OwningRef::new(owner)
    }
}

impl<'t, O, T: 't + ?Sized> From<O> for OwningRefMut<'t, O, T>
where
    O: StableAddress,
    O: DerefMut<Target = T>,
{
    fn from(owner: O) -> Self {
        OwningRefMut::new(owner)
    }
}

impl<'t, O, T: ?Sized> Debug for OwningRef<'t, O, T>
where
    O: Debug + StableAddress,
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("OwningRef")
            .field("owner", self.as_owner())
            .field("reference", &&**self)
            .finish()
    }
}

impl<'t, O, T: ?Sized> Debug for OwningRefMut<'t, O, T>
where
    O: Debug,
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        struct Elided;
        impl Debug for Elided {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "_")
            }
        }
        f.debug_struct("OwningRefMut")
            .field("owner", &Elided)
            .field("reference", &&**self)
            .finish()
    }
}

impl<'t, O, T: ?Sized> Clone for OwningRef<'t, O, T>
where
    O: CloneStableAddress,
{
    fn clone(&self) -> Self {
        Self(
            self.0
                .borrow()
                .map(|(view, slot), intro| slot.spawn().fill(*view, intro)),
        )
    }
}

impl Debug for dyn Erased {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "<dyn Erased>",)
    }
}

impl<'t, O, T: ?Sized> PartialEq for OwningRef<'t, O, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        (&*self as &T).eq(&*other as &T)
    }
}

impl<'t, O, T: ?Sized> Eq for OwningRef<'t, O, T> where T: Eq {}

impl<'t, O, T: ?Sized> PartialOrd for OwningRef<'t, O, T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (&*self as &T).partial_cmp(&*other as &T)
    }
}

impl<'t, O, T: ?Sized> Ord for OwningRef<'t, O, T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        (&*self as &T).cmp(&*other as &T)
    }
}

impl<'t, O, T: ?Sized> Hash for OwningRef<'t, O, T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&*self as &T).hash(state);
    }
}

impl<'t, O, T: ?Sized> PartialEq for OwningRefMut<'t, O, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        (&*self as &T).eq(&*other as &T)
    }
}

impl<'t, O, T: ?Sized> Eq for OwningRefMut<'t, O, T> where T: Eq {}

impl<'t, O, T: ?Sized> PartialOrd for OwningRefMut<'t, O, T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (&*self as &T).partial_cmp(&*other as &T)
    }
}

impl<'t, O, T: ?Sized> Ord for OwningRefMut<'t, O, T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        (&*self as &T).cmp(&*other as &T)
    }
}

impl<'t, O, T: ?Sized> Hash for OwningRefMut<'t, O, T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&*self as &T).hash(state);
    }
}

/////////////////////////////////////////////////////////////////////////////
// std types integration and convenience type defs
/////////////////////////////////////////////////////////////////////////////

// NB: Implementing ToHandle{,Mut} for Mutex and RwLock requires a decision
// about which handle creation to use (i.e. read() vs try_read()) as well as
// what to do with error results.

pub type BoxRef<'u, T, U = T> = OwningRef<'u, Box<T>, U>;
pub type VecRef<'u, T, U = T> = OwningRef<'u, Vec<T>, U>;
pub type StringRef<'u> = OwningRef<'u, String, str>;

pub type RcRef<'u, T, U = T> = OwningRef<'u, Rc<T>, U>;
pub type ArcRef<'u, T, U = T> = OwningRef<'u, Arc<T>, U>;

pub type RefRef<'a, T, U = T> = OwningRef<'a, Ref<'a, T>, U>;
pub type RefMutRef<'a, T, U = T> = OwningRef<'a, RefMut<'a, T>, U>;
pub type MutexGuardRef<'a, T, U = T> = OwningRef<'a, MutexGuard<'a, T>, U>;
pub type RwLockReadGuardRef<'a, T, U = T> = OwningRef<'a, RwLockReadGuard<'a, T>, U>;
pub type RwLockWriteGuardRef<'a, T, U = T> = OwningRef<'a, RwLockWriteGuard<'a, T>, U>;

pub type BoxRefMut<'u, T, U = T> = OwningRefMut<'u, Box<T>, U>;
pub type VecRefMut<'u, T, U = T> = OwningRefMut<'u, Vec<T>, U>;
pub type StringRefMut<'u> = OwningRefMut<'u, String, str>;

pub type RefMutRefMut<'a, T, U = T> = OwningRefMut<'a, RefMut<'a, T>, U>;
pub type MutexGuardRefMut<'a, T, U = T> = OwningRefMut<'a, MutexGuard<'a, T>, U>;
pub type RwLockWriteGuardRefMut<'a, T, U = T> = OwningRefMut<'a, RwLockWriteGuard<'a, T>, U>;

pub type ErasedBoxRef<'u, U> = OwningRef<'u, Box<dyn Erased>, U>;
pub type ErasedRcRef<'u, U> = OwningRef<'u, Rc<dyn Erased>, U>;
pub type ErasedArcRef<'u, U> = OwningRef<'u, Arc<dyn Erased>, U>;

pub type ErasedBoxRefMut<'u, U> = OwningRefMut<'u, Box<dyn Erased>, U>;

#[cfg(test)]
mod tests;
