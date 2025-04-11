//@no-rustfix: name conflicts

//@revisions: default outside_cfg_test
//@[default] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/redundant_test_prefix/default
//@[outside_cfg_test] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/redundant_test_prefix/outside_cfg_test
//@compile-flags: --test
#![allow(dead_code)]
#![warn(clippy::redundant_test_prefix)]

fn main() {}

// Has no `#[cfg(test)]` annotation.
mod tests_no_annotations {
    use super::*;

    #[test]
    fn test_has_annotation() {
        //~[outside_cfg_test]^ redundant_test_prefix
    }

    fn no_annotation() {}
}

#[test]
fn test_main_module_has_annotation() {
    //~[outside_cfg_test]^ redundant_test_prefix
}

fn test_main_module_no_annotation() {}

fn foo() {}

#[cfg(test)]
#[test]
fn test_foo() {
    //~^ redundant_test_prefix

    todo!()
    // Has prefix, has `#[test]` attribute, within a `#[cfg(test)]`.
    // Collision with existing function, so suggestion is to rename the function.
}

fn bar() {}

#[test]
fn test_bar() {
    //~[outside_cfg_test]^ redundant_test_prefix

    todo!()
    // Has prefix, has `#[test]` attribute.
    // NOT within a `#[cfg(test)]`, but the lint is enabled for integration tests.
    // Collision with existing function, so suffix is added.
}

mod m1 {
    pub fn baz() {}
}

#[cfg(test)]
#[test]
fn test_baz() {
    //~^ redundant_test_prefix

    use m1::baz;

    baz();
    // Has prefix, has `#[test]` attribute, within a `#[cfg(test)]`.
    // No collision, but has a function call that will result in recursion.
}

#[test]
fn test_1() {
    //~[outside_cfg_test]^ redundant_test_prefix

    todo!()
    // `1` is invalid function name, so suggestion to rename is emitted
}

#[test]
fn test_const() {
    //~[outside_cfg_test]^ redundant_test_prefix

    todo!()
    // `const` is reserved keyword, so suggestion to rename is emitted
}

#[test]
fn test_async() {
    //~[outside_cfg_test]^ redundant_test_prefix

    todo!()
    // `async` is reserved keyword, so suggestion to rename is emitted
}

#[test]
fn test_yield() {
    //~[outside_cfg_test]^ redundant_test_prefix

    todo!()
    // `yield` is reserved keyword for future use, so suggestion to rename is emitted
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_1() {
        //~^ redundant_test_prefix

        todo!()
        // `1` is invalid function name, so suggestion to rename is emitted
    }

    #[test]
    fn test_const() {
        //~^ redundant_test_prefix

        todo!()
        // `const` is reserved keyword, so suggestion to rename is emitted
    }

    #[test]
    fn test_async() {
        //~^ redundant_test_prefix

        todo!()
        // `async` is reserved keyword, so suggestion to rename is emitted
    }

    #[test]
    fn test_yield() {
        //~^ redundant_test_prefix

        todo!()
        // `yield` is reserved keyword for future use, so suggestion to rename is emitted
    }
}
