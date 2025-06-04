#![warn(clippy::needless_path_new)]

use std::fs;
use std::path::Path;

fn takes_path(_: &Path) {}

fn takes_path_and_impl_path(_: &Path, _: impl AsRef<Path>) {}

fn takes_two_impl_paths_with_the_same_generic<P: AsRef<Path>>(_: P, _: P) {}

struct Foo;

impl Foo {
    fn takes_path(_: &Path) {}
    fn takes_self_and_path(&self, _: &Path) {}
    fn takes_path_and_impl_path(_: &Path, _: impl AsRef<Path>) {}
    fn takes_self_and_path_and_impl_path(&self, _: &Path, _: impl AsRef<Path>) {}
}

fn main() {
    let f = Foo;

    fs::write(Path::new("foo.txt"), "foo"); //~ needless_path_new

    fs::copy(
        Path::new("foo"), //~ needless_path_new
        Path::new("bar"), //~ needless_path_new
    );

    Foo::takes_path(Path::new("foo"));

    f.takes_self_and_path_and_impl_path(
        Path::new("foo"),
        Path::new("bar"), //~ needless_path_new
    );

    // we can and should change both at the same time
    takes_two_impl_paths_with_the_same_generic(
        Path::new("foo"), //~ needless_path_new
        Path::new("bar"), //~ needless_path_new
    );

    // no warning
    takes_path(Path::new("foo"));

    // the paramater that _could_ be passed directly, was
    // the parameter that could't, wasn't
    takes_path_and_impl_path(Path::new("foo"), "bar");

    // same but as a method
    Foo::takes_path_and_impl_path(Path::new("foo"), "bar");
    f.takes_self_and_path_and_impl_path(Path::new("foo"), "bar");

    fn foo() -> Option<&'static Path> {
        // Some(...) is `ExprKind::Call`, but we don't consider it
        Some(Path::new("foo.txt"))
    }
}
