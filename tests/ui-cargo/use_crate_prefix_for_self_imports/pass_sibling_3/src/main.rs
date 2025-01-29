mod foo;
pub use foo::{Bar, Foo};

fn main() {
    let _foo = Foo;
    let _bar = Bar;
}
