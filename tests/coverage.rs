// Exercises parts of the public API not covered by other test files.
// Organized by type, then by method or trait impl.

use oyakodon::{BowlBox, BowlMut, BowlRef};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
};

fn hash_of<H: Hash>(val: &H) -> u64 {
    let mut h = DefaultHasher::new();
    val.hash(&mut h);
    h.finish()
}

// Named functions avoid HRTB annotation issues that closures cannot express.
fn to_len(s: &String) -> usize {
    s.len()
}
fn to_str(s: &String) -> &str {
    s.as_str()
}
fn len_of_str(s: &str) -> usize {
    s.len()
}
fn str_len_mut(s: &mut String) -> usize {
    s.len()
}
fn identity_i32_mut(x: &mut i32) -> &mut i32 {
    x
}
fn deref_i32(x: &mut i32) -> i32 {
    *x
}
fn double_i32(x: i32) -> i32 {
    x * 2
}
fn identity_usize(x: usize) -> usize {
    x
}

// ================================================================================
// BowlRef
// ================================================================================

// --- from_derive ----------------------------------------------------------------
// `new` delegates here; test the method directly to cover its own code path.
#[rustversion::since(1.78)]
#[test]
fn bowl_ref_from_derive() {
    let bowl: BowlRef<'_, Box<String>, fn(&String) -> usize> =
        BowlRef::from_derive(Box::new("hello".to_owned()), to_len);
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- from_fn_mut ----------------------------------------------------------------
#[test]
fn bowl_ref_from_fn_mut() {
    let bowl: BowlRef<'_, Box<String>, fn(&String) -> usize> =
        BowlRef::from_fn_mut(Box::new("hello".to_owned()), &mut |s: &String| s.len());
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- from_fn_once ---------------------------------------------------------------
#[test]
fn bowl_ref_from_fn_once() {
    let bowl: BowlRef<'_, Box<String>, fn(&String) -> usize> =
        BowlRef::from_fn_once(Box::new("hello".to_owned()), Box::new(|s: &String| s.len()));
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- map -----------------------------------------------------------------------
#[test]
fn bowl_ref_map() {
    let bowl = BowlRef::new(Box::new("hello".to_owned()), to_str);
    let mapped = bowl.map(len_of_str);
    assert_eq!(mapped.spawn(|v: &_| *v), 5);
}

// --- cast -----------------------------------------------------------------------
// Combines cast_life and cast_view; here only the lifetime changes.
#[test]
fn bowl_ref_cast() {
    let bowl: BowlRef<'_, Box<String>, fn(&String) -> usize> =
        BowlRef::new(Box::new("hello".to_owned()), to_len);
    let casted: BowlRef<'static, Box<String>, fn(&String) -> usize> = bowl.cast();
    assert_eq!(casted.spawn(|v: &_| *v), 5);
}

// --- into_view -----------------------------------------------------------------
// Requires a lifetime-independent view output (usize does not borrow from the owner).
#[test]
fn bowl_ref_into_view() {
    let bowl = BowlRef::new(Box::new("hello".to_owned()), to_len);
    let len: usize = bowl.into_view();
    assert_eq!(len, 5);
}

// --- into_parts ----------------------------------------------------------------
#[test]
fn bowl_ref_into_parts() {
    let bowl = BowlRef::new(Box::new("hello".to_owned()), to_len);
    let (owner, len): (Box<String>, usize) = bowl.into_parts();
    assert_eq!(*owner, "hello");
    assert_eq!(len, 5);
}

// --- Hash ----------------------------------------------------------------------
// Hash over both owner and view; equal owners and views produce equal hashes.
#[test]
fn bowl_ref_hash() {
    let a = BowlRef::new(Rc::new("hello".to_owned()), to_len).cast_life::<'static>();
    let b = BowlRef::new(Rc::new("hello".to_owned()), to_len).cast_life::<'static>();
    let c = BowlRef::new(Rc::new("world".to_owned()), to_len).cast_life::<'static>();
    assert_eq!(hash_of(&a), hash_of(&b));
    assert_ne!(hash_of(&a), hash_of(&c));
}

// --- AsRef / AsMut -------------------------------------------------------------
// These impls allow re-borrowing with a different lifetime placeholder.
#[test]
fn bowl_ref_as_ref_as_mut() {
    let mut bowl: BowlRef<'_, Box<String>, fn(&String) -> usize> =
        BowlRef::new(Box::new("hello".to_owned()), to_len);
    let r: &BowlRef<'static, Box<String>, fn(&String) -> usize> = bowl.as_ref();
    assert_eq!(r.spawn(|v: &_| *v), 5);
    let m: &mut BowlRef<'static, Box<String>, fn(&String) -> usize> = bowl.as_mut();
    assert_eq!(m.spawn(|v: &_| *v), 5);
}

// --- Bowl trait ----------------------------------------------------------------
#[test]
fn bowl_ref_bowl_trait() {
    let bowl = BowlRef::new(Box::new("hello".to_owned()), to_len);
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// ================================================================================
// BowlMut
// ================================================================================

// --- from_fn -------------------------------------------------------------------
#[test]
fn bowl_mut_from_fn() {
    let bowl: BowlMut<'_, Box<String>, fn(&mut String) -> usize> =
        BowlMut::from_fn(Box::new("hello".to_owned()), &|s: &mut String| s.len());
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- from_fn_mut ---------------------------------------------------------------
#[test]
fn bowl_mut_from_fn_mut() {
    let bowl: BowlMut<'_, Box<String>, fn(&mut String) -> usize> =
        BowlMut::from_fn_mut(Box::new("hello".to_owned()), &mut |s: &mut String| s.len());
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- from_fn_once --------------------------------------------------------------
#[test]
fn bowl_mut_from_fn_once() {
    let bowl: BowlMut<'_, Box<String>, fn(&mut String) -> usize> = BowlMut::from_fn_once(
        Box::new("hello".to_owned()),
        Box::new(|s: &mut String| s.len()),
    );
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- map -----------------------------------------------------------------------
#[test]
fn bowl_mut_map() {
    let bowl = BowlMut::new(Box::new(21i32), deref_i32);
    let mapped = bowl.map(double_i32);
    assert_eq!(mapped.spawn(|v: &_| *v), 42);
}

// --- cast_life -----------------------------------------------------------------
#[test]
fn bowl_mut_cast_life() {
    let bowl: BowlMut<'_, Box<String>, fn(&mut String) -> usize> =
        BowlMut::new(Box::new("hello".to_owned()), str_len_mut);
    let casted: BowlMut<'static, Box<String>, fn(&mut String) -> usize> = bowl.cast_life();
    assert_eq!(casted.spawn(|v: &_| *v), 5);
}

// --- cast_view -----------------------------------------------------------------
// Map applies an identity usize → usize transformation, yielding a Map<...> marker,
// then cast_view restores the original fn(&mut String) -> usize marker.
#[test]
fn bowl_mut_cast_view() {
    let bowl = BowlMut::new(Box::new("hello".to_owned()), str_len_mut);
    let mapped = bowl.map(identity_usize);
    let casted: BowlMut<'_, Box<String>, fn(&mut String) -> usize> = mapped.cast_view();
    assert_eq!(casted.spawn(|v: &_| *v), 5);
    drop(casted); // suppress unused-variable warning
}

// --- into_view -----------------------------------------------------------------
#[test]
fn bowl_mut_into_view() {
    let bowl = BowlMut::new(Box::new("hello".to_owned()), str_len_mut);
    let len: usize = bowl.into_view();
    assert_eq!(len, 5);
}

// --- into_parts ----------------------------------------------------------------
#[test]
fn bowl_mut_into_parts() {
    let bowl = BowlMut::new(Box::new("hello".to_owned()), str_len_mut);
    let (owner, len): (Box<String>, usize) = bowl.into_parts();
    assert_eq!(*owner, "hello");
    assert_eq!(len, 5);
}

// --- into_async ----------------------------------------------------------------
#[cfg_attr(miri, ignore)]
#[test]
fn bowl_mut_into_async() {
    fn get_ready(x: &mut i32) -> std::future::Ready<i32> {
        std::future::ready(*x)
    }
    let bowl = smol::block_on(BowlMut::new(Box::new(42i32), get_ready).into_async());
    assert_eq!(bowl.spawn(|v: &_| *v), 42);
}

// --- into_result ---------------------------------------------------------------
#[test]
fn bowl_mut_into_result() {
    fn try_parse(s: &mut String) -> Result<i32, ()> {
        s.parse().map_err(|_| ())
    }
    let ok = BowlMut::new(Box::new("42".to_owned()), try_parse)
        .into_result()
        .unwrap();
    assert_eq!(ok.spawn(|v: &_| *v), 42);

    let err = BowlMut::new(Box::new("bad".to_owned()), try_parse)
        .into_result()
        .unwrap_err();
    assert_eq!(err.spawn(|v: &_| *v), ());
}

// --- Debug ---------------------------------------------------------------------
#[test]
fn bowl_mut_debug() {
    let bowl = BowlMut::new(Box::new(42i32), identity_i32_mut);
    let s = format!("{bowl:?}");
    assert!(s.contains("42"));
}

// --- Sync ----------------------------------------------------------------------
// BowlMut does not require T: Sync; only the view output need be Sync.
#[test]
fn bowl_mut_sync() {
    fn assert_sync<T: Sync>(_: &T) {}
    let bowl = BowlMut::new(Box::new(42i32), identity_i32_mut);
    assert_sync(&bowl);
}

// --- From<BowlRef> -------------------------------------------------------------
// A BowlRef can be widened to a BowlMut when the view type works with &mut as well.
#[test]
fn bowl_mut_from_bowl_ref() {
    let ref_bowl: BowlRef<'_, Box<String>, fn(&String) -> usize> =
        BowlRef::new(Box::new("hello".to_owned()), to_len);
    let mut_bowl: BowlMut<'_, Box<String>, fn(&mut String) -> usize> = BowlMut::from(ref_bowl);
    assert_eq!(mut_bowl.spawn(|v: &_| *v), 5);
}

// --- AsRef / AsMut -------------------------------------------------------------
#[test]
fn bowl_mut_as_ref_as_mut() {
    let mut bowl: BowlMut<'_, Box<String>, fn(&mut String) -> usize> =
        BowlMut::new(Box::new("hello".to_owned()), str_len_mut);
    let r: &BowlMut<'static, Box<String>, fn(&mut String) -> usize> = bowl.as_ref();
    assert_eq!(r.spawn(|v: &_| *v), 5);
    let m: &mut BowlMut<'static, Box<String>, fn(&mut String) -> usize> = bowl.as_mut();
    assert_eq!(m.spawn(|v: &_| *v), 5);
}

// --- Bowl trait ----------------------------------------------------------------
#[test]
fn bowl_mut_bowl_trait() {
    let mut bowl = BowlMut::new(Box::new("hello".to_owned()), str_len_mut);
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
    bowl.spawn_mut(|v: &mut _| {
        *v = 99;
    });
    assert_eq!(bowl.spawn(|v: &_| *v), 99);
}

// ================================================================================
// BowlBox
// ================================================================================

// --- from_derive ----------------------------------------------------------------
#[rustversion::since(1.78)]
#[test]
fn bowl_box_from_derive() {
    let bowl: BowlBox<'_, String, fn(&mut String) -> usize> =
        BowlBox::from_derive("hello".to_owned(), str_len_mut);
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- from_fn -------------------------------------------------------------------
#[test]
fn bowl_box_from_fn() {
    let bowl: BowlBox<'_, String, fn(&mut String) -> usize> =
        BowlBox::from_fn("hello".to_owned(), &|s: &mut String| s.len());
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- from_fn_mut ---------------------------------------------------------------
#[test]
fn bowl_box_from_fn_mut() {
    let bowl: BowlBox<'_, String, fn(&mut String) -> usize> =
        BowlBox::from_fn_mut("hello".to_owned(), &mut |s: &mut String| s.len());
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- from_fn_once --------------------------------------------------------------
#[test]
fn bowl_box_from_fn_once() {
    let bowl: BowlBox<'_, String, fn(&mut String) -> usize> =
        BowlBox::from_fn_once("hello".to_owned(), Box::new(|s: &mut String| s.len()));
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
}

// --- map -----------------------------------------------------------------------
#[test]
fn bowl_box_map() {
    let bowl = BowlBox::new(21i32, deref_i32);
    let mapped = bowl.map(double_i32);
    assert_eq!(mapped.spawn(|v: &_| *v), 42);
}

// --- cast_life -----------------------------------------------------------------
#[test]
fn bowl_box_cast_life() {
    let bowl: BowlBox<'_, String, fn(&mut String) -> usize> =
        BowlBox::new("hello".to_owned(), str_len_mut);
    let casted: BowlBox<'static, String, fn(&mut String) -> usize> = bowl.cast_life();
    assert_eq!(casted.spawn(|v: &_| *v), 5);
}

// --- cast_view -----------------------------------------------------------------
#[test]
fn bowl_box_cast_view() {
    let bowl = BowlBox::new("hello".to_owned(), str_len_mut);
    let mapped = bowl.map(identity_usize);
    let casted: BowlBox<'_, String, fn(&mut String) -> usize> = mapped.cast_view();
    assert_eq!(casted.spawn(|v: &_| *v), 5);
    drop(casted);
}

// --- into_view -----------------------------------------------------------------
#[test]
fn bowl_box_into_view() {
    let bowl = BowlBox::new("hello".to_owned(), str_len_mut);
    let len: usize = bowl.into_view();
    assert_eq!(len, 5);
}

// --- into_parts ----------------------------------------------------------------
// BowlBox::into_parts unboxes the owner, unlike BowlMut::into_parts.
#[test]
fn bowl_box_into_parts() {
    let bowl = BowlBox::new("hello".to_owned(), str_len_mut);
    let (owner, len): (String, usize) = bowl.into_parts();
    assert_eq!(owner, "hello");
    assert_eq!(len, 5);
}

// --- into_async ----------------------------------------------------------------
#[cfg_attr(miri, ignore)]
#[test]
fn bowl_box_into_async() {
    fn get_ready(x: &mut i32) -> std::future::Ready<i32> {
        std::future::ready(*x)
    }
    let bowl = smol::block_on(BowlBox::new(42i32, get_ready).into_async());
    assert_eq!(bowl.spawn(|v: &_| *v), 42);
}

// --- into_result ---------------------------------------------------------------
#[test]
fn bowl_box_into_result() {
    fn try_parse(s: &mut String) -> Result<i32, ()> {
        s.parse().map_err(|_| ())
    }
    let ok = BowlBox::new("42".to_owned(), try_parse)
        .into_result()
        .unwrap();
    assert_eq!(ok.spawn(|v: &_| *v), 42);

    let err = BowlBox::new("bad".to_owned(), try_parse)
        .into_result()
        .unwrap_err();
    assert_eq!(err.spawn(|v: &_| *v), ());
}

// --- Debug ---------------------------------------------------------------------
#[test]
fn bowl_box_debug() {
    let bowl = BowlBox::new(42i32, identity_i32_mut);
    let s = format!("{bowl:?}");
    assert!(s.contains("42"));
}

// --- From / Into with BowlMut --------------------------------------------------
// BowlBox and BowlMut<Box<T>, F> are interconvertible.
#[test]
fn bowl_box_from_into_bowl_mut() {
    let box_bowl: BowlBox<'_, String, fn(&mut String) -> usize> =
        BowlBox::new("hello".to_owned(), str_len_mut);
    let mut_bowl: BowlMut<'_, Box<String>, fn(&mut String) -> usize> = BowlMut::from(box_bowl);
    assert_eq!(mut_bowl.spawn(|v: &_| *v), 5);
    let box_bowl2: BowlBox<'_, String, fn(&mut String) -> usize> = BowlBox::from(mut_bowl);
    assert_eq!(box_bowl2.spawn(|v: &_| *v), 5);
}

// --- AsRef / AsMut (self and underlying BowlMut) -------------------------------
#[test]
fn bowl_box_as_ref_as_mut() {
    let mut bowl: BowlBox<'_, String, fn(&mut String) -> usize> =
        BowlBox::new("hello".to_owned(), str_len_mut);

    // AsRef<BowlBox<'b, T, F>>
    let r: &BowlBox<'static, String, fn(&mut String) -> usize> = bowl.as_ref();
    assert_eq!(r.spawn(|v: &_| *v), 5);

    // AsRef<BowlMut<'b, Box<T>, F>>
    let rm: &BowlMut<'static, Box<String>, fn(&mut String) -> usize> = bowl.as_ref();
    assert_eq!(rm.spawn(|v: &_| *v), 5);

    // AsMut<BowlBox<'b, T, F>>
    let m: &mut BowlBox<'static, String, fn(&mut String) -> usize> = bowl.as_mut();
    assert_eq!(m.spawn(|v: &_| *v), 5);

    // AsMut<BowlMut<'b, Box<T>, F>>
    let mm: &mut BowlMut<'static, Box<String>, fn(&mut String) -> usize> = bowl.as_mut();
    assert_eq!(mm.spawn(|v: &_| *v), 5);
}

// --- Bowl trait ----------------------------------------------------------------
#[test]
fn bowl_box_bowl_trait() {
    let mut bowl = BowlBox::new("hello".to_owned(), str_len_mut);
    assert_eq!(bowl.spawn(|v: &_| *v), 5);
    bowl.spawn_mut(|v: &mut _| {
        *v = 99;
    });
    assert_eq!(bowl.spawn(|v: &_| *v), 99);
}
