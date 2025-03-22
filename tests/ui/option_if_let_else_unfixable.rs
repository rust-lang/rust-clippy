//@no-rustfix
#![warn(clippy::option_if_let_else)]

mod issue11059 {
    use std::fmt::Debug;

    fn box_coercion_unsize(o: Option<i32>) -> Box<dyn Debug> {
        if let Some(o) = o { Box::new(o) } else { Box::new("foo") }
        //~^ option_if_let_else
    }

    static S: String = String::new();

    fn deref_with_overload(o: Option<&str>) -> &str {
        if let Some(o) = o { o } else { &S }
        //~^ option_if_let_else
    }
}

fn main() {}
