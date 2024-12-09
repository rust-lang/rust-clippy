#![warn(clippy::unnecessary_literal_bound)]

struct Struct<'a> {
    not_literal: &'a str,
}

impl Struct<'_> {
    fn returns_lit(&self) -> &str {
        //~^ error: returning a `str` unnecessarily tied to the lifetime of arguments
        "Hello"
    }

    fn returns_non_lit(&self) -> &str {
        self.not_literal
    }

    fn conditionally_returns_lit(&self, cond: bool) -> &str {
        //~^ error: returning a `str` unnecessarily tied to the lifetime of arguments
        if cond { "Literal" } else { "also a literal" }
    }

    fn conditionally_returns_non_lit(&self, cond: bool) -> &str {
        if cond { "Literal" } else { self.not_literal }
    }

    fn contionally_returns_literals_explicit(&self, cond: bool) -> &str {
        //~^ error: returning a `str` unnecessarily tied to the lifetime of arguments
        if cond {
            return "Literal";
        }

        "also a literal"
    }

    fn conditionally_returns_non_lit_explicit(&self, cond: bool) -> &str {
        if cond {
            return self.not_literal;
        }

        "Literal"
    }
}

trait ReturnsStr {
    fn trait_method(&self) -> &str;
}

impl ReturnsStr for u8 {
    fn trait_method(&self) -> &str {
        //~^ error: returning a `str` unnecessarily tied to the lifetime of arguments
        "Literal"
    }
}

impl ReturnsStr for Struct<'_> {
    fn trait_method(&self) -> &str {
        self.not_literal
    }
}

fn main() {}
