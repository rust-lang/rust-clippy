#![feature(plugin)]
#![plugin(clippy)]

#[deny(zero_width_space)]
fn zero() {
    print!("Here >​< is a ZWS, and ​another"); //~ ERROR 2 zero-width space ranges detected.
}

#[deny(unicode_not_nfc)]
fn canon() {
    print!("̀àh?"); //~ERROR non-NFC unicode range detected. Consider using `àh`
}

#[deny(non_ascii_literal)]
fn uni() {
    print!("Üben!"); //~ERROR non-ascii literal range detected. Consider using `\u{dc}`
}

fn main() {
    zero();
    uni();
    canon();
}
