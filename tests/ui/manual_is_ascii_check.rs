#![allow(unused, dead_code)]
#![warn(clippy::manual_is_ascii_check)]

fn main() {
    assert!(matches!('x', 'a'..='z'));
    //~^ ERROR: manual check for common ascii range
    //~| NOTE: `-D clippy::manual-is-ascii-check` implied by `-D warnings`
    assert!(matches!('X', 'A'..='Z'));
    //~^ ERROR: manual check for common ascii range
    assert!(matches!(b'x', b'a'..=b'z'));
    //~^ ERROR: manual check for common ascii range
    assert!(matches!(b'X', b'A'..=b'Z'));
    //~^ ERROR: manual check for common ascii range

    let num = '2';
    assert!(matches!(num, '0'..='9'));
    //~^ ERROR: manual check for common ascii range
    assert!(matches!(b'1', b'0'..=b'9'));
    //~^ ERROR: manual check for common ascii range
    assert!(matches!('x', 'A'..='Z' | 'a'..='z'));
    //~^ ERROR: manual check for common ascii range

    assert!(matches!('x', 'A'..='Z' | 'a'..='z' | '_'));

    (b'0'..=b'9').contains(&b'0');
    //~^ ERROR: manual check for common ascii range
    (b'a'..=b'z').contains(&b'a');
    //~^ ERROR: manual check for common ascii range
    (b'A'..=b'Z').contains(&b'A');
    //~^ ERROR: manual check for common ascii range

    ('0'..='9').contains(&'0');
    //~^ ERROR: manual check for common ascii range
    ('a'..='z').contains(&'a');
    //~^ ERROR: manual check for common ascii range
    ('A'..='Z').contains(&'A');
    //~^ ERROR: manual check for common ascii range

    let cool_letter = &'g';
    ('0'..='9').contains(cool_letter);
    //~^ ERROR: manual check for common ascii range
    ('a'..='z').contains(cool_letter);
    //~^ ERROR: manual check for common ascii range
    ('A'..='Z').contains(cool_letter);
    //~^ ERROR: manual check for common ascii range
}

#[clippy::msrv = "1.23"]
fn msrv_1_23() {
    assert!(matches!(b'1', b'0'..=b'9'));
    assert!(matches!('X', 'A'..='Z'));
    assert!(matches!('x', 'A'..='Z' | 'a'..='z'));
}

#[clippy::msrv = "1.24"]
fn msrv_1_24() {
    assert!(matches!(b'1', b'0'..=b'9'));
    //~^ ERROR: manual check for common ascii range
    assert!(matches!('X', 'A'..='Z'));
    //~^ ERROR: manual check for common ascii range
    assert!(matches!('x', 'A'..='Z' | 'a'..='z'));
    //~^ ERROR: manual check for common ascii range
}

#[clippy::msrv = "1.46"]
fn msrv_1_46() {
    const FOO: bool = matches!('x', '0'..='9');
}

#[clippy::msrv = "1.47"]
fn msrv_1_47() {
    const FOO: bool = matches!('x', '0'..='9');
    //~^ ERROR: manual check for common ascii range
}
