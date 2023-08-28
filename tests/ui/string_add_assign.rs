#[allow(clippy::string_add, unused)]
#[warn(clippy::string_add_assign)]
fn main() {
    // ignores assignment distinction
    let mut x = String::new();

    for _ in 1..3 {
        x = x + ".";
        //~^ ERROR: you assigned the result of adding something to this string. Consider u
        //~| NOTE: `-D clippy::string-add-assign` implied by `-D warnings`
        //~| ERROR: manual implementation of an assign operation
        //~| NOTE: `-D clippy::assign-op-pattern` implied by `-D warnings`
    }

    let y = String::new();
    let z = y + "...";

    assert_eq!(&x, &z);

    let mut x = 1;
    x = x + 1;
    //~^ ERROR: manual implementation of an assign operation
    assert_eq!(2, x);
}
