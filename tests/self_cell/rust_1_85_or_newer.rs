// Tests migrated from self_cell/tests-extra/rust_1_85_or_newer/src/lib.rs.
// Kept in the same order as the original so missing entries are easy to spot.
//
// Mapping: self_cell's async_builder generates an async *constructor*,
// while oyakodon models async by storing a Future as the view and calling into_async().
// The semantic result is equivalent: the cell/bowl is created asynchronously
// and the dependent/view holds the resolved value afterwards.
//
// All async tests are ignored under Miri: smol unconditionally spawns a background
// I/O thread that calls `timerfd_create`, a Linux syscall Miri does not support.
// The memory-safety properties exercised by these tests are already covered by the
// synchronous tests above.

use oyakodon::BowlBox;

const OWNER_STR: &str = "some longer string xxx with even more chars";
const CAPTURE_IDX: usize = 33;

// --- async_self_cell ------------------------------------------------------------

fn view_slice(s: &mut String) -> std::future::Ready<&str> {
    std::future::ready(&s[CAPTURE_IDX..])
}

#[test]
#[cfg_attr(miri, ignore)]
fn async_bowl() {
    let bowl = smol::block_on(async {
        BowlBox::new(OWNER_STR.to_string(), view_slice)
            .into_async()
            .await
    });
    assert_eq!(*bowl.get(), "more chars");
}

// --- async_self_cell_try_new ----------------------------------------------------

fn view_slice_ok(s: &mut String) -> Result<std::future::Ready<&str>, ()> {
    Ok(std::future::ready(&s[CAPTURE_IDX..]))
}

#[test]
#[cfg_attr(miri, ignore)]
fn async_bowl_try_new() {
    let bowl = smol::block_on(async {
        BowlBox::new(OWNER_STR.to_string(), view_slice_ok)
            .into_result()
            .unwrap()
            .into_async()
            .await
    });
    assert_eq!(*bowl.get(), "more chars");
}

// --- async_self_cell_try_new_or_recover -----------------------------------------

#[test]
#[cfg_attr(miri, ignore)]
fn async_bowl_try_new_or_recover() {
    let bowl = smol::block_on(async {
        BowlBox::new(OWNER_STR.to_string(), view_slice_ok)
            .into_result()
            .unwrap()
            .into_async()
            .await
    });
    assert_eq!(*bowl.get(), "more chars");
}

// --- async_self_cell_recover ----------------------------------------------------
// Note: recovering the base is not possible in oyakodon (no into_base() after into_result()).
// We verify that the Err branch contains the expected error value.

fn view_err(s: &mut String) -> Result<std::future::Ready<&str>, usize> {
    Err(s.len())
}

#[test]
#[cfg_attr(miri, ignore)]
fn async_bowl_recover() {
    let err = BowlBox::new(OWNER_STR.to_string(), view_err)
        .into_result()
        .unwrap_err()
        .into_view();
    assert_eq!(err, OWNER_STR.len());
}

// --- async_self_cell_with_sleep -------------------------------------------------

fn view_slice_after_sleep(
    s: &mut String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = &str> + '_>> {
    let slice = &s[CAPTURE_IDX..];
    Box::pin(async move {
        smol::Timer::after(std::time::Duration::from_millis(100)).await;
        slice
    })
}

#[test]
#[cfg_attr(miri, ignore)]
fn async_bowl_with_sleep() {
    let bowl = smol::block_on(async {
        BowlBox::new(OWNER_STR.to_string(), view_slice_after_sleep)
            .into_async()
            .await
    });
    assert_eq!(*bowl.get(), "more chars");
}

// --- async_self_cell_with_mut_borrow --------------------------------------------

#[test]
#[cfg_attr(miri, ignore)]
fn async_bowl_with_mut_borrow() {
    fn view_len(s: &mut String) -> std::future::Ready<usize> {
        std::future::ready(s.len())
    }

    let mut bowl = smol::block_on(async {
        BowlBox::new(OWNER_STR.to_string(), view_len)
            .into_async()
            .await
    });
    *bowl.get_mut() += 1;
    assert_eq!(*bowl.get(), OWNER_STR.len() + 1);
}
