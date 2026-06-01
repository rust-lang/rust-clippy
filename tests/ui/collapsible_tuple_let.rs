#![warn(clippy::collapsible_tuple_let)]
#![allow(unused)]

fn side_effect() -> i32 {
    42
}

fn expensive() -> String {
    String::from("hello")
}

fn other() -> i32 {
    1
}

// ----- Should lint -----

fn lint_all_block_locals() {
    // Both elements are paths to block-locals in declaration order
    let (a, b) = {
        let x = side_effect();
        let y = other();
        (x, y)
    };
    //~^^^^^ collapsible_tuple_let
}

fn lint_one_block_local_one_inline() {
    // Block-local first, then inline expression
    let (a, b) = {
        let x = side_effect();
        (x, other())
    };
    //~^^^^ collapsible_tuple_let
}

fn lint_only_inline() {
    // No block-locals at all — block is still unnecessary
    let (a, b) = { (side_effect(), other()) };
    //~^ collapsible_tuple_let
}

fn lint_three_elements() {
    let (a, b, c) = {
        let x = 1i32;
        let y = 2i32;
        (x, y, 3i32)
    };
    //~^^^^^ collapsible_tuple_let
}

fn lint_wildcard() {
    // Wildcard outer binding
    let (_, b) = {
        let x = side_effect();
        (x, other())
    };
    //~^^^^ collapsible_tuple_let
}

// ----- Should NOT lint -----

fn no_lint_type_annotation() {
    // Outer let has a type annotation
    let (a, b): (i32, i32) = {
        let x = 1;
        (x, 2)
    };
}

fn no_lint_block_local_after_inline() {
    // Inline expression comes before a block-local: would reorder evaluation
    let (a, b) = {
        let x = side_effect();
        let y = other();
        (y, x) // reversed — out of declaration order
    };
}

fn no_lint_block_local_used_twice() {
    // Same block-local referenced twice in tuple
    let (a, b) = {
        let x = side_effect();
        (x, x)
    };
}

fn no_lint_block_local_used_in_other_stmt() {
    // Block-local `x` is used in the init of `y`, not just the trailing tuple
    let (a, b) = {
        let x = side_effect();
        let y = x + 1; // x used here
        (x, y)
    };
}

fn no_lint_inline_before_block_local() {
    // Inline expression comes before a block-local — reordering would change eval order
    let (a, b) = {
        let x = side_effect();
        (other(), x) // inline before block-local
    };
}

fn no_lint_non_simple_outer_pat() {
    // Outer tuple contains a struct/enum sub-pattern (refutable if used as a let)
    let x: Option<i32> = Some(1);
    // This already doesn't match PatKind::Binding with no sub-pattern, so we skip
}

fn no_lint_let_else_in_block() {
    // Block contains a `let-else` (not a simple `let x = init;`)
    let (a, b): (i32, i32) = {
        let x: i32 = 1;
        (x, 2)
    };
}

fn no_lint_block_has_expr_stmt() {
    // Block contains an expression statement (not just let stmts)
    let (a, b): (i32, i32) = {
        let _ = side_effect(); // expression statement (StmtKind::Semi)
        let x = other();
        (x, 2)
    };
}

fn no_lint_inline_references_block_local() {
    // Inline expression references a block-local — can't eliminate the local
    let (a, b) = {
        let x = side_effect();
        let y = other();
        (x, y + x) // y+x references x which is a block-local
    };
}

fn main() {}
