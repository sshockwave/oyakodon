// Tests migrated from:
//   icu4x/utils/yoke/tests/miri.rs
//   icu4x/utils/yoke/src/yoke.rs    (inline #[test] functions)
//   icu4x/utils/yoke/tests/bincode.rs
//   icu4x/utils/yoke/src/cartable_ptr.rs
//
// API mapping:
//   Yoke::<Y, C>::attach_to_cart(cart, |data| derive(data))
//     → BowlRef::new(cart, derive)
//   Yoke::<Y, C>::attach_to_zero_copy_cart(cart)
//     → BowlRef::new(cart, identity_derive)
//   yoke.get()                       → bowl.spawn(|v| ...)
//   yoke.with_mut(|y| ...)           → bowl.spawn_mut(|v| ...)
//
// miri.rs::run_test is also covered by tests/soundness.rs::yoke_3696 (using BowlMut).
// It is included here for completeness using BowlRef to match the original Yoke semantics.

use oyakodon::{BowlMut, BowlRef, Derive, View};
use std::rc::Rc;

// ================================================================================
// yoke/tests/miri.rs
// ================================================================================

// Tests that passing a bowl by value into a function does not trigger
// Stacked Borrows / Tree Borrows violations under Miri.
// Should be run with `-Zmiri-retag-fields` (enabled by default in recent Miri).
// See https://github.com/unicode-org/icu4x/issues/3696

struct GetSlice;
impl<'a> View<&'a [u8]> for GetSlice {
    type Output = &'a [u8];
}
impl<'a> Derive<&'a [u8]> for GetSlice {
    fn call(self, data: &'a [u8]) -> &'a [u8] {
        data
    }
}

fn example_ref(_: BowlRef<'_, Vec<u8>, GetSlice>) {}

#[test]
fn run_test() {
    example_ref(BowlRef::new(vec![0, 1, 2], GetSlice));
}

// ================================================================================
// yoke/src/yoke.rs inline tests
// ================================================================================

// Shared derive type used by most tests below.
struct BorrowStr;
impl<'a> View<&'a String> for BorrowStr {
    type Output = &'a str;
}
impl<'a> Derive<&'a String> for BorrowStr {
    fn call(self, s: &'a String) -> &'a str {
        s.as_str()
    }
}

// --- test_debug -----------------------------------------------------------------

#[test]
fn test_debug() {
    let bowl = BowlRef::new(Rc::new("foo".to_owned()), BorrowStr);
    let s = format!("{bowl:?}");
    assert!(s.contains("foo"));
}

// --- test_display ---------------------------------------------------------------
// NOT MIGRATABLE: BowlRef does not implement Display.
// Yoke implements Display by delegating to the yokeable's impl.
// oyakodon intentionally omits Display to avoid ambiguity between owner and view.

// --- test_partialeq -------------------------------------------------------------
// BowlRef::PartialEq compares both the owner and the view.
// Two bowls built from separate-but-equal Rc<String> instances are considered equal.
// cast_life::<'static>() normalizes both placeholder lifetimes to the same type
// so the PartialEq impl (which requires matching 'a on both sides) applies.

#[test]
fn test_partialeq() {
    let a = Rc::new("same".to_string());
    let b = Rc::new("same".to_string());
    let y1 = BowlRef::new(a, BorrowStr).cast_life::<'static>();
    let y2 = BowlRef::new(b, BorrowStr).cast_life::<'static>();
    assert_eq!(y1, y2);
}

// --- test_eq_trait --------------------------------------------------------------

#[test]
fn test_eq_trait() {
    let x = Rc::new("equal".to_string());
    let y = Rc::new("equal".to_string());
    let y1 = BowlRef::new(x, BorrowStr).cast_life::<'static>();
    let y2 = BowlRef::new(y, BorrowStr).cast_life::<'static>();
    assert!(y1 == y2);
    let vec = [y1];
    assert!(vec.contains(&y2));
}

// --- test_partialord_ord --------------------------------------------------------
// NOT MIGRATABLE: BowlRef does not implement PartialOrd or Ord.
// Yoke derives these by delegating to the yokeable's impl.
// oyakodon intentionally omits ordering traits for the same reason as Display.

// --- test_clone -----------------------------------------------------------------
// BowlRef::Clone clones the owner (via CloneStableDeref) and the view independently.
// Mutations via get_mut() on a clone must not affect the original.
// Uses Cow<str> to demonstrate that a borrowed view becomes owned after mutation.

use std::borrow::Cow;

struct BorrowCow;
impl<'a> View<&'a String> for BorrowCow {
    type Output = Cow<'a, str>;
}
impl<'a> Derive<&'a String> for BorrowCow {
    fn call(self, s: &'a String) -> Cow<'a, str> {
        Cow::Borrowed(s.as_str())
    }
}

#[test]
fn test_clone() {
    let y1 = BowlRef::new(Rc::new("foo".to_owned()), BorrowCow).cast_life::<'static>();

    let y2 = y1.clone();
    y1.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foo"));
    y2.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foo"));

    let mut y3 = y1.clone();
    y3.spawn_mut(|v: &mut Cow<'_, str>| v.to_mut().push_str("bar"));
    y1.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foo"));
    y2.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foo"));
    y3.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foobar"));

    let y4 = y3.clone();
    y3.spawn_mut(|v: &mut Cow<'_, str>| v.to_mut().push_str("baz"));
    y1.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foo"));
    y2.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foo"));
    y3.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foobarbaz"));
    y4.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "foobar"));
}

// ================================================================================
// yoke/src/cartable_ptr.rs inline tests
// ================================================================================

// NOT MIGRATABLE: CartableOptionPointer is a yoke-specific internal optimization
// that stores Option<Yoke<Y, C>> in the same size as Yoke<Y, C> by using a
// sentinel pointer value for None. oyakodon has no equivalent type and no
// analogous size optimization to test.
// Affected tests: test_sizes, test_new_sentinel, test_new_rc, test_rc_clone.

// ================================================================================
// yoke/tests/bincode.rs
// ================================================================================

// NOT MIGRATABLE as written: the original test uses the bincode crate to
// deserialize bytes into Cow<'_, str> and Cow<'_, [u8]> fields that borrow from
// the cart. It also requires unsafe impl Yokeable for the view struct, a
// yoke-specific trait with no equivalent in oyakodon.
// Adding bincode as a dev-dependency solely for this test is not warranted.
//
// The core behavior being tested — a view that initially borrows from the owner
// and transitions to owned on mutation — is demonstrated below using only std.

#[test]
fn borrowed_then_mutated() {
    fn make_cow(s: &mut String) -> Cow<'_, str> {
        Cow::Borrowed(s.as_str())
    }

    let mut bowl = BowlMut::new(Box::new("hello".to_owned()), make_cow);
    bowl.spawn(|v: &Cow<'_, str>| assert!(matches!(*v, Cow::Borrowed(_))));
    bowl.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "hello"));

    bowl.spawn_mut(|v: &mut Cow<'_, str>| v.to_mut().push_str(" world"));
    bowl.spawn(|v: &Cow<'_, str>| assert!(matches!(*v, Cow::Owned(_))));
    bowl.spawn(|v: &Cow<'_, str>| assert_eq!(v.as_ref(), "hello world"));
}
