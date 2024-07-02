#![warn(clippy::unused_format_specs)]
#![allow(unused)]

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

fn main() {
    let f = 1.0f64;
    println!("{:.}", 1.0);
    println!("{f:.} {f:.?}");
    println!("{:.}", 1);

    my_println2!(true, "{:.}", 1.0);
    my_println2!(true, "{f:.} {f:.?}");
    my_println2!(true, "{:.}", 1);

    my_println2_args!(true, "{:.}", 1.0);
    my_println2_args!(true, "{f:.} {f:.?}");
    my_println2_args!(true, "{:.}", 1);
}

fn should_not_lint() {
    let f = 1.0f64;
    println!("{:.1}", 1.0);
    println!("{f:.w$} {f:.*?}", 3, w = 2);

    my_println2!(true, "{:.1}", 1.0);
    my_println2!(true, "{f:.w$} {f:.*?}", 3, w = 2);

    my_println2_args!(true, "{:.1}", 1.0);
    my_println2_args!(true, "{f:.w$} {f:.*?}", 3, w = 2);
}
