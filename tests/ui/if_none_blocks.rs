#![warn(clippy::if_none_blocks)]

fn side_effects() -> u32 {
    println!("Side effect.");
    8
}

fn main() {
    let a = true;
    let b = Some(8);
    let c = Some(10);
    let d = false;

    // Eager Positives
    let _ = if a { Some(8) } else { None };

    #[allow(unused_braces)]
    let _ = if a {
        {
            {
                let x = 8;
                Some(x)
            }
        }
    } else {
        {
            {
                { None }
            }
        }
    };

    let _ = if a {
        Some(8)
    } else if d {
        Some(10)
    } else {
        None
    };

    // Flipped Eager Positives
    let _ = if a { None } else { Some(8) };

    // Lazy Positives
    let _ = if a { Some(side_effects()) } else { None };

    let _ = if a {
        side_effects();
        Some(8)
    } else {
        None
    };

    // Flipped Lazy Positives
    let _ = if a { None } else { Some(side_effects()) };

    // Negatives
    let _ = if a && let Some(_) = b && let Some(_) = c {
        Some(8)
    } else {
        None
    };

    let _ = if a { "Some" } else { "None" };

    // Intentionally left as negative
    let _: Option<u8> = if a { panic!() } else { None };
}
