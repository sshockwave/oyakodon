use oyakodon::primitive::{Bowl, View};

/// Regression: [`into_owner()`] must drop `owner` even when `view`'s drop panics.
/// Previously, `view: _` in the destructure produced an unnamed temporary
/// that Rust's unwind machinery did not track,
/// causing `owner` to leak when `view`'s drop panicked.
/// Fixed by using a named binding so `owner` is a proper tracked local.
///
/// [`into_owner()`]: Bowl::into_owner
#[test]
#[should_panic]
fn into_owner_drops_owner_on_view_panic() {
    #[allow(dead_code)]
    struct PanicOnDrop<'a>(&'a String);
    impl Drop for PanicOnDrop<'_> {
        fn drop(&mut self) {
            panic!("view drop panic");
        }
    }
    fn make(s: &mut String) -> PanicOnDrop<'_> {
        PanicOnDrop(s)
    }
    // Miri detects the leak if `Box<String>` is not freed after the panic.
    Bowl::new_box(String::from("hello"))
        .map_view(make)
        .into_owner();
}

/// The same as [`into_owner_drops_owner_on_view_panic`] but for [`Bowl::into_view()`].
#[test]
#[should_panic]
fn into_view_drops_view_on_owner_panic() {
    struct PanicOnDrop(String);
    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("owner drop panic");
        }
    }
    fn make_view(owner: &mut PanicOnDrop) -> Box<String> {
        Box::new(owner.0.clone())
    }
    // Miri detects the leak if Box<String> (the view) is not freed after the panic.
    let _: Box<String> = Bowl::new_box(PanicOnDrop("hello".to_string()))
        .map_view(make_view)
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
    fn example<P>(_: Bowl<P, dyn for<'x> View<'x, Output = &'x mut [u8]>>) {}
    let bowl = Bowl::new_box(vec![0u8, 1, 2]).map(|view, slot| {
        let view = view.map(&slot, |view, stamp| {
            stamp.stamp::<dyn for<'x> View<'x, Output = &'x mut [u8]>>(view)
        });
        slot.unseal().fill(view)
    });
    example(bowl);
}

/// https://github.com/Kimundi/owning-ref-rs/issues/49
/// We don't have this problem by always accessing the owner
/// through the reference in the derived view,
/// so we will not trigger the `noalias` attribute on the owner.
/// Hence the test seems a bit trivial.
#[test]
fn owning_ref_49() {
    use std::cell::Cell;

    let owning_ref = Bowl::new_box(Cell::new(25u8));
    let res = owning_ref.with(|v, _| {
        v.set(10);
        v.set(20);
        v.get()
    });
    assert_eq!(res, 20);

    // Extra test to ensure that the owner value is correct
    let owner = owning_ref.into_owner();
    assert_eq!(owner.get(), 20);
}
