//! Oyakodon is a library for almost[^almost] zero-cost self-referential structs in Rust.
//! It aims to provide a primitive, flexible, and safe API for this purpose,
//! without any assumptions about the container type or the view type.
//! This is a technical documentation for the library.
//! Users can simply refer to [crates.io](https://crates.io/crates/oyakodon) for a quick start guide.
//!
//! [^almost]: "Almost" because the API is zero-cost,
//! but the some niche compiler optimizations are turned off for safety.
//!
//! # What are self-referential structs?
//! There are many cases where we want to bundle a reference together with the owner of the data it points to.
//! For example, a zero-copy parser might store some [`Cow`] inside the parsed result
//! instead of creating new owned strings.
//! ```rust,ignore
//! fn read_and_parse(file: &str) -> (String, Vec<&str>) {
//!     let s = std::fs::read_to_string(file).unwrap();
//!     let parsed = s.split_whitespace().collect();
//!     (s, parsed)
//! }
//! ```
//! String `s` is moved out of the function, too,
//! so the reference in `parsed` is still valid,
//! but Rust's borrow checker cannot verify that
//! and will report a [E0515] error.
//! This is because each reference must have a determined lifetime,
//! and the lifetime it assigns to `parsed` is only valid before `s` is moved.
//!
//! [`Cow`]: std::borrow::Cow
//! [E0515]: https://doc.rust-lang.org/error_codes/E0515.html
//!
//! In general, we want to create a struct
//! that allows one or more fields to borrow from other fields in the same struct:
//! ```rust,ignore
//! struct SelfRef {
//!    owner: String,
//!    view: Vec<&'owner str>,
//! }
//! ```
//! The `'owner` here is not a real lifetime that can be used in Rust,
//! but it represents the fact that `view` borrows from `owner`.
//! [`oyakodon`][crate], like many other crates,
//! provides a way to construct such self-referential containers without using unsafe code directly.
//! What makes [`oyakodon`][crate] different is that
//! it does not require the user to use macros or implement unsafe traits.
//! It also allows arbitrary view types and
//! is more powerful[^powerful] than [covariance]-based solutions without sacrificing safety.
//!
//! [covariance]: https://doc.rust-lang.org/nomicon/subtyping.html#variance
//! [^powerful]: See `covariant.rs` in the `examples/` for a constructive proof.
//!
//! # Safety Overivew
//! The idea of typing is simple.
//! We don't know when will the owner be dropped,
//! so we require users to proof that their code works for every possible `'owner`
//! using higher-ranked trait bounds ([HRTB]s).
//! This is an invariant that we maintain throughout the codebase.
//!
//! [HRTB]: https://doc.rust-lang.org/nomicon/hrtb.html
//!
//! `unsafe` is avoided wherever possible to minimize the review surface.
//! We employ the standard [Miri] tool to run tests for better memory checks.
//! The issue related to LLVM `noalias` found in other solutions are mitigated using [`MaybeDangling`].
//! [`BowlMut`] does not actually need that because we do not allow access to the pointer that marked `noalias`,
//! but we still need that to eliminate Miri `Unique` tagging errors.
//! We also use [`MaybeDangling`] to remove the `dereferenceable` attribute for views.
//!
//! [Miri]: https://github.com/rust-lang/miri/
//! [`MaybeDangling`]: std::mem::MaybeDangling
//! [`BowlMut`]: crate::legacy::BowlMut
//!
//! # About AI
//! The tests are vibed while not the rest.
//! AI-generated code are explicitly marked with `Co-Authored-By` in commit messages.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::type_complexity)]
#![warn(unsafe_op_in_unsafe_fn)]
#![deny(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
mod view;

mod aliasable_deref;
mod helper;
pub mod legacy;
mod low_level;
mod owned;
mod polyfill;

use self::deref_move::*;
pub use self::{aliasable_deref::*, low_level::*, owned::*, view::*};
#[cfg(feature = "alloc")]
pub use ::aliasable::{boxed::AliasableBox, string::AliasableString, vec::AliasableVec};

mod deref_move {
    pub trait DerefMove: ::core::ops::DerefMut {
        fn deref_move(self) -> Self::Target;
    }
}
