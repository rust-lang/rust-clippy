#![warn(clippy::empty_enum_variants_with_brackets)]

enum TestEnum {
    // These two enums should not trigger the lint.
    NonEmptyBrace { x: i32, y: i32 },
    NonEmptyParentheses(i32, i32),

    // Enum variant with braces and no data
    EmptyBrace {}, //~ ERROR: enum variant with brackets has empty body

    // Enum variant with parentheses and no data
    EmptyParentheses(), //~ ERROR: enum variant with brackets has empty body
}

enum TestEnumWithFeature {
    // These two enums should not trigger the lint.
    NonEmptyBrace {
        #[cfg(feature = "thisisneverenabled")]
        x: i32,
    },
    NonEmptyParentheses(#[cfg(feature = "thisisneverenabled")] i32),
}

fn main() {}
