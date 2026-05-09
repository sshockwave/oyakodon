//! Primitive types and traits.
//! This is where all `unsafe` code lives,
//! and the rest of the codebase is built on top of it.
//!
//! # A Journey to Safe Self-Referential Types
//! TODO

mod helper;
mod view;

pub(crate) use self::deref_move::*;
pub use self::{super::low_level::core::*, aliasable_deref::*, view::*};

mod deref_move {
    pub trait DerefMove: ::core::ops::DerefMut {
        fn deref_move(self) -> Self::Target;
    }
}

mod aliasable_deref {
    use crate::polyfill::MaybeDangling;
    use ::core::ops::{Deref, DerefMut};

    #[derive(Clone)]
    pub struct AliasableDeref<T>(MaybeDangling<T>);

    impl<T> AliasableDeref<T> {
        pub fn new(inner: T) -> Self {
            Self(MaybeDangling::new(inner))
        }

        pub fn get(&self) -> &T {
            &*self.0
        }

        pub fn get_mut(&mut self) -> &mut T {
            &mut *self.0
        }

        pub fn into_inner(self) -> T {
            MaybeDangling::into_inner(self.0)
        }
    }

    impl<T: Deref> Deref for AliasableDeref<T> {
        type Target = T::Target;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: DerefMut> DerefMut for AliasableDeref<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}
