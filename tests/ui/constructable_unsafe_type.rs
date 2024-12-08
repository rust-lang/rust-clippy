#![warn(clippy::constructable_unsafe_type)]

struct PrivateUnsafeToken;
pub struct GoodUnsafeToken {
    _private: (),
}

pub struct DangerousUnsafeToken1;
//~^ error: `Unsafe` type is publically constructable
pub struct DangerousUnsafeToken2();
//~^ error: `Unsafe` type is publically constructable
pub struct DangerousUnsafeToken3 {}
//~^ error: `Unsafe` type is publically constructable
pub struct DangerousUnsafeToken4 {
    //~^ error: `Unsafe` type is publically constructable
    pub public: (),
}

fn main() {}
