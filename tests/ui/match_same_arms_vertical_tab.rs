#![warn(clippy::match_same_arms)]
#![allow(clippy::manual_range_patterns)]

fn main() {
    let x = 1;

    // ================= ASCII WHITESPACE =================

    // Space
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {},
    }

    // Horizontal tab \t
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {},
    }

    // ================= IMPORTANT BUG TARGET =================

    // Vertical tab (U+000B) ← MAIN BUG YOU FIXED
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {},
    }

    // ================= OTHER NON-ASCII WHITESPACE =================

    // Form feed (U+000C)
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {},
    }

    // Next line (U+0085)
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {},
    }

    // ================= UNICODE WHITESPACE =================

    // Left-to-right mark (U+200E)
    match x {
        1 => println!("same"),‎ //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {}
    }

    // Right-to-left mark (U+200F)
    match x {
        1 => println!("same"),‏ //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {}
    }

    // Line separator (U+2028)
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {},
    }

    // Paragraph separator (U+2029)
    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        _ => {},
    }

    // ================= MULTIPLE DUPLICATE ARMS =================

    match x {
        1 => println!("same"), //~ ERROR: these match arms have identical bodies
        2 => println!("same"),
        3 => println!("same"),
        _ => {},
    }
}
