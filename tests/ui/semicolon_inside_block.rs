#![allow(
    unused,
    clippy::unused_unit,
    clippy::unnecessary_operation,
    clippy::no_effect,
    clippy::single_element_loop,
    clippy::double_parens
)]
#![warn(clippy::semicolon_inside_block)]

macro_rules! m {
    (()) => {
        ()
    };
    (0) => {{
        0
    };};
    (1) => {{
        1;
    }};
    (2) => {{
        2;
    }};
}

fn unit_fn_block() {
    ()
}

#[rustfmt::skip]
fn main() {
    { unit_fn_block() }
    unsafe { unit_fn_block() }

    {
        unit_fn_block()
    }

    { unit_fn_block() };
    //~^ semicolon_inside_block
    unsafe { unit_fn_block() };
    //~^ semicolon_inside_block

    { unit_fn_block(); }
    unsafe { unit_fn_block(); }

    { unit_fn_block(); };
    unsafe { unit_fn_block(); };

    {
    //~^ semicolon_inside_block
        unit_fn_block();
        unit_fn_block()
    };
    {
        unit_fn_block();
        unit_fn_block();
    }
    {
        unit_fn_block();
        unit_fn_block();
    };

    { m!(()) };
    //~^ semicolon_inside_block
    { m!(()); }
    { m!(()); };
    m!(0);
    m!(1);
    m!(2);

    for _ in [()] {
        unit_fn_block();
    }
    for _ in [()] {
        unit_fn_block()
    }

    let _d = || {
        unit_fn_block();
    };
    let _d = || {
        unit_fn_block()
    };

    { unit_fn_block(); };

    unit_fn_block()
}

// TODO: merge the function bodies once https://github.com/rust-lang/rust-clippy/issues/15389 is fixed
//
// Right now, if `first` and `second` were to be merged (uitest comments omitted for clarity):
// ``` fn issue15380() {
//     ( {0;0});
//     ({
//         0;
//         0
//     });
// }
// ```
// then the fixed version would look as follows:
// ```
// fn issue15380() {
//     ( {0;0;})
//     ({
//         0;
//         0
//     });
// }
// ```
// However, that looks like a function call `(f)(x)`, and so we get an error because `{0;0;}` is
// not a function.
mod issue15380 {
    // TODO: apply `[rustfmt::skip]` only to the block once https://github.com/rust-lang/rust-clippy/issues/15388 is fixed
    //
    // If we do this right now:
    // ```
    // fn issue15380() {
    //     #[rustfmt::skip]
    //     ( {0;0});
    // }
    // ```
    // then the fixed version would look as follows:
    // ```
    // fn issue15380() {
    //     #[rustfmt::skip]
    //     ( {0;0;})
    // }
    // ```
    // But that wouldn't compile because `[rustfmt::skip]` is now placed on an expr
    #[rustfmt::skip]
    fn first() {
        ( {0;0});
        //~^ semicolon_inside_block
    }

    fn second() {
        ({
            //~^ semicolon_inside_block
            0;
            0
        });
    }

    #[rustfmt::skip] // TODO: apply onto the block itself, see explanation in `first`
    fn third() {
        (({ 0 }))      ;
        //~^ semicolon_inside_block
    }
}
