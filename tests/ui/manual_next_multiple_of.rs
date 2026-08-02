#![warn(clippy::manual_next_multiple_of)]

use std::hint::black_box;

fn out_of_scope(u1: u32, u2: u32) {
    // This lint should not detect manual `div_ceil` implementation.
    #[expect(clippy::manual_div_ceil)]
    let _ = (u1 + u2 - 1) / u2 * u2;
}

fn basic(u1: u32, u2: u32) {
    let _ = u1.div_ceil(u2) * u2; //~ manual_next_multiple_of
    let _ = u2 * u1.div_ceil(u2); //~ manual_next_multiple_of
    let _ = u1 + (u2 - u1 % u2) % u2; //~ manual_next_multiple_of
    let _ = (u2 - u1 % u2) % u2 + u1; //~ manual_next_multiple_of
}

fn power_of_two(u1: u32) {
    let _ = (u1 + 3) & !3; //~ manual_next_multiple_of
    let _ = !3 & (u1 + 3); //~ manual_next_multiple_of
    let _ = (u1 + 3) & 0xffff_fffc; //~ manual_next_multiple_of
    let _ = 0xffff_fffc & (u1 + 3); //~ manual_next_multiple_of

    // These two cases will be ignored because they are useless
    #[expect(clippy::identity_op)]
    let _ = (u1 + 0) & !0;
    #[expect(clippy::erasing_op)]
    let _ = (u1 + !0) & 0;
}

fn checked_ops(u1: u32, u2: u32) {
    let _ = u1.div_ceil(u2).checked_mul(u2); //~ manual_next_multiple_of
    let _ = u2.checked_mul(u1.div_ceil(u2)); //~ manual_next_multiple_of
    let _ = u1.checked_add((u2 - u1 % u2) % u2); //~ manual_next_multiple_of
    let _ = ((u2 - u1 % u2) % u2).checked_add(u1); //~ manual_next_multiple_of
}

fn checked_ops_with_try(u1: u32, u2: u32) -> Option<u32> {
    let _ = u1.div_ceil(u2).checked_mul(u2)?; //~ manual_next_multiple_of
    let _ = u1.checked_add((u2 - u1.checked_rem(u2)?).checked_rem(u2)?)?;
    //~^ manual_next_multiple_of

    None
}

fn function_call(u1: u32, u2: u32) {
    // The lint should NOT be triggered because:
    // 1. function or method calls may have side effects (they can modify interior or global state)
    // 2. the return value may differ on each invocation
    let _ = u1.div_ceil(black_box(u2)) * black_box(u2);

    // In contrast, this pattern should be linted because function is invoked only once.
    let _ = black_box(u1).div_ceil(u2) * u2; //~ manual_next_multiple_of
}

fn macros(u1: u32, u2: u32) {
    macro_rules! may_have_side_effect {
        ( $e:expr ) => {{
            black_box(());
            $e
        }};
    }
    // See `function_call` above for details
    let _ = u1.div_ceil(may_have_side_effect!(u2)) * may_have_side_effect!(u2);
    let _ = may_have_side_effect!(u1).div_ceil(u2) * u2; //~ manual_next_multiple_of

    // This macro cannot be detected because span is not updated during expansion.
    macro_rules! identity {
        ( $e:expr ) => {
            $e
        };
    }
    // This is an inevitable false positive because no-op macro is transparent as pointed above.
    // See <https://github.com/rust-lang/rust-clippy/pull/17472#discussion_r3714247207>.
    let _ = u1.div_ceil(identity!(u2)) * identity!(u2); //~ manual_next_multiple_of
}
