//@no-rustfix
#![expect(unused)]
#![warn(clippy::unnecessary_operation)]

// don't lint if any of the fields has an ambiguous type when used by themselves
fn issue15381() {
    struct DescriptorSet {
        slots: Vec<u32>,
    }

    DescriptorSet { slots: Vec::new() };
    //~^ unnecessary_operation
}
