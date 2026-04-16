// Tests migrated from ouroboros/examples/src/ok_tests.rs.
// Kept in source order so missing entries are easy to spot.
//
// API mapping:
//   Builder { data, dref_builder: |data| data }.build()
//     → BowlRef::new(owner, derive)  /  BowlBox::new(owner, derive)
//   bar.with_dref(|dref| *dref)      → *bowl.get()
//   bar.borrow_dref()                → bowl.get()
//   bar.with_dref_mut(|dref| ...)    → *bowl.get_mut() = ...
//   bar.into_heads().data            → bowl.into_owner()
//   TryBuilder { ... }.try_build()   → BowlRef::new(...).into_result()
//   .try_build_or_recover()          → .into_result() then .unwrap_err().into_parts()

use oyakodon::{BowlBox, BowlMut, BowlRef};

// Shared helpers.
fn get_ref(data: &i32) -> &i32 {
    data
}
fn get_mut_ref(data: &mut i32) -> &mut i32 {
    data
}
fn try_get_ref_err(_data: &i32) -> Result<&i32, i32> {
    Err(56)
}

// --- box_and_ref ----------------------------------------------------------------

#[test]
fn box_and_ref() {
    let bowl = BowlRef::new(Box::new(12i32), get_ref);
    assert_eq!(**bowl.get(), 12);
    drop(bowl);
}

// --- async_new ------------------------------------------------------------------
// ouroboros uses tokio; we use smol.
// All async tests are ignored under Miri: smol unconditionally spawns a background
// I/O thread that calls `timerfd_create`, a Linux syscall Miri does not support.

#[cfg_attr(miri, ignore)]
#[test]
fn async_new() {
    fn get_ready(data: &i32) -> std::future::Ready<&i32> {
        std::future::ready(data)
    }
    let bowl = smol::block_on(BowlRef::new(Box::new(12i32), get_ready).into_async());
    assert_eq!(**bowl.get(), 12);
}

// --- async_try_new --------------------------------------------------------------

#[cfg_attr(miri, ignore)]
#[test]
fn async_try_new() {
    fn get_ready_ok(data: &i32) -> std::future::Ready<Result<&i32, ()>> {
        std::future::ready(Ok(data))
    }
    let bowl = smol::block_on(BowlRef::new(Box::new(12i32), get_ready_ok).into_async())
        .into_result()
        .unwrap();
    assert_eq!(**bowl.get(), 12);
}

// --- async_try_new_err ----------------------------------------------------------

#[cfg_attr(miri, ignore)]
#[test]
fn async_try_new_err() {
    fn get_ready_err(_data: &i32) -> std::future::Ready<Result<&i32, u64>> {
        std::future::ready(Err(56u64))
    }
    let err_bowl = smol::block_on(BowlRef::new(Box::new(12i32), get_ready_err).into_async())
        .into_result()
        .unwrap_err();
    assert_eq!(*err_bowl.get(), 56u64);
}

// --- try_new --------------------------------------------------------------------

fn try_get_ref_ok(data: &i32) -> Result<&i32, ()> {
    Ok(data)
}

#[test]
fn try_new() {
    let bowl = BowlRef::new(Box::new(12i32), try_get_ref_ok)
        .into_result()
        .unwrap();
    assert_eq!(**bowl.get(), 12);
}

// --- try_new_err ----------------------------------------------------------------

#[test]
fn try_new_err() {
    let err_bowl = BowlRef::new(Box::new(12i32), try_get_ref_err)
        .into_result()
        .unwrap_err();
    assert_eq!(*err_bowl.get(), 56);
}

// --- try_new_recover_heads ------------------------------------------------------
// into_result() always runs derive and stores the owner regardless of Ok or Err.
// On Err, use into_parts() to recover both the owner and the error value at once.
// Semantic difference from ouroboros: try_build_or_recover() skips storage on Err,
// so into_result() always incurs the derive cost even on the error path.

#[test]
fn try_new_recover_heads() {
    let err_bowl = BowlRef::new(Box::new(12i32), try_get_ref_err)
        .into_result()
        .unwrap_err();
    let (owner, err) = err_bowl.into_parts();
    assert_eq!(*owner, 12);
    assert_eq!(err, 56);
}

// --- into_heads -----------------------------------------------------------------

#[test]
fn into_heads() {
    let bowl = BowlRef::new(Box::new(12i32), get_ref);
    assert_eq!(*bowl.into_owner(), 12);
}

// --- box_and_mut_ref ------------------------------------------------------------

#[test]
fn box_and_mut_ref() {
    let mut bowl = BowlMut::new(Box::new(12i32), get_mut_ref);
    assert_eq!(**bowl.get(), 12);
    **bowl.get_mut() = 34;
    assert_eq!(**bowl.get(), 34);
}

// --- self_reference_with --------------------------------------------------------
// NOT MIGRATABLE: ouroboros' with() and with_mut() pass simultaneous (&owner, &dep)
// or (&owner, &mut dep) to the closure. oyakodon never exposes the owner after
// construction; only the view is accessible. The borrow_dref() and with_mut()
// parts are equivalent to get() and get_mut(), but the simultaneous owner access
// in with_mut(|fields| { *fields.dref = fields.data; }) has no equivalent.

// --- single_lifetime ------------------------------------------------------------
// The owner is an external &str reference; T = &str, T::Target = str.
// borrow_external() has no equivalent: oyakodon does not expose the owner.

#[test]
fn single_lifetime() {
    let external = "Hello world!".to_owned();
    fn identity(s: &str) -> &str {
        s
    }
    let bowl = BowlRef::new(&external[..], identity);
    let _ = bowl.get();
    drop(bowl);
}

// --- double_lifetime ------------------------------------------------------------
// NOT MIGRATABLE: the original test has no body; it only verifies that a
// macro-generated struct with two external lifetime parameters compiles.
// oyakodon does not generate structs; multiple external lifetimes are naturally
// supported by generic type parameters and require no dedicated test.

// --- custom_ref -----------------------------------------------------------------
// ouroboros uses with_phrase_mut(|phrase| phrase.change_phrase()) and then
// into_heads() to recover the data. oyakodon uses get_mut() and into_owner().

struct PhraseRef<'a> {
    data: &'a mut String,
}

impl PhraseRef<'_> {
    fn change_phrase(&mut self) {
        *self.data = self.data.replace("Hello", "Goodbye");
    }
}

fn make_phrase_ref(data: &mut String) -> PhraseRef<'_> {
    PhraseRef { data }
}

#[test]
fn custom_ref() {
    let mut bowl = BowlBox::new("Hello world!".to_owned(), make_phrase_ref);
    bowl.get_mut().change_phrase();
    assert_eq!(bowl.into_owner(), "Goodbye world!");
}

// --- compile_tests --------------------------------------------------------------
// NOT MIGRATABLE: uses trybuild to assert that specific code patterns fail to compile.
// oyakodon's safety relies on the type system and MaybeDangling rather than on
// proc-macro restrictions, so there are no analogous compile-fail cases to test.

// --- test_hygiene ---------------------------------------------------------------
// NOT MIGRATABLE: tests that the ouroboros! macro works when standard Rust names
// (std, core, Result, Drop, Fn, etc.) are shadowed in the local scope. oyakodon
// has no macros; name shadowing in user code is not a concern.
