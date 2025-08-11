//@no-rustfix
#![expect(unused)]
#![warn(clippy::unnecessary_operation)]

// don't lint if any of the fields has an ambiguous type when used by themselves
fn issue15381() {
    struct DescriptorSet {
        slots: Vec<u32>,
    }

    // the repro
    DescriptorSet { slots: Vec::new() };
    //~^ unnecessary_operation

    // other cases
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
}
