//! Primitive types and traits.
//! This is where all `unsafe` code lives,
//! and the rest of the codebase is built on top of it.
//!
//! # A Journey to Safe Self-Referential Types
//! TODO

mod helper;
mod view;

pub use self::{super::low_level::core::*, dangling_deref::DanglingDeref, view::*};
pub(crate) use self::{deref_move::*, phantom_variance_markers::*};

mod deref_move {
    pub trait DerefMove: ::core::ops::DerefMut {
        fn deref_move(self) -> Self::Target;
    }
}

mod phantom_variance_markers {
    use ::core::marker::PhantomData;
    pub struct PhantomInvariant<T>(PhantomData<fn(T) -> T>);
    pub struct PhantomInvariantLifetime<'a>(PhantomInvariant<&'a ()>);
}

mod dangling_deref {
    use ::{
        core::ops::{Deref, DerefMut},
        maybe_dangling::MaybeDangling,
    };

    pub struct DanglingDeref<T>(MaybeDangling<T>);

    impl<T> DanglingDeref<T> {
        pub fn new(inner: T) -> Self {
            Self(MaybeDangling::new(inner))
        }

        pub fn into_inner(self) -> T {
            MaybeDangling::into_inner(self.0)
        }
    }

    impl<T: Deref> Deref for DanglingDeref<T> {
        type Target = T::Target;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: DerefMut> DerefMut for DanglingDeref<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}
