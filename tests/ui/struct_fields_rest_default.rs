//@aux-build:proc_macros.rs
#![warn(clippy::struct_fields_rest_default)]
extern crate proc_macros;

#[derive(Default)]
struct DeriveCase {
    a: i32,
    b: i32,
    c: i32,
}

impl DeriveCase {
    fn get_derive_case() -> Self {
        DeriveCase { a: 0, b: 0, c: 0 }
    }
}

struct ManuallyCase {
    a: i32,
    b: i32,
    c: i32,
}

impl Default for ManuallyCase {
    fn default() -> Self {
        ManuallyCase { a: 10, b: 20, c: 30 }
    }
}

fn main() {
    #[rustfmt::skip]
    let _ = DeriveCase {
        a: 10,
        ..Default::default()
        //~^ struct_fields_rest_default
    };

    #[rustfmt::skip]
    let _ = DeriveCase {
        a: 10,
        ..DeriveCase::default()
        //~^ struct_fields_rest_default
    };

    // should not lint
    let _ = DeriveCase {
        a: 10,
        ..DeriveCase::get_derive_case()
    };

    // ------- ManuallyCase -------
    // should not lint manually default
    let _ = ManuallyCase {
        a: 10,
        ..Default::default()
    };

    // should not lint manually default
    let _ = ManuallyCase {
        a: 10,
        ..ManuallyCase::default()
    };
    // ----------------------------

    // should not lint in external macro
    proc_macros::external! {
        #[derive(Default)]
        struct ExternalDeriveCase {
            a: i32,
            b: i32,
        }

        let _ = ExternalDeriveCase {
            a: 10,
            ..Default::default()
        };

        struct ExternalManuallyCase {
            a: i32,
            b: i32,
        }

        impl Default for ExternalManuallyCase {
            fn default() -> Self {
                ExternalManuallyCase { a: 10, b: 20 }
            }
        }

        let _ = ExternalManuallyCase {
            a: 10,
            ..Default::default()
        };
    }

    // should not lint
    let _ = DeriveCase {
        a: Default::default(),
        b: Default::default(),
        c: Default::default(),
    };
}
