#![warn(clippy::char_lit_as_u8)]

fn main() {
    let _ = 'a' as u8;
    //~^ ERROR: casting a character literal to `u8` truncates
    //~| NOTE: `char` is four bytes wide, but `u8` is a single byte
    let _ = '\n' as u8;
    //~^ ERROR: casting a character literal to `u8` truncates
    //~| NOTE: `char` is four bytes wide, but `u8` is a single byte
    let _ = '\0' as u8;
    //~^ ERROR: casting a character literal to `u8` truncates
    //~| NOTE: `char` is four bytes wide, but `u8` is a single byte
    let _ = '\x01' as u8;
    //~^ ERROR: casting a character literal to `u8` truncates
    //~| NOTE: `char` is four bytes wide, but `u8` is a single byte
}
