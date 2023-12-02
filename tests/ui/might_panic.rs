#![allow(clippy::might_panic)]

fn main() {
    // declare 2 array to access
    let arr1 = &[1, 2, 3];
    let arr2 = &[[1], [2], [3]];

    // trigger `might-panic` lint
    // with and without explicit type declaration
    let _num1 = arr1[1]; // warning
    let _num2: i32 = arr1[5]; // warning

    let _num3 = arr2[1]; // warning
    let _num4: [i32; 1] = arr2[5]; // warning

    // not trigger `might-panic` lint
    let _num5 = arr2[0][0]; // no warning
}
