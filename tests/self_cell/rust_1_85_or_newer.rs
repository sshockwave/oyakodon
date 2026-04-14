// Tests migrated from self_cell/tests-extra/rust_1_85_or_newer/src/lib.rs.
// Kept in the same order as the original so missing entries are easy to spot.

// --- async_self_cell ------------------------------------------------------------
// NOT MIGRATABLE: self_cell's #[async_builder] attribute generates an async
// constructor (SelfCell::new(owner, async |owner| ...).await).
// oyakodon's constructors are synchronous; there is no async equivalent.

// --- async_self_cell_try_new ----------------------------------------------------
// NOT MIGRATABLE: requires async + try_new; oyakodon has neither.

// --- async_self_cell_try_new_or_recover -----------------------------------------
// NOT MIGRATABLE: same as above.

// --- async_self_cell_recover ----------------------------------------------------
// NOT MIGRATABLE: same as above.

// --- async_self_cell_with_sleep -------------------------------------------------
// NOT MIGRATABLE: same as above.

// --- async_self_cell_with_mut_borrow --------------------------------------------
// NOT MIGRATABLE: requires async constructor and MutBorrow;
// oyakodon has neither.
