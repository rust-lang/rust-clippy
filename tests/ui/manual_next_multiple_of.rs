#![warn(clippy::manual_next_multiple_of)]

fn f_val() -> u32 {
    todo!()
}

// This macro cannot be detected because span is not updated during expansion.
// However, this rarely matters in practice.
macro_rules! identity {
    ( $e:expr ) => {
        $e
    };
}

macro_rules! may_have_side_effect {
    ( $e:expr ) => {{
        f_val();
        $e
    }};
}

fn main() {
    let u1 = 1_u32;
    let u2 = 2_u32;

    // basic cases
    let _ = u1.div_ceil(u2) * u2; //~ manual_next_multiple_of
    let _ = u2 * u1.div_ceil(u2); //~ manual_next_multiple_of
    let _ = u1 + (u2 - u1 % u2) % u2; //~ manual_next_multiple_of
    let _ = (u2 - u1 % u2) % u2 + u1; //~ manual_next_multiple_of

    // checked ops
    let _ = u1.div_ceil(u2).checked_mul(u2); //~ manual_next_multiple_of
    let _ = u2.checked_mul(u1.div_ceil(u2)); //~ manual_next_multiple_of
    let _ = u1.checked_add((u2 - u1 % u2) % u2); //~ manual_next_multiple_of
    let _ = ((u2 - u1 % u2) % u2).checked_add(u1); //~ manual_next_multiple_of

    // power of two
    let _ = (u1 + 3) & !3; //~ manual_next_multiple_of
    let _ = (u1 + 7) & !7; //~ manual_next_multiple_of

    // function and macro
    let _ = u1.div_ceil(f_val()) * f_val();
    let _ = f_val().div_ceil(u2) * u2; //~ manual_next_multiple_of
    // False positive
    let _ = u1.div_ceil(identity!(u2)) * identity!(u2); //~ manual_next_multiple_of
    let _ = identity!(u1).div_ceil(u2) * u2; //~ manual_next_multiple_of

    let _ = u1.div_ceil(may_have_side_effect!(u2)) * may_have_side_effect!(u2);
    let _ = may_have_side_effect!(u1).div_ceil(u2) * u2; //~ manual_next_multiple_of

    // This lint ignore manual `div_ceil` implementation
    #[expect(clippy::manual_div_ceil)]
    let _ = (u1 + u2 - 1) / u2 * u2;
}
