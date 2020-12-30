

#[non_exhaustive]
pub enum AnotherCrateEnum {
    AFirst,
    ASecond,
    AThird,
}

#[derive(Default)]
#[non_exhaustive]
pub struct AnotherCrateStruct {
    pub a1: i32,
    pub b1: i32,
    pub c1: i32,
}

#[derive(Default)]
#[non_exhaustive]
pub struct TPrivateField(pub i32, pub i32, i32);
