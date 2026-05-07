use super::{Aliasable, Cell, ForAll, View};
use ::core::{marker::PhantomData, mem::transmute};

pub struct Owner<'id, P: ?Sized>(PhantomData<&'id ()>, P);
impl<'id, P> Owner<'id, P> {
    pub fn into_inner(self) -> P {
        self.1
    }
}

pub fn new<'ub, P>(
    owner: P,
) -> ForAll<
    'ub,
    dyn for<'life> View<'life, Output = (Owner<'life, P>, Cell<'life, &'life P::Target>)> + 'static,
>
where
    P: Aliasable + ::core::ops::Deref,
    P::Target: 'ub,
{
    let view = unsafe { transmute::<&P::Target, &'ub P::Target>(&*owner) };
    ForAll::new().map(|(), stamp| stamp.stamp((Owner(PhantomData, owner), Cell::new(view))))
}

pub fn new_mut<'ub, P>(
    mut owner: P,
) -> ForAll<
    'ub,
    dyn for<'life> View<'life, Output = (Owner<'life, P>, Cell<'life, &'life mut P::Target>)>
        + 'static,
>
where
    P: Aliasable + ::core::ops::DerefMut,
    P::Target: 'ub,
{
    let view = unsafe { transmute::<&mut P::Target, &'ub mut P::Target>(&mut *owner) };
    ForAll::new().map(|(), stamp| stamp.stamp((Owner(PhantomData, owner), Cell::new(view))))
}
