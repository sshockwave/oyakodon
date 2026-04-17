# oyakodon

[![crates.io](https://img.shields.io/crates/v/oyakodon)](https://crates.io/crates/oyakodon)
[![docs.rs](https://docs.rs/oyakodon/badge.svg)](https://docs.rs/oyakodon)
[![CI](https://github.com/sshockwave/oyakodon/actions/workflows/test.yml/badge.svg)](https://github.com/sshockwave/oyakodon/actions/workflows/test.yml)

[`oyakodon`] lets you store a value together with a derived view of it.
This pattern is known as self-referential structs in Rust,
which currently can't be written without the help of unsafe code.
This crate provides simple primitives to eliminate the need for macros or unsafe code and supports [`no_std`].

The name comes from the Japanese dish where chicken (the parent) and egg (the child) are served together.

## Usage

```rust
use oyakodon::BowlBox;

fn parse_words(s: &mut String) -> Vec<&str> {
    s.split_whitespace().collect()
}

let mut view = BowlBox::new("hello world foo".to_owned(), parse_words);

// Get a reference to the derived value via `spawn` and `spawn_mut`
view.spawn_mut(|v: &mut Vec<&_>| {
    v[2] = "bar";
});
let new_sentence = view.spawn(|v: &Vec<&str>| {
    v.iter()
        .map(|x| (*x).to_owned())
        .collect::<Vec<_>>()
        .join(" ")
});
assert_eq!(new_sentence, "hello world bar");

assert_eq!(view.into_owner(), "hello world foo");
```

The container is a monadic type that you can play around like [`Option`].

```rust
use oyakodon::BowlBox;

fn parse_and_double(
    s: &mut String,
) -> Result<std::future::Ready<i32>, std::num::ParseIntError> {
    Ok(std::future::ready(s.trim().parse::<i32>()? * 2))
}

let result = smol::block_on(async {
    BowlBox::new("21".to_owned(), parse_and_double)
        .into_result() // Result<BowlBox<...>, BowlBox<...>>
        .unwrap()      // BowlBox<impl Future<Output = i32>>
        .into_async()  // impl Future<Output = BowlBox<i32>>
        .await         // BowlBox<i32>
        .into_view()   // i32
});
assert_eq!(result, 42);
```

Named functions are recommended,
but closures can be used to create a [`BowlBox`] when the target type is specified.
Choose a function from [`from_fn`]/[`from_fn_mut`]/[`from_fn_once`]:

```rust
use oyakodon::{BowlBox, View};

struct Word;
impl<'a> View<&'a mut String> for Word {
    type Output = &'a str;
}

let nth_word = 1;
let view = BowlBox::<_, Word>::from_fn("hello world foo".to_owned(), &|s| {
    s.split_whitespace().nth(nth_word).unwrap_or("")
});
view.spawn(|v: &&_| assert_eq!(*v, "world"));
```

<details>
<summary>Why does closure usages have to be like this?</summary>

This is a limitation in current Rust.
Closures require the unstable [`#![feature(closure_lifetime_binder)]`][closure_lifetime_binder]
to return a reference depending on an argument.
In comparison, named functions are automatically generic over the lifetimes of their inputs.

We use a `dyn` function trick to coerce the closure into a generic one.
You can use the [`higher_order_closure`] crate to use the unstable feature today,
or you can define your own [`Derive`] implementation to avoid this performance cost:

```rust
use oyakodon::{BowlBox, Derive, View};

struct NthWord(usize);
impl<'a> View<&'a mut String> for NthWord {
    type Output = &'a str;
}
impl<'a> Derive<&'a mut String> for NthWord {
    fn call(self, s: &'a mut String) -> &'a str {
        s.split_whitespace().nth(self.0).unwrap_or("")
    }
}

let view = BowlBox::new("hello world foo".to_owned(), NthWord(1));
view.spawn(|v: &&_| assert_eq!(*v, "world"));
```

</details>

If the `owner` value is behind a shared pointer, use [`BowlRef`] to receive a pointer instead:

```rust
use oyakodon::BowlRef;
use std::rc::Rc;

fn parse_words(s: &String) -> Vec<&str> {
    s.split_whitespace().collect()
}

let view = BowlRef::new(Rc::new("hello world foo".to_owned()), parse_words);
view.spawn(|v: &Vec<&_>| assert_eq!(v, &["hello", "world", "foo"]));

let _view = view.clone();
```

We also created [`BowlMut`] to support other owned containers like [`String`] or [`Vec`].
You can create your own container as well,
but that requires an unsafe implementation of [`StableDeref`].

Due to technical reasons, two [`Bowl`]s with exactly same owner and view types might not be of the same type.
But you can [`cast()`] between them.

```rust
use oyakodon::{BowlBox, View};

// Two different view marker types that both produce `usize`
fn str_len(s: &mut String) -> usize {
    s.len()
}

struct Len;
impl<'a> View<&'a mut String> for Len {
    type Output = usize;
}

let a = BowlBox::new("hello".to_owned(), str_len);
let b: BowlBox<_, Len> = a.cast();
assert_eq!(b.spawn(|v: &_| *v), 5);
```

See [docs.rs](https://docs.rs/oyakodon) for full API documentation.

## Safety Overivew

We use HRBT to make the constructor of the view prove that every lifetime is valid for the derived view.
Hence it's safe to alter its lifetime to the latest known lifetime of `'self`.
We employ the standard [Miri] tool to run tests for better memory checks.
`unsafe` is avoided wherever possible to minimize the review surface.

The issue related to LLVM `noalias` found in other solutions are mitigated using [`MaybeDangling`].
[`BowlMut`] does not actually need that because we do not allow access to the pointer that marked `noalias`,
but we still need that to remove the `dereferenceable` attribute and Miri `Unique` tagging errors.

About AI: The tests are vibed while not the rest.
AI-generated code are explicitly marked with `Co-Authored-By` in commit messages.

## Alternatives

There are plenty of crates for creating self-referential structs
and here we only list those that are still relevant.
However, we fully recognize their efforts on the matter.
We have used the test suites and experiences from them to create our tests.

- [`owning_ref`](https://docs.rs/owning_ref/)/[`safer_owning_ref`](https://docs.rs/safer_owning_ref) - similar concept, simpler typing but only allows storing one reference
- [`yoke`](https://docs.rs/yoke) - similar concept, provides macro to implement unsafe traits
- [`self_cell`](https://docs.rs/self_cell) - macro-based struct generation
- [`rel_ptr`](https://docs.rs/rel-ptr) - uses unsafe code for relative pointers
- [`nolife`](https://docs.rs/nolife/) - uses async functions

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[Miri]: https://github.com/rust-lang/miri/
[`no_std`]: https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute
[`Option`]: https://doc.rust-lang.org/std/option/enum.Option.html
[`String`]: https://doc.rust-lang.org/std/string/struct.String.html
[`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
[`MaybeDangling`]: https://docs.rs/maybe-dangling/latest/maybe_dangling/struct.MaybeDangling.html
[`higher_order_closure`]: https://docs.rs/higher-order-closure/
[closure_lifetime_binder]: https://rust-lang.github.io/rfcs/3216-closure-lifetime-binder.html
[`oyakodon`]: https://docs.rs/oyakodon
[`Bowl`]: https://docs.rs/oyakodon/latest/oyakodon/trait.Bowl.html
[`BowlRef`]: https://docs.rs/oyakodon/latest/oyakodon/struct.BowlRef.html
[`BowlMut`]: https://docs.rs/oyakodon/latest/oyakodon/struct.BowlMut.html
[`BowlBox`]: https://docs.rs/oyakodon/latest/oyakodon/struct.BowlBox.html
[`cast()`]: https://docs.rs/oyakodon/latest/oyakodon/struct.BowlRef.html#method.cast
[`from_fn()`]: https://docs.rs/oyakodon/latest/oyakodon/struct.BowlRef.html#method.from_fn
[`from_fn_ref()`]: https://docs.rs/oyakodon/latest/oyakodon/struct.BowlRef.html#method.from_fn_ref
[`from_fn_mut()`]: https://docs.rs/oyakodon/latest/oyakodon/struct.BowlRef.html#method.from_fn_mut
