// Test for useless_format_width lint (issue #15039)
// Format width has no effect on output for certain traits when it's too small.

#![warn(clippy::useless_format_width)]
#![allow(clippy::zero_ptr, clippy::manual_dangling_ptr)]

fn main() {
    // Integer formats with # (alternate): 0x/0o/0b prefix makes min width 3
    println!("{:#02X}", 1u8); //~ ERROR: format width has no effect on the output
    println!("{:#2X}", 1u8); //~ ERROR: format width has no effect on the output
    println!("{:#02x}", 1u8); //~ ERROR: format width has no effect on the output
    println!("{:#02o}", 1u8); //~ ERROR: format width has no effect on the output
    println!("{:#02b}", 1u8); //~ ERROR: format width has no effect on the output

    // Exponent formats: min width 4 (e.g. 1e0)
    println!("{:02e}", 1u8); //~ ERROR: format width has no effect on the output
    println!("{:02E}", 1u8); //~ ERROR: format width has no effect on the output
    println!("{:2e}", 1.0); //~ ERROR: format width has no effect on the output
    println!("{:2E}", 1.0); //~ ERROR: format width has no effect on the output
    println!("{:2e}", 0.1); //~ ERROR: format width has no effect on the output
    println!("{:2E}", 0.1); //~ ERROR: format width has no effect on the output

    // Pointer: min width 3 (0x0)
    println!("{:2p}", 0 as *const usize); //~ ERROR: format width has no effect on the output
    println!("{:02p}", 1 as *const usize); //~ ERROR: format width has no effect on the output

    // Width 2 still too small for exponent (min 3); precision+width
    println!("{:2.2e}", 1.0); //~ ERROR: format width has no effect on the output
    println!("{:2.2E}", 1.0); //~ ERROR: format width has no effect on the output
    println!("{:2.2e}", 0.1); //~ ERROR: format width has no effect on the output
    println!("{:2.2E}", 0.1); //~ ERROR: format width has no effect on the output

    // Width 3 is exactly the minimum for exponent, still warn as it is unlikely to be intentional
    println!("{:#03X}", 1u8); //~ ERROR: format width has no effect on the output

    // Not linted: width more than 3, or no # for x/o/b
    println!("{:#04X}", 1u8);
    println!("{:2X}", 1u8); // no #, so no prefix
    println!("{:2o}", 1u8);
    println!("{}", 1);
}
