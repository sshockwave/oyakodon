use oyakodon::{Bowl, BowlMut, BowlRef, View};

/// Regression: [`into_inner()`] must drop `base` even when `derived`'s drop panics.
/// Previously, `derived: _` in the destructure produced an unnamed temporary
/// that Rust's unwind machinery did not track,
/// causing `base` to leak when `derived`'s drop panicked.
/// Fixed by using a named binding so `base` is a proper tracked local.
///
/// [`into_inner()`]: BowlRef::into_inner
#[test]
#[should_panic]
fn into_inner_drops_base_on_derived_panic() {
    #[allow(dead_code)]
    struct PanicOnDrop<'a>(&'a String);
    impl Drop for PanicOnDrop<'_> {
        fn drop(&mut self) {
            panic!("derived drop panic");
        }
    }
    fn make(s: &String) -> PanicOnDrop<'_> {
        PanicOnDrop(s)
    }
    // Miri detects the leak if `Box<String>` is not freed after the panic.
    BowlRef::new(Box::new(String::from("hello")), make).into_inner();
}

/// The same as [`into_inner_drops_base_on_derived_panic`] but for [`BowlRef::into_view()`]
#[test]
#[should_panic]
fn into_view_drops_view_on_base_panic() {
    struct PanicOnDrop(String);
    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("base drop panic");
        }
    }
    fn make_view(owner: &PanicOnDrop) -> Box<String> {
        Box::new(owner.0.clone())
    }
    // Miri detects the leak if Box<String> (the view) is not freed after the panic.
    let _: Box<String> = BowlRef::<Box<PanicOnDrop>, fn(&PanicOnDrop) -> Box<String>>::new(
        Box::new(PanicOnDrop("hello".to_string())),
        make_view,
    )
    .into_view();
}

/// https://github.com/someguynamedjosh/ouroboros/issues/88
/// The issue states that Miri requires all parameters to a function
/// must be valid throughout the entire function body,
/// which is not the case for `Drop::drop`.
/// If the problem occurs in this crate,
/// it should be visible immediately when we run any test with Miri.
#[test]
fn ouroboros_88() {}

/// https://github.com/unicode-org/icu4x/issues/3696
/// Should be run with `-Zmiri-retag-fields` though enabled in new Miri versions by default.
#[test]
fn yoke_3696() {
    struct GetRef;
    impl<'a> View<&'a mut [u8]> for GetRef {
        type Output = &'a mut [u8];
    }
    fn example(_: BowlMut<'_, Vec<u8>, GetRef>) {}
    example(BowlMut::<_, GetRef>::from_fn(vec![0, 1, 2], &|data| data));
}

/// https://github.com/Kimundi/owning-ref-rs/issues/49
/// We don't have this problem by always accessing the owner
/// through the reference in the derived view,
/// so we will not trigger the `noalias` attribute on the owner.
/// Hence the test seems a bit trivial.
#[test]
fn owning_ref_49() {
    use std::cell::Cell;

    fn derive(cell: &mut Cell<u8>) -> &Cell<u8> {
        &*cell
    }

    fn helper(owning_ref: &impl for<'a> Bowl<Value<'a> = &'a Cell<u8>>) -> u8 {
        owning_ref.get().set(10);
        owning_ref.get().set(20);
        owning_ref.get().get() // should return 20
    }

    let val: Box<Cell<u8>> = Box::new(Cell::new(25));
    let owning_ref = BowlMut::new(val, derive);
    let res = helper(&owning_ref);
    assert_eq!(res, 20);

    // Extra test to ensure that the base value is correct
    let base = owning_ref.into_inner();
    assert_eq!(base.get(), 20);
}
