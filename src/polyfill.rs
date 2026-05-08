pub use crate::low_level::polyfill::*;
use ::core::marker::PhantomData;
pub use ::maybe_dangling::MaybeDangling;

pub struct PhantomInvariant<T>(PhantomData<fn(T) -> T>);
pub struct PhantomInvariantLifetime<'a>(PhantomInvariant<&'a ()>);
