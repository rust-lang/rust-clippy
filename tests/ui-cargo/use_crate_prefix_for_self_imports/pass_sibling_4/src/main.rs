pub use foo::{Bar, Foo};
mod foo;

fn main() {
    let _foo = Foo;
    let _bar = Bar;
}
