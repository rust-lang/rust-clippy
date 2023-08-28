#[allow(clippy::no_effect, clippy::unnecessary_operation)]
#[warn(clippy::int_plus_one)]
fn main() {
    let x = 1i32;
    let y = 0i32;

    let _ = x >= y + 1;
    //~^ ERROR: unnecessary `>= y + 1` or `x - 1 >=`
    //~| NOTE: `-D clippy::int-plus-one` implied by `-D warnings`
    let _ = y + 1 <= x;
    //~^ ERROR: unnecessary `>= y + 1` or `x - 1 >=`

    let _ = x - 1 >= y;
    //~^ ERROR: unnecessary `>= y + 1` or `x - 1 >=`
    let _ = y <= x - 1;
    //~^ ERROR: unnecessary `>= y + 1` or `x - 1 >=`

    let _ = x > y; // should be ok
    let _ = y < x; // should be ok
}
