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
    //~^ constant_bool_expr
    let _b = a == A && a == 101;
    //~^ constant_bool_expr

    let _b = a != A || a != const { 100 + 1 };
    //~^ constant_bool_expr
    let _b = a == A && a == const { 100 + 1 };
    //~^ constant_bool_expr
}

const B: i32 = 100;
const C: i32 = 99;

const _: bool = C != B || C != const { 100 + 1 };
//~^ constant_bool_expr
const _: bool = C == B && C == const { 100 + 1 };
//~^ constant_bool_expr

static D: bool = C != B || C != const { 100 + 1 };
//~^ constant_bool_expr
static E: bool = C == B && C == const { 100 + 1 };
//~^ constant_bool_expr
