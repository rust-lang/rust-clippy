#![warn(clippy::no_return_conversion)]

fn main() {}

struct Converter;

impl Converter {
    fn as_type(&self) {}
    //~^ no_return_conversion

    fn to_type(&self) {}
    //~^ no_return_conversion

    fn into_type(self) {}
    //~^ no_return_conversion
}
