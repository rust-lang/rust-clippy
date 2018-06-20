#![allow(dead_code)]
#![warn(enum_sex)]

// Separate modules so we can test the same enum name multiple times
mod bad_cases {

    // Typical cases to catch
    enum Sex {
        Female,
        Male,
    }

    enum Gender {
        M,
        F,
    }

}

mod good_cases {

    // Not caught, because 3 variants
    enum Sex {
        A,
        B,
        C,
    }

    // Not caught, because only one variant
    enum Gender {
        No
    }

    // Not caught, because different name
    enum Connector {
        Male,
        Female,
    }

}

fn main() {
}
