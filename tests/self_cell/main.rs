// Tests migrated from self_cell/tests/self_cell.rs and self_cell/tests-extra/.
// Kept in the same order as the originals so missing entries are easy to spot.
//
// API mapping:
//   self_cell::new(owner, |o| dep)          → BowlRef::new(base, derive)
//   self_cell::borrow_dependent()           → BowlRef::get()
//   self_cell::borrow_owner()               → no equivalent; oyakodon does not expose the base
//   self_cell::with_dependent(|owner, dep|) → no equivalent; requires simultaneous owner + dep access
//   self_cell::into_owner()                 → BowlRef::into_inner()
//   self_cell::with_dependent_mut()         → BowlMut::get_mut()

mod no_std_lib;
mod rust_1_85_or_newer;
mod tests_extra;

use oyakodon::{BowlMut, BowlRef, Derive, View};
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

// ================================================================================
// self_cell/tests/self_cell.rs
// ================================================================================

// --- parse_ast ------------------------------------------------------------------
// Clone is tested via Rc<String> (CloneStableDeref); plain String is not CloneStableDeref.
// borrow_owner() has no equivalent; only the derived view is checked.

#[derive(Clone, Debug, PartialEq)]
struct Ast<'a>(Vec<&'a str>);

struct MakeAst;
impl<'a> View<&'a String> for MakeAst {
    type Output = Ast<'a>;
}
impl<'a> Derive<&'a String> for MakeAst {
    fn call(self, s: &'a String) -> Ast<'a> {
        Ast(vec![&s[2..5], &s[1..3]])
    }
}

#[test]
fn parse_ast() {
    let body = String::from("some longer string that ends now");
    let expected_ast = MakeAst.call(&body);

    let bowl = BowlRef::<Rc<String>, MakeAst>::new(Rc::new(body.clone()), MakeAst);
    assert_eq!(*bowl.get(), expected_ast);

    let moved = bowl;
    assert_eq!(*moved.get(), expected_ast);

    let cloned = moved.clone();
    drop(moved);
    assert_eq!(*cloned.get(), expected_ast);
}

// --- return_self_ref_struct -----------------------------------------------------

fn make_ast_stripped(body: &str) -> BowlRef<'_, Box<String>, fn(&String) -> &str> {
    BowlRef::new(Box::new(body.replace('\n', "")), String::as_str)
}

#[test]
fn return_self_ref_struct() {
    let bowl = make_ast_stripped("a\nb\nc\ndef");
    assert_eq!(*bowl.get(), "abcdef");
}

// --- failable_constructor_success -----------------------------------------------
// NOT MIGRATABLE: oyakodon has no fallible constructor (try_new).
// The caller must validate the base before calling new().

// --- failable_constructor_fail --------------------------------------------------
// NOT MIGRATABLE: same as above.

// --- from_fn --------------------------------------------------------------------
// Cell<usize> is used for the counter so the closure borrows it immutably (&self),
// avoiding a conflict with reading it while `bowl` still holds the closure.

#[test]
fn from_fn() {
    let call_count = Cell::new(0usize);
    let bowl = BowlRef::new(Box::new(String::from("hello")), |s: &String| {
        call_count.set(call_count.get() + 1);
        s.len()
    });
    assert_eq!(call_count.get(), 1);
    assert_eq!(*bowl.get(), 5);
    assert_eq!(call_count.get(), 1); // get() does not re-invoke the derive function
}

// --- catch_panic_in_from --------------------------------------------------------
// NOT MIGRATABLE: the pattern specifically uses try_new to catch a panic via
// catch_unwind and return Err, ensuring the owner is freed.
// The no-leak property itself is covered by panic_in_from_owner below.

// --- no_derive_owner_type -------------------------------------------------------
// A named fn is used because closures do not satisfy for<'a> Fn(&'a T) → &'a U
// without HRTB inference, which rustc does not yet perform for closures.

#[test]
fn no_derive_owner_type() {
    struct NoDerive(i32);
    fn get_field(o: &NoDerive) -> &i32 {
        &o.0
    }
    let bowl = BowlRef::new(Box::new(NoDerive(22)), get_field);
    assert_eq!(*bowl.get(), &22);
}

// --- public_cell ----------------------------------------------------------------
// NOT MIGRATABLE: self_cell! requires an explicit `pub` annotation on the generated
// struct. oyakodon's structs follow ordinary Rust visibility; no special test needed.

// --- custom_drop ----------------------------------------------------------------

#[test]
fn custom_drop() {
    struct Ref<'a, T>(&'a T);
    impl<'a, T> Drop for Ref<'a, T> {
        fn drop(&mut self) {
            let _ = self.0;
        }
    }
    fn make_ref(n: &i32) -> Ref<'_, i32> {
        Ref(n)
    }
    let bowl = BowlRef::new(Box::new(42i32), make_ref);
    drop(bowl);
}

// --- drop_order -----------------------------------------------------------------
// The derived view must be dropped before the base.
// Rust drops struct fields in declaration order;
// BowlRef declares `derived` before `base` for exactly this reason.

