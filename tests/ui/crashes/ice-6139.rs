//@ check-pass

trait T<'a> {}

fn bar(_: Vec<Box<dyn T<'_>>>) {}

fn main() {
    bar(vec![]);
}
