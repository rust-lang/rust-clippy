#![warn(clippy::constant_bool_expr)]
#![expect(clippy::needless_ifs)]

fn main() {
    let a = 99;

    if a != 100 || a != 101 {}
    //~^ constant_bool_expr
    if a == 100 && a == 101 {}
    //~^ constant_bool_expr

    let _b = a != 100 || a != 101;
    //~^ constant_bool_expr
    let _b = a == 100 && a == 101;
    //~^ constant_bool_expr

    const A: i32 = 100;

    let _b = a != A || a != 101;
    let _b = a == A && a == 101;
}
