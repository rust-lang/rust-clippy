#![warn(clippy::manual_next_multiple_of)]

fn f_val() -> u32 {
    todo!()
}

macro_rules! identity {
    ( $t:tt ) => {
        $t
    };
}

fn main() {
    let u1 = 1_u32;
    let u2 = 2_u32;

    // basic cases
    let _ = u1.div_ceil(u2) * u2; //~ manual_next_multiple_of
    let _ = u2 * u1.div_ceil(u2); //~ manual_next_multiple_of
    let _ = u1 + (u2 - u1 % u2) % u2; //~ manual_next_multiple_of
    let _ = (u2 - u1 % u2) % u2 + u1; //~ manual_next_multiple_of

    // special cases: function and macro may have side effect, so must be ignored
    let _ = u1.div_ceil(f_val()) * f_val();
    let _ = u1 + (f_val() - u1 % f_val()) % f_val();
    let _ = f_val() + (u2 - f_val() % u2) % u2;

    let _ = u1.div_ceil(identity!(u2)) * identity!(u2);
    let _ = u1 + (identity!(u2) - u1 % identity!(u2)) % identity!(u2);
    let _ = identity!(u1) + (u2 - identity!(u1) % u2) % u2;

    // This line ignore manual `div_ceil` implementation
    #[expect(clippy::manual_div_ceil)]
    let _ = (u1 + u2 - 1) / u2 * u2;
}
