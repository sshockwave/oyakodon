pub use crate::low_level::polyfill::*;
use ::core::{clone::Clone, default::Default, fmt::Debug, marker::PhantomData};

pub struct PhantomInvariant<T>(PhantomData<fn(T) -> T>);
pub struct PhantomInvariantLifetime<'a>(PhantomInvariant<&'a ()>);

impl<P: Clone> Clone for MaybeDangling<P> {
    fn clone(&self) -> Self {
        MaybeDangling::new(self.as_ref().clone())
    }
}

impl<P: Default> Default for MaybeDangling<P> {
    fn default() -> Self {
        MaybeDangling::new(P::default())
    }
}

impl<P: Debug> Debug for MaybeDangling<P> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        f.debug_tuple("MaybeDangling").field(self.as_ref()).finish()
    }
}

#[rustversion::since(1.66)]
pub use ::core::mem::transmute as transmute_since_1_66;
#[rustversion::before(1.66)]
pub use transmute_unchecked as transmute_since_1_66;