#[test]
fn drop_order() {
    #[derive(Debug, PartialEq)]
    enum Dropped {
        Owner,
        Dependent,
    }

    struct Owner(Rc<RefCell<Vec<Dropped>>>);
    struct Dep<'a>(&'a Owner, Rc<RefCell<Vec<Dropped>>>);

    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.borrow_mut().push(Dropped::Owner);
        }
    }
    impl Drop for Dep<'_> {
        fn drop(&mut self) {
            self.1.borrow_mut().push(Dropped::Dependent);
        }
    }

    struct MakeDep(Rc<RefCell<Vec<Dropped>>>);
    impl<'a> View<&'a Owner> for MakeDep {
        type Output = Dep<'a>;
    }
    impl<'a> Derive<&'a Owner> for MakeDep {
        fn call(self, o: &'a Owner) -> Dep<'a> {
            Dep(o, self.0)
        }
    }

    let drops: Rc<RefCell<Vec<Dropped>>> = Default::default();
    let bowl = BowlRef::new(Box::new(Owner(drops.clone())), MakeDep(drops.clone()));
    drop(bowl);
    assert_eq!(&drops.borrow()[..], &[Dropped::Dependent, Dropped::Owner]);
}

// --- into_owner_drop_dependent_without_panic ------------------------------------
// into_inner() drops derived first, then returns base.

#[test]
fn into_owner_drop_dependent_without_panic() {
    type O = Cell<Option<Box<u8>>>;

    struct D<'a>(&'a O);
    impl Drop for D<'_> {
        fn drop(&mut self) {
            self.0.take();
        }
    }

    struct MakeD;
    impl<'a> View<&'a O> for MakeD {
        type Output = D<'a>;
    }
    impl<'a> Derive<&'a O> for MakeD {
        fn call(self, c: &'a O) -> D<'a> {
            D(c)
        }
    }

    let bowl = BowlRef::new(Rc::new(Cell::new(Some(Box::new(42u8)))), MakeD);
    let base = bowl.into_inner(); // drops D first (takes from Cell), then returns Rc
    let cell = Rc::try_unwrap(base).ok().expect("Rc has multiple owners");
    assert!(cell.into_inner().is_none());
}

// --- into_owner_drop_dependent_with_panic ---------------------------------------

#[test]
#[should_panic]
fn into_owner_drop_dependent_with_panic() {
    type O = Cell<Option<Box<u8>>>;

    struct D<'a>(&'a O);
    impl Drop for D<'_> {
        fn drop(&mut self) {
            self.0.take();
            panic!("dependent drop panic");
        }
    }

    struct MakeD;
    impl<'a> View<&'a O> for MakeD {
        type Output = D<'a>;
    }
    impl<'a> Derive<&'a O> for MakeD {
        fn call(self, c: &'a O) -> D<'a> {
            D(c)
        }
    }

    let bowl = BowlRef::new(Rc::new(Cell::new(Some(Box::new(42u8)))), MakeD);
    bowl.into_inner();
}

// --- drop_panic_owner -----------------------------------------------------------

#[test]
fn drop_panic_owner() {
    struct PanickingOwner(String);
    impl Drop for PanickingOwner {
        fn drop(&mut self) {
            panic!("owner drop");
        }
    }
    fn get_str(o: &PanickingOwner) -> &str {
        o.0.as_str()
    }
    let bowl = BowlRef::new(Box::new(PanickingOwner("hello".into())), get_str);
    assert!(catch_unwind(AssertUnwindSafe(|| drop(bowl))).is_err());
}

// --- drop_panic_dependent -------------------------------------------------------

#[test]
fn drop_panic_dependent() {
    struct PanickingDep<'a>(&'a String);
    impl Drop for PanickingDep<'_> {
        fn drop(&mut self) {
            panic!("dependent drop");
        }
    }
    fn make_dep(s: &String) -> PanickingDep<'_> {
        PanickingDep(s)
    }
    let bowl = BowlRef::new(Box::new(String::from("hello")), make_dep);
    assert!(catch_unwind(AssertUnwindSafe(|| drop(bowl))).is_err());
}

// --- dependent_mutate -----------------------------------------------------------
// self_cell uses with_dependent_mut(|_, dep| ...); oyakodon uses BowlMut::get_mut().
// NOTE: dependent_replace (simultaneous &owner + &mut dep in with_dependent_mut)
// has no equivalent in oyakodon; see "not migratable" below.

fn get_slice(v: &mut Vec<u8>) -> &mut [u8] {
    v.as_mut_slice()
}

#[test]
fn dependent_mutate() {
    let mut bowl = BowlMut::new(Box::new(vec![1u8, 2, 3]), get_slice);
    assert_eq!(*bowl.get(), [1, 2, 3]);
    bowl.get_mut()[0] = 99;
    assert_eq!(*bowl.get(), [99, 2, 3]);
}

