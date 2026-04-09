#![warn(clippy::match_same_arms)]

fn multiple_duplicate_groups() {
    let y = 5;

    match y {
        1 => println!("same"),
        2 => println!("same"), //~ ERROR: these match arms have identical bodies
        3 => println!("different"),
        4 => println!("same"),
        5 => println!("same"),
        _ => println!("other"),
    }
}
