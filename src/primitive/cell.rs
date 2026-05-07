use super::Owner;
use ::{core::marker::PhantomData, maybe_dangling::MaybeDangling};

pub struct Cell<'id, T>(PhantomData<&'id ()>, MaybeDangling<T>);

impl<'id, T> Cell<'id, T> {
    pub fn borrow<'a>(&'a self) -> Cell<'id, &'a T> {
        Cell::new(&*self.1)
    }

    pub fn borrow_mut<'a>(&'a mut self) -> Cell<'id, &'a mut T> {
        Cell::new(&mut *self.1)
    }
}

impl<'id, T> Cell<'id, T> {
    pub fn new(value: T) -> Self {
        Self(PhantomData, MaybeDangling::new(value))
    }
}

impl<'id, T> Cell<'id, T> {
    pub fn map<R, P>(self, _token: &Owner<'id, P>, f: impl for<'x> FnOnce(T) -> R) -> R {
        f(MaybeDangling::into_inner(self.1))
    }
}
