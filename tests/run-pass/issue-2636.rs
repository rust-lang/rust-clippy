use std::hash::{Hash, Hasher};

fn main() {}

pub trait Fact: 'static + Eq + Hash { }

pub enum TestData<T: Fact> {
    F32(fn(&T) -> &f32, Vec<u16>),
    F64(fn(&T) -> &f64, Vec<u16>),
}

impl<T: Fact> TestData<T> {
    fn hash_self() {}
}

macro_rules! test_hash {
    ($($t:ident => $ord:expr),+ ) => {
        impl<T:Fact>Hash for TestData<T> {
            fn hash < H: Hasher > ( & self, _state: & mut H) {
                use self::TestData::*;
                    match self {
                    $ ( & $ t(_accessor, ref _test) => Self::hash_self(),
                    )*
                }
            }
        }
    };
}


test_hash!(F32 => 10, F64 => 11);
