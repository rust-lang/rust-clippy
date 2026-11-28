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

    fn to_dont_fire(&self) -> Self {
        Self
    }

    fn into_dont_fire(self) -> Self {
        Self
    }
}

trait ConverterTrait: Sized {
    fn as_trait(&self) {}
    //~^ missing_return_type

    fn to_trait(&self) {}
    //~^ missing_return_type

    fn into_trait(self) {}
    //~^ missing_return_type
}

struct ConverterImpl;

impl ConverterTrait for ConverterImpl {
    fn as_trait(&self) {}

    fn to_trait(&self) {}

    fn into_trait(self) {}
}
