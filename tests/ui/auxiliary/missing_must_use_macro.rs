#[macro_export]
macro_rules! define_types {
    () => {
        pub struct ExternalStruct(u8);
        pub enum ExternalEnum {
            First,
            Second,
        }
        pub union ExternalUnion {
            value: u8,
        }
    };
}
