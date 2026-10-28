#![warn(clippy::missing_return_type)]

fn main() {}

struct Converter;

impl Converter {
    fn as_type(&self) {}
    //~^ missing_return_type

    fn to_type(&self) {}
    //~^ missing_return_type

    fn into_type(self) {}
    //~^ missing_return_type
}