// --- dependent_replace ----------------------------------------------------------
// NOT MIGRATABLE: self_cell::with_dependent_mut(|owner, dep| { *dep = f(owner); })
// gives simultaneous &owner and &mut dep access.
// oyakodon exposes only &mut Output via get_mut(); the base is inaccessible.

// --- try_new_or_recover ---------------------------------------------------------
// NOT MIGRATABLE: same root limitation as failable_constructor_*.

// --- into_owner -----------------------------------------------------------------

#[test]
fn into_owner() {
    let expected = Rc::new(String::from("Endless joy for you never 2"));
    let bowl =
        BowlRef::<Rc<String>, fn(&String) -> &str>::new(Rc::clone(&expected), String::as_str);
    assert_eq!(*bowl.get(), expected.as_str());

    let recovered: Rc<String> = bowl.into_inner();
    assert_eq!(recovered, expected);
    assert_eq!(Rc::strong_count(&expected), 2);
}

// --- zero_size_cell -------------------------------------------------------------
// NOT MIGRATABLE: self_cell panics on ZST owners (implementation artifact of its
// pointer-based storage). oyakodon accepts ZST base types without restriction.

// --- nested_cells ---------------------------------------------------------------
// The child cell owns &'a String (a reference into the parent's base),
// so its lifetime is tied to the parent's.

#[test]
fn nested_cells() {
    struct MakeChild;
    impl<'a> View<&'a String> for MakeChild {
        type Output = BowlRef<'a, &'a String, fn(&String) -> &str>;
    }
    impl<'a> Derive<&'a String> for MakeChild {
        fn call(self, s: &'a String) -> Self::Output {
            BowlRef::new(s, String::as_str)
        }
    }

    let parent_str = String::from("some string it is");
    let parent = BowlRef::<Box<String>, MakeChild>::new(Box::new(parent_str.clone()), MakeChild);

    let child = parent.get();
    assert_eq!(*child.get(), parent_str.as_str());
}

// --- panic_in_from_owner --------------------------------------------------------
// A panic inside the derive function must not leak the base allocation.
// (Verified by running under Miri.)

#[test]
fn panic_in_from_owner() {
    let result = catch_unwind(|| {
        BowlRef::new(Box::new(String::from("hello")), |_: &String| -> &str {
            panic!()
        })
    });
    assert!(result.is_err());
}

// --- result_name_hygiene --------------------------------------------------------
// NOT MIGRATABLE: tests self_cell! macro hygiene (local `Result` type alias shadowing).
// oyakodon is a generic library; macro hygiene does not apply.

// --- debug_impl -----------------------------------------------------------------
// NOT MIGRATABLE: BowlRef/BowlMut do not implement Debug.

// --- lazy_ast -------------------------------------------------------------------
// Uses std::cell::OnceCell instead of once_cell crate.
// The original test lazily initialises the OnceCell by calling
// dep.get_or_init(|| owner.into()) inside with_dependent(),
// which requires simultaneous access to both owner and dependent —
// a pattern with no equivalent in oyakodon.
// Our version initialises eagerly in the derive function instead.

#[test]
fn lazy_ast() {
    use std::cell::OnceCell;

    struct LazyAst<'a>(OnceCell<&'a str>);

    struct MakeLazy;
    impl<'a> View<&'a String> for MakeLazy {
        type Output = LazyAst<'a>;
    }
    impl<'a> Derive<&'a String> for MakeLazy {
        fn call(self, s: &'a String) -> LazyAst<'a> {
            let cell = OnceCell::new();
            cell.set(s.as_str()).unwrap();
            LazyAst(cell)
        }
    }

    let bowl = BowlRef::<Box<String>, MakeLazy>::new(Box::new(String::from("hello")), MakeLazy);
    assert_eq!(*bowl.get().0.get().unwrap(), "hello");
}

// --- cell_mem_size --------------------------------------------------------------
// NOT MIGRATABLE: self_cell stores owner + dependent behind a single heap pointer,
// so size_of::<Cell>() == size_of::<*const u8>().
// oyakodon stores two MaybeDangling fields inline; size scales with T and Output.

// --- mut_borrow_* ---------------------------------------------------------------
// NOT MIGRATABLE: self_cell ships MutBorrow<T>, a RefCell-like wrapper that allows
// &mut access to the owner. oyakodon has no equivalent.
// Affected tests: mut_borrow_new_borrow, mut_borrow_mutate, mut_borrow_double_borrow,
//                 mut_borrow_new, mut_borrow_try_new, mut_borrow_try_new_or_recover,
//                 mut_borrow_new_borrow_mut.

// ================================================================================
// self_cell/tests-extra/src/lib.rs  →  tests/self_cell/tests_extra.rs
// self_cell/tests-extra/rust_1_85_or_newer/  →  tests/self_cell/rust_1_85_or_newer.rs
// self_cell/tests-extra/no_std_lib/  →  tests/self_cell/no_std_lib.rs
// self_cell/tests-extra/invalid/  →  documented in tests/self_cell/tests_extra.rs
// ================================================================================
