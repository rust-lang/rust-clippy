#![warn(clippy::vec_from_literal_array)]

fn get_fixed_number() -> u32 {
    5
}

fn double_number(n: u32) -> u32 {
    n * 2
}

fn print_vec(v: Vec<u32>) {
    println!("{v:?}");
}

macro_rules! get_macro_number {
    () => {
        14
    };
}

macro_rules! double_macro_number {
    ($n:literal) => {
        $n * 2
    };
}

mod with_extension_trait {
    pub trait CustomVec {
        fn to_vec(&self) -> Vec<u32> {
            vec![1, 2, 3]
        }
    }

    impl<T, const N: usize> CustomVec for [T; N] {}
}

macro_rules! get_array {
    () => {
        [1, 2, 3]
    };
}

struct HasToVec;

impl HasToVec {
    fn to_vec(&self) -> Vec<u32> {
        vec![1, 2, 3]
    }
}

fn main() {
    let _v1: Vec<_> = [1, 2, 3].to_vec();
    //~^ vec_from_literal_array
    let _v2: Vec<_> = [4, get_fixed_number(), 6].to_vec();
    //~^ vec_from_literal_array
    let _v3: Vec<_> = [7, double_number(8), 9].to_vec();
    //~^ vec_from_literal_array
    print_vec(
        [10, 11, 12].to_vec(),
        //~^ vec_from_literal_array
    );
    let _v4: Vec<_> = [13, get_macro_number!(), 15].to_vec();
    //~^ vec_from_literal_array
    let _v5: Vec<_> = [16, double_macro_number!(17), 18].to_vec();
    //~^ vec_from_literal_array
    // Should not trigger the lint - not the built-in to_vec() method
    {
        use with_extension_trait::CustomVec;
        let _v6: Vec<u32> = ["a", "b", "c"].to_vec();
    }
    // Should not trigger the lint - array comes from a macro
    let _v7: Vec<u32> = get_array!().to_vec();
    // Should not trigger the lint - not called on an array literal
    let _v8: Vec<u32> = HasToVec.to_vec();
}
