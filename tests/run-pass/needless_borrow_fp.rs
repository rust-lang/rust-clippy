#[deny(clippy::all)]
#[allow(clippy::missing_copy_implementations)]
#[derive(Debug)]
pub enum Error {
    Type(&'static str),
}

fn main() {}
