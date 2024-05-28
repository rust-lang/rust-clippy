#![crate_type = "lib"]
#![warn(clippy::let_arr_const)]

// <https://github.com/rust-lang/rust/issues/73825>.
pub mod issue_rust_73825 {
    use std::mem::{ManuallyDrop, MaybeUninit};
    macro_rules! mac_gen_arr {
        () => {
            [0u32; 64]
        };
    }

    pub fn do_not() {
        let _repeat = *&[0; 64];
        let _arr = *&[0, 1, 2, 3, 4];

        let (x, y) = (1, 2);
        let _arr = [x, y];
        let _arr = [42u8; 16];
        let _arr = mac_gen_arr!();
        let _arr = gen_array();
        let _arr = gen_array_non_copy();
        let _arr = [gen_array_no_const(), gen_array_no_const()];
        {
            let mut arr = [0; 32];
            arr[1] = 42;
        }
        {
            let a = String::from("a");
            let b = String::from("a");
            let _arr = [a, b];
        }
        {
            let mut arr = [const { String::new() }; 32];
            arr[1] = "a".to_owned();
        }
        let _repeat: [i32; 64] = [0; 64];
        let _repeat = [42u8; 17];

        type T = MaybeUninit<u64>;
        let _array = [T::new(42), T::new(3), T::new(42), T::new(42)];

        type U = ManuallyDrop<u64>;
        let _array = [U::new(42), U::new(3), U::new(42), U::new(42)];
    }

    fn gen_array_no_const() -> [u32; 42] {
        unimplemented!()
    }

    const fn gen_array() -> [u32; 42] {
        unimplemented!()
    }

    const fn gen_array_non_copy() -> [String; 42] {
        unimplemented!()
    }

    pub fn do_it() {
        // Copy type
        let _arr = [0, 1, 3, 5, 7, 8];
        let _arr = [gen_array(), gen_array()];
        // Non Copy type
        {
            let _repeat_const = [const { String::new() }; 32];
            const EMPTY: String = String::new();
            let _repeat = [EMPTY; 32];
        }
    }
}
