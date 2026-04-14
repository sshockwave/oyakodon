// Tests migrated from self_cell/tests-extra/no_std_lib/src/lib.rs.
// Kept in the same order as the original so missing entries are easy to spot.

// --- self_cell_works_in_no_std_env ----------------------------------------------
// Partially migratable: the logic (constructing a cell with a static-slice owner
// and an array-of-slices dependent) is reproduced below using Box<StaticString>.
//
// The no_std environment test itself is NOT MIGRATABLE here:
// testing actual no_std behaviour requires a separate crate with #![no_std]
// (like self_cell/tests-extra/no_std_lib/), because an integration test binary
// always links the std crate.
//
// oyakodon has std/alloc/stable_deref features; no_std+alloc support is possible
// but not covered by this file.

use oyakodon::{BowlRef, Derive, View};

const SCRATCH_REGION: [u8; 4096] = [0u8; 4096];

struct StaticString {
    region: &'static [u8],
}

const MAX_NODES: usize = 8;

#[derive(Eq, PartialEq)]
struct Ast<'a>([Option<&'a [u8]>; MAX_NODES]);

struct MakeAst;
impl<'a> View<&'a StaticString> for MakeAst {
    type Output = Ast<'a>;
}
impl<'a> Derive<&'a StaticString> for MakeAst {
    fn call(self, code: &'a StaticString) -> Ast<'a> {
        let mut ast_nodes = [None; MAX_NODES];
        ast_nodes[0] = Some(&code.region[3..7]);
        ast_nodes[1] = Some(&code.region[10..12]);
        Ast(ast_nodes)
    }
}

#[test]
fn self_cell_works_in_no_std_env() {
    let pre_processed_code = StaticString {
        region: &SCRATCH_REGION[4000..4015],
    };

    let ast_cell =
        BowlRef::<Box<StaticString>, MakeAst>::new(Box::new(pre_processed_code), MakeAst);
    assert_eq!(ast_cell.get().0.iter().filter(|v| v.is_some()).count(), 2);
}
