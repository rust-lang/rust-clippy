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

    #[expect(clippy::unused_unit)]
    fn as_explicit_unit(&self) -> () {}
    //~^ missing_return_type

    fn as_dont_fire(&self) -> Self {
        Self
    }
}
