#![warn(clippy::items_after_statements)]
#![expect(clippy::uninlined_format_args)]

fn ok() {
    fn foo() {
        println!("foo");
    }
    foo();
}

fn last() {
    foo();
    fn foo() {
        //~^ items_after_statements

        println!("foo");
    }
}

fn main() {
    foo();
    fn foo() {
        //~^ items_after_statements

        println!("foo");
    }
    foo();
}

fn mac() {
    let mut a = 5;
    println!("{}", a);
    // do not lint this, because it needs to be after `a`
    macro_rules! b {
        () => {{
            a = 6;
            fn say_something() {
                //~^ items_after_statements
                println!("something");
            }
        }};
    }
    b!();
    println!("{}", a);
}

fn semicolon() {
    struct S {
        a: u32,
    };
    impl S {
        fn new(a: u32) -> Self {
            Self { a }
        }
    }

    let _ = S::new(3);
}

fn item_from_macro() {
    macro_rules! static_assert_size {
        ($ty:ty, $size:expr) => {
            const _: [(); $size] = [(); ::std::mem::size_of::<$ty>()];
        };
    }

    let _ = 1;
    static_assert_size!(u32, 4);
}

fn cfg_select_arm() {
    let x = 1;
    // `cfg_select!` splices the tokens of the arm into this block, without any expansion marker
    std::cfg_select! {
        true => {
            use std::{cmp, mem};
            let _ = (x, cmp::max(0, 1), mem::size_of::<u8>());
        },
    }
}

fn item_after_cfg_select() {
    let x = 1;
    std::cfg_select! {
        true => {
            use std::{cmp, mem};
            let _ = (x, cmp::max(0, 1), mem::size_of::<u8>());
        },
    }
    fn foo() {}
    //~^ items_after_statements
}

fn grouped_use_after_statement() {
    let x = 1;
    use std::cmp::{max, min};
    //~^ items_after_statements
    //~| items_after_statements
    //~| items_after_statements
    let _ = (x, max(0, 1), min(0, 1));
}

fn nested_block() {
    let _ = 1;
    {
        let _ = 2;
        fn foo() {}
        //~^ items_after_statements
    }
}

fn allow_attribute() {
    let _ = 1;
    #[allow(clippy::items_after_statements)]
    const _: usize = 1;
}
