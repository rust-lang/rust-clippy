#![warn(clippy::uninlined_format_args)]
#![allow(named_arguments_used_positionally, unused_imports, unused_macros, unused_variables)]
#![allow(clippy::eq_op, clippy::format_in_format_args, clippy::print_literal)]

macro_rules! _internal {
    ($($args:tt)*) => {
        println!("{}", format_args!($($args)*))
    };
}

macro_rules! my_println2 {
   ($target:expr, $($args:tt)+) => {{
       if $target {
           _internal!($($args)+)
       }
    }};
}

macro_rules! my_println2_args {
    ($target:expr, $($args:tt)+) => {{
       if $target {
           _internal!("foo: {}", format_args!($($args)+))
       }
    }};
}

macro_rules! my_concat {
    ($fmt:literal $(, $e:expr)*) => {
        println!(concat!("ERROR: ", $fmt), $($e,)*)
    }
}

macro_rules! my_good_macro {
    ($fmt:literal $(, $e:expr)* $(,)?) => {
        println!($fmt $(, $e)*)
    }
}

macro_rules! my_bad_macro {
    ($fmt:literal, $($e:expr),*) => {
        println!($fmt, $($e,)*)
    }
}

macro_rules! my_bad_macro2 {
    ($fmt:literal) => {
        let s = $fmt.clone();
        println!("{}", s);
    };
    ($fmt:literal, $($e:expr)+) => {
        println!($fmt, $($e,)*)
    };
}

// This abomination was suggested by @Alexendoo, may the Rust gods have mercy on their soul...
// https://github.com/rust-lang/rust-clippy/pull/9948#issuecomment-1327965962
macro_rules! used_twice {
    (
        large = $large:literal,
        small = $small:literal,
        $val:expr,
    ) => {
        if $val < 5 {
            println!($small, $val);
        } else {
            println!($large, $val);
        }
    };
}

fn main() {
    let local_i32 = 1;
    println!("val='{}'", local_i32);
    my_println2_args!(true, "{}", local_i32);
    my_println2!(true, "{}", local_i32);
    my_concat!("{}", local_i32);
    my_good_macro!("{}", local_i32);
    my_good_macro!("{}", local_i32,);

    // FIXME: Broken false positives, currently unhandled
    // my_bad_macro!("{}", local_i32);
    // my_bad_macro2!("{}", local_i32);
    // used_twice! {
    //     large = "large value: {}",
    //     small = "small value: {}",
    //     local_i32,
    // };
}
