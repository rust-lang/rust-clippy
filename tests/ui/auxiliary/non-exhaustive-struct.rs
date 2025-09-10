#[non_exhaustive]
#[derive(Default)]
pub struct NonExhaustiveStruct {
    pub field1: i32,
    pub field2: i32,
    _private: i32,
}
