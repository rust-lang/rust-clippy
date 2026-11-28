#![warn(clippy::from_str_instead_of_from_ascii_bytes)]

fn from_ascii_bytes_radix() {
    let _ = u32::from_ascii_bytes_radix("A", 16).unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let s = "B";
    let _ = u32::from_ascii_bytes_radix(s, 16).unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    use std::num::NonZero;
    let _ = NonZero::<u32>::from_ascii_bytes_radix(s, 16).unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let _ = <NonZero<u32>>::from_ascii_bytes_radix("C", 16).unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let _ = std::num::NonZeroU32::from_ascii_bytes_radix("D", 16).unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let _ = std::num::NonZeroU32::from_ascii_bytes_radix(String::from("E"), 16).unwrap(); // No lint expected

    let _ = std::num::NonZeroU32::from_ascii_bytes_radix(b"F", 16).unwrap(); // No lint expected
}

fn from_ascii_bytes() {
    let _ = u32::from_ascii_bytes("A").unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let s = "B";
    let _ = u32::from_ascii_bytes(s).unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    use std::num::NonZero;
    let _ = NonZero::<u32>::from_ascii_bytes(s).unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let _ = <NonZero<u32>>::from_ascii_bytes("C").unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let _ = std::num::NonZeroU32::from_ascii_bytes("D").unwrap();
    //~^ from_str_instead_of_from_ascii_bytes

    let _ = std::num::NonZeroU32::from_ascii_bytes(String::from("E")).unwrap(); // No lint expected

    let _ = std::num::NonZeroU32::from_ascii_bytes(b"F").unwrap(); // No lint expected
}

fn main() {}
