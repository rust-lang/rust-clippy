//@compile-flags: --test

#![allow(clippy::unnecessary_literal_unwrap)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

#[test]
fn test() {
    Some(3).unwrap();
    Some(3).expect("");
}

mod issue16369 {
    #[cfg(all(test, variant = "variant"))]
    mod all {
        // should not lint modules configured as `test`
        fn test_fn() {
            let _a: u8 = Some(2).unwrap();
            let _a: u8 = Some(3).expect("");
        }
    }

    #[cfg(any(test, true))]
    mod any {
        // should lint modules that can can be compiled without the `test` attribute
        fn test_fn() {
            let _a: u8 = Some(2).unwrap(); //~ unwrap_used
            let _a: u8 = Some(3).expect(""); //~ expect_used
        }
    }

    #[cfg(any(test, all(test, true)))]
    mod complex {
        // should not lint in `test` modules even if the `cfg` is complex
        fn test_fn() {
            let _a: u8 = Some(2).unwrap();
            let _a: u8 = Some(3).expect("");
        }
    }
}
