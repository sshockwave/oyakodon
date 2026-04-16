// Tests migrated from self_cell/tests-extra/src/lib.rs.
// Kept in the same order as the original so missing entries are easy to spot.

use std::sync::Arc;

use oyakodon::BowlRef;

// --- not_send -------------------------------------------------------------------
// NOT MIGRATABLE: uses the `impls!` macro to assert negative trait bounds at runtime.
// The positive direction (BowlRef: Sync when Output: Sync) is verified implicitly
// by share_across_threads below. The negative direction would need the `impls` crate.

// --- not_sync -------------------------------------------------------------------
// NOT MIGRATABLE: same as not_send.

// --- mut_borrow_traits ----------------------------------------------------------
// NOT MIGRATABLE: tests Send/Sync of MutBorrow; oyakodon has no MutBorrow.

// --- invalid_compile (trybuild) -------------------------------------------------
// The 10 compile-fail tests in self_cell/tests-extra/invalid/ are listed below
// in the same order as the directory.
// None are directly migratable; the reasons are noted per test.
//
// --- contravariant_owner --------------------------------------------------------
// NOT MIGRATABLE (needs trybuild): the test would require a named compile_fail
// test file and the trybuild crate; not yet ported.
//
// --- covariant_owner_non_covariant_dependent ------------------------------------
// NOT MIGRATABLE: tests self_cell's #[covariant] annotation.
// oyakodon has no such annotation; variance is expressed directly in the
// View impl's Output type.
//
// --- escape_dependent -----------------------------------------------------------
// NOT MIGRATABLE (no runtime test needed): get() ties the return lifetime to &self,
// so the derived reference cannot outlive the bowl. Statically rejected.
//
// --- leak_dependent -------------------------------------------------------------
// NOT MIGRATABLE (no runtime test needed): get() lifetime is tied to &self.
// Leaking the reference beyond self's lifetime is a compile error.
//
// --- leak_outside_ref -----------------------------------------------------------
// NOT MIGRATABLE (no runtime test needed): HRTB on from_derive
// (for<'b> Derive<&'b T::Target>) prevents the derive fn from capturing references
// with outside lifetimes. Statically rejected.
//
// --- reborrow_dependent_cyclic --------------------------------------------------
// NOT MIGRATABLE: the derive function receives &T::Target and cannot reference the
// BowlRef being constructed, making cyclic self-reference impossible to express.
//
// --- swap_cell_member -----------------------------------------------------------
// NOT MIGRATABLE: exploits self_cell's exposed `unsafe_self_cell` public field to
// swap internals between two cells. oyakodon exposes no such field.
//
// --- with_mut_stack_use ---------------------------------------------------------
// NOT MIGRATABLE (no runtime test needed): oyakodon's get_mut() lifetime is tied to
// &mut self, so assigning a stack reference to *dependent is a compile error.
//
// --- wrong_covariance -----------------------------------------------------------
// NOT MIGRATABLE: tests self_cell's #[covariant] annotation. See above.
//
// --- wrong_covariance_unsize_coercion -------------------------------------------
// NOT MIGRATABLE: tests self_cell's #[covariant] annotation. See above.

// --- share_across_threads -------------------------------------------------------
// BowlRef is Sync when Output: Sync (see unsafe impl in bowl_ref.rs).

#[test]
fn share_across_threads() {
    let bowl = BowlRef::<Arc<String>, fn(&String) -> &str>::new(
        Arc::new(String::from("hy hyperspeed")),
        String::as_str,
    );

    std::thread::scope(|scope| {
        scope.spawn(|| assert_eq!(*bowl.get(), "hy hyperspeed"));
        scope.spawn(|| assert_eq!(*bowl.get(), "hy hyperspeed"));
        assert_eq!(*bowl.get(), "hy hyperspeed");
    });
}
