//! Primitive types and traits.
//! This is where all `unsafe` code lives,
//! and the rest of the codebase is built on top of it.
//!
//! # A Journey to Safe Self-Referential Types
//! TODO

mod helper;
mod view;

pub(crate) use self::deref_move::*;
pub use self::{super::low_level::*, aliasable_deref::*, owned::*, view::*};

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
            self.0.as_ref()
        }

        pub fn get_mut(&mut self) -> &mut T {
            self.0.as_mut()
        }

        pub fn into_inner(self) -> T {
            MaybeDangling::into_inner(self.0)
        }
    }

    impl<T: Deref> Deref for AliasableDeref<T> {
        type Target = T::Target;
        fn deref(&self) -> &Self::Target {
            self.get()
        }
    }

    impl<T: DerefMut> DerefMut for AliasableDeref<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.get_mut()
        }
    }
}

mod owned {
    use crate::primitive::DerefMove;
    use ::core::ops::{Deref, DerefMut};

    pub struct Owned<T: ?Sized>(T);

    impl<T> Owned<T> {
        pub const fn new(value: T) -> Self {
            Self(value)
        }
    }

    impl<T: ?Sized> Deref for Owned<T> {
        type Target = T;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: ?Sized> DerefMut for Owned<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T> DerefMove for Owned<T> {
        fn deref_move(self) -> Self::Target {
            self.0
        }
    }
}
