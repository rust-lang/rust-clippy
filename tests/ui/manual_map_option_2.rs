// edition:2018
// run-rustfix

#![warn(clippy::manual_map)]
#![allow(
    clippy::no_effect,
    clippy::map_identity,
    clippy::unit_arg,
    clippy::match_ref_pats,
    clippy::redundant_pattern_matching,
    dead_code
)]

fn main() {
    fn f1(s: String) -> String {
        s
    }

    let s = String::new();
    // Ok, `s` is consumed.
    let _: Option<String> = match Some(0) {
        Some(_) => Some(f1(s)),
        None => None,
    };

    let s = String::new();
    let s2 = &String::new();
    // Ok, `s` is consumed, and `s2` is copied.
    let _: Option<(String, &str)> = match Some(0) {
        Some(_) => Some((f1(s), s2.as_str())),
        None => None,
    };
    println!("{}", s2);

    let s = String::new();
    let s2 = &mut String::new();
    // Ok, `s` is borrowed, and `s2` is reborrowed.
    let _: Option<(&str, &str)> = match Some(0) {
        Some(_) => Some((s.as_str(), s2.as_str())),
        None => None,
    };
    println!("{}", s2);

    fn f2(s: &mut String) -> &mut str {
        s
    }

    let s = String::new();
    let s2 = &mut String::new();
    // Ok, `s` is borrowed, and `s2` is reborrowed.
    let _: Option<(&str, &mut str)> = match Some(0) {
        Some(_) => Some((s.as_str(), f2(s2))),
        None => None,
    };
    println!("{}", s2);

    let v = vec![0];
    // Ok, `v` is borrowed.
    let _: Option<u32> = match Some(0) {
        Some(i) => Some(v[i]),
        None => None,
    };
    println!("{}", v[0]);

    // Ok, no need for a move closure.
    let _: Option<String> = match Some(0) {
        Some(_) => Some({
            let s = String::new();
            f1(s)
        }),
        None => None,
    };

    let s = String::new();
    let x = 0u32;
    // Can't use map, `s` is consumed, but `x` is borrowed.
    let _: Option<(String, &u32)> = match Some(0) {
        Some(_) => Some((f1(s), &x)),
        None => None,
    };
    println!("{}", v[0]);

    let s = String::new();
    let s2 = String::new();
    // Can't use map, `s` is consumed, but `s2` is used afterwards.
    let _: Option<(String, &str)> = match Some(0) {
        Some(_) => Some((f1(s), s2.as_str())),
        None => None,
    };
    println!("{}", s2);

    let s = String::new();
    let s2 = String::new();
    // Can't use map, `s` is consumed, but `s2` is used afterwards.
    let _: Option<(String, &String)> = match Some(0) {
        Some(_) => Some((f1(s), &s2)),
        None => None,
    };
    println!("{}", s2);

    let s = String::new();
    let s2 = &mut String::new();
    // Can't use map, `s` is consumed, but `s2` is used afterwards.
    let _: Option<(String, &str)> = match Some(0) {
        Some(_) => Some((f1(s), s2.as_str())),
        None => None,
    };
    println!("{}", s2);

    let s = String::new();
    let v = vec![0];
    // Can't use map, `s` is consumed, but `v` is used afterwards.
    let _: Option<(String, u32)> = match Some(0) {
        Some(i) => Some((f1(s), v[i])),
        None => None,
    };
    println!("{}", v[0]);
}
