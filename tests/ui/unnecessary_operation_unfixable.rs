//@no-rustfix
#![expect(unused)]
#![warn(clippy::unnecessary_operation)]

// don't lint if any of the fields has an ambiguous type when used by themselves
fn issue15381_original() {
    struct DescriptorSet {
        slots: Vec<u32>,
    }

    // the repro
    DescriptorSet { slots: Vec::new() };
    //~^ unnecessary_operation
}

fn issue15381() {
    enum E {
        Foo { f: Vec<u32> },
        Bar(Vec<u32>),
    }
    E::Foo { f: Vec::new() };
    //~^ unnecessary_operation
    E::Bar(Vec::new());
    //~^ unnecessary_operation

    struct Tuple(Vec<u32>);
    Tuple(Vec::new());
    //~^ unnecessary_operation

    // the type of the second slice gets inferred based on it needing to be the same to that of the
    // first one, but that doesn't happen when they're outside the array
    [[1, 2, 3].as_slice(), [].as_slice()];
    //~^ unnecessary_operation
}
