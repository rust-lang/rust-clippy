#![allow(unused)]
#![warn(clippy::fn_param_ref_cloned)]
#![feature(stmt_expr_attributes)]

// Impl methods
#[derive(Clone, Default)]
pub struct IsClone;

#[derive(Default)]
pub struct IsNotClone;

#[derive(Default)]
pub struct PartialNotClone {
    clone_field: IsClone,
    not_clone_field: IsNotClone,
}

#[derive(Default)]
pub struct PartialClone {
    clone_field: IsClone,
    not_clone_field: IsNotClone,
}

impl IsNotClone {
    fn clone(&self) {}
}

// We know about this edgecase situation. I am not entirely sure how to filter this case
fn set_cell(cell: &std::cell::Cell<u32>) {
    cell.set(5);
    let a = cell.clone();
    //~^ fn_param_ref_cloned
}

fn create_cell() {
    let a = std::cell::Cell::new(0);
    set_cell(&a);
    println!("{}", a.get());
}

impl IsClone {
    fn this_is_not_clone(&self) {}

    pub fn no_ref(&self) {
        let is_clone = IsClone;
        let cloned_no_ref = is_clone.clone();
    }

    pub fn cloning_ref(&self, is_clone: &IsClone) {
        let cloned_ref_param = is_clone.clone();
        //~^ fn_param_ref_cloned
    }

    pub fn using_ref(&self, is_clone: &IsClone) {
        let x = "";
        let b = is_clone;
    }
}

pub fn test_gen_function<T: Clone>(is_clone_ref: &T, is_clone_owned: T) {
    // lint when we clone a param that is a reference
    is_clone_ref.clone();
    //~^ fn_param_ref_cloned

    // don't lint when we call clone on an owned parameter
    is_clone_owned.clone();
}

pub fn basic_clone_function(is_not_clone_ref: &IsNotClone, is_clone_ref: &IsClone, is_clone_owned: IsClone) {
    // lint when we clone a param that is a reference
    is_clone_ref.clone();
    //~^ fn_param_ref_cloned

    // don't lint when we call a different method
    is_clone_ref.this_is_not_clone();

    // don't lint when we call clone on an owned parameter
    is_clone_owned.clone();

    // don't lint when we call clone on a type that doesn't implement the trait Clone
    is_not_clone_ref.clone();

    // don't lint when we clone a local variable of IsClone type
    let local_is_clone = IsClone;
    local_is_clone.clone();

    // or any other method of that local variable
    local_is_clone.this_is_not_clone();

    // lint when we clone on a re-binding of a cloneable reference
    let rebound = is_clone_ref;
    rebound.clone();
    //~^ fn_param_ref_cloned
}

fn partial_clone(
    partial_not_clone_owned: PartialNotClone,
    partial_not_clone_ref: &PartialNotClone,
    partial_clone_owned: PartialClone,
    partial_clone_ref: &PartialClone,
) {
    // Move owned values
    let moved = partial_not_clone_owned.clone_field;
    let moved = partial_not_clone_owned.not_clone_field;

    let moved = partial_clone_owned.clone_field;
    let moved = partial_clone_owned.not_clone_field;

    // Clone only struct fields on non-clone struct
    let cloned = partial_not_clone_ref.clone_field.clone();

    // Clone only struct fields on clone struct
    let cloned = partial_clone_ref.clone_field.clone();
}

fn dont_check_if_stmts(clone_ref: &IsClone, if_arg: usize) {
    // #[clippy::dump]
    if if_arg == 0usize {
        let should_allow_clone = clone_ref.clone();
    }

    let should_not_allow_clone = clone_ref.clone();
    //~^ fn_param_ref_cloned
}

fn main() {
    let a = IsClone;
    let b = IsClone;
    let c = IsNotClone;
    basic_clone_function(&c, &a, b);

    let d = PartialNotClone::default();
    let e = PartialClone::default();
    let f = PartialNotClone::default();
    let g = PartialClone::default();
    partial_clone(d, &f, e, &g);

    let h = std::cell::Cell::new(0);
    set_cell(&h);
    create_cell();
    dont_check_if_stmts(&a, 0usize);
}
