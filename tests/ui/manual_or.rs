#![warn(clippy::manual_or)]
#[allow(irrefutable_let_patterns)]

fn main() {
    let a = true;
    let b = false;
    let c = true;

    let _ = if a { true } else { b }; //~ ERROR: this `if`-then-`else` expression can be simplified with `||`

    let _ = if a {
        true
    } else if b {
        false
    } else {
        c
    };

    let _ = if a { true } else { !b };
    //~^ ERROR: this `if`-then-`else` expression can be simplified with `||`

    let _ = if !a { true } else { b };
    //~^ ERROR: this `if`-then-`else` expression can be simplified with `||`

    let _ = if let x = a { true } else { b };
}
