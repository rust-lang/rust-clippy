#[macro_export]
macro_rules! fallible_macro {
    ( $x:expr ) => {{
        let _ = $x;
        panic!("a");
    }};
}

#[macro_export]
macro_rules! non_fallible_macro {
    ( $x:expr ) => {{
        let _ = $x;
    }};
}
