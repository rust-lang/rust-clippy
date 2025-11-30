#![warn(clippy::manual_checked_div)]

fn main() {
    let a = 10u32;
    let b = 5u32;

    // Should trigger lint
    if b != 0 {
        let _result = a / b;
        //~^ manual_checked_div
    }

    if b > 0 {
        let _result = a / b;
        //~^ manual_checked_div
    }

    if b == 0 {
        println!("zero");
    } else {
        let _result = a / b;
        //~^ manual_checked_div
    }

    // Should NOT trigger (already using checked_div)
    if let Some(result) = b.checked_div(a) {
        println!("{result}");
    }

    // Should NOT trigger (signed integers)
    let c = -5i32;
    if c != 0 {
        let _result = 10 / c;
    }
}
