#![allow(clippy::needless_return, clippy::diverging_sub_expression)]
#![warn(clippy::default_mismatches_new)]

fn main() {}

//
// Nothing to change
//
struct ManualDefault(i32);
impl ManualDefault {
    fn new() -> Self {
        Self(42)
    }
}
impl Default for ManualDefault {
    fn default() -> Self {
        Self(42)
    }
}

#[derive(Default)]
struct CallToDefaultDefault(i32);
impl CallToDefaultDefault {
    fn new() -> Self {
        Default::default()
    }
}

#[derive(Default)]
struct CallToSelfDefault(i32);
impl CallToSelfDefault {
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
struct CallToTypeDefault(i32);
impl CallToTypeDefault {
    fn new() -> Self {
        CallToTypeDefault::default()
    }
}

#[derive(Default)]
struct CallToFullTypeDefault(i32);
impl CallToFullTypeDefault {
    fn new() -> Self {
        crate::CallToFullTypeDefault::default()
    }
}

#[derive(Default)]
struct ReturnCallToSelfDefault(i32);
impl ReturnCallToSelfDefault {
    fn new() -> Self {
        return Self::default();
    }
}

#[derive(Default)]
struct MakeResultSelf(i32);
impl MakeResultSelf {
    fn new() -> Result<Self, ()> {
        Ok(Self(10))
    }
}

#[derive(Default)]
struct WithParams(i32);
impl WithParams {
    fn new(val: i32) -> Self {
        Self(val)
    }
}

#[derive(Default)]
struct Async(i32);
impl Async {
    async fn new() -> Self {
        Self(42)
    }
}

#[derive(Default)]
struct DeriveDefault;
impl DeriveDefault {
    fn new() -> Self {
        // Adding ::default() would cause clippy::default_constructed_unit_structs
        Self
    }
}

#[derive(Default)]
struct DeriveTypeDefault;
impl DeriveTypeDefault {
    fn new() -> Self {
        // Adding ::default() would cause clippy::default_constructed_unit_structs
        return crate::DeriveTypeDefault;
    }
}

//
// Offer suggestions
//

#[derive(Default)]
struct DeriveIntDefault {
    value: i32,
}
impl DeriveIntDefault {
    fn new() -> Self {
        //~^ default_mismatches_new
        Self { value: 0 }
    }
}

#[derive(Default)]
struct DeriveTupleDefault(i32);
impl DeriveTupleDefault {
    fn new() -> Self {
        //~^ default_mismatches_new
        Self(0)
    }
}

#[derive(Default)]
struct NonZeroDeriveDefault(i32);
impl NonZeroDeriveDefault {
    fn new() -> Self {
        //~^ default_mismatches_new
        Self(42)
    }
}

#[derive(Default)]
struct ExtraBlockDefault(i32);
impl ExtraBlockDefault {
    fn new() -> Self {
        //~^ default_mismatches_new
        { Self::default() }
    }
}

#[derive(Default)]
struct ExtraBlockRetDefault(i32);
impl ExtraBlockRetDefault {
    fn new() -> Self {
        //~^ default_mismatches_new
        return {
            {
                {
                    return Self::default();
                }
            }
        };
    }
}

#[derive(Default)]
struct MultiStatements(i32);
impl MultiStatements {
    fn new() -> Self {
        //~^ default_mismatches_new
        println!("Hello, world!");
        let i = 42;
        Self(i)
    }
}

//
// TODO: Fix in the future
//
#[derive(Default)]
struct OptionGeneric<T>(Option<T>);
impl<T> OptionGeneric<T> {
    fn new() -> Self {
        OptionGeneric(None)
    }
}
