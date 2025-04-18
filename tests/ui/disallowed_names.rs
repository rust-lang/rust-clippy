#![allow(
    dead_code,
    clippy::needless_if,
    clippy::similar_names,
    clippy::single_match,
    clippy::toplevel_ref_arg,
    unused_mut,
    unused_variables
)]
#![warn(clippy::disallowed_names)]

fn test(foo: ()) {}
//~^ disallowed_names

fn main() {
    let foo = 42;
    //~^ disallowed_names

    let baz = 42;
    //~^ disallowed_names

    let quux = 42;
    //~^ disallowed_names

    // Unlike these others, `bar` is actually considered an acceptable name.
    // Among many other legitimate uses, bar commonly refers to a period of time in music.
    // See https://github.com/rust-lang/rust-clippy/issues/5225.
    let bar = 42;

    let food = 42;
    let foodstuffs = 42;
    let bazaar = 42;

    match (42, Some(1337), Some(0)) {
        (foo, Some(baz), quux @ Some(_)) => (),
        //~^ disallowed_names
        //~| disallowed_names
        //~| disallowed_names
        _ => (),
    }
}

fn issue_1647(mut foo: u8) {
    //~^ disallowed_names

    let mut baz = 0;
    //~^ disallowed_names

    if let Some(mut quux) = Some(42) {}
    //~^ disallowed_names
}

fn issue_1647_ref() {
    let ref baz = 0;
    //~^ disallowed_names

    if let Some(ref quux) = Some(42) {}
    //~^ disallowed_names
}

fn issue_1647_ref_mut() {
    let ref mut baz = 0;
    //~^ disallowed_names

    if let Some(ref mut quux) = Some(42) {}
    //~^ disallowed_names
}

#[cfg(test)]
mod tests {
    fn issue_7305() {
        // `disallowed_names` lint should not be triggered inside of the test code.
        let foo = 0;

        // Check that even in nested functions warning is still not triggered.
        fn nested() {
            let foo = 0;
        }
    }
}

#[test]
fn test_with_disallowed_name() {
    let foo = 0;
}

mod functions_test {
    fn foo() {}
    //~^ disallowed_names

    pub fn quux(_some_meaningful_arg: i32) {}
    //~^ disallowed_names

    pub async fn baz(_more_meaningful_arg: bool) {}
    //~^ disallowed_names

    fn do_not_lint_foo() {}

    struct SomeMeaningfulStruct {}
    impl SomeMeaningfulStruct {
        fn foo(&self) {}
        //~^ disallowed_names

        const fn baz(&self) {}
        //~^ disallowed_names
    }
}
