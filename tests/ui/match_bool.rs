//@no-rustfix: overlapping suggestions
#![deny(clippy::match_bool)]

fn match_bool() {
    let test: bool = true;

    match test {
    //~^ nonminimal_bool
        true => 0,
        false => 42,
    };

    let option = 1;
    match option == 1 {
    //~^ nonminimal_bool
        true => 1,
        false => 0,
    };

    match test {
    //~^ nonminimal_bool
        true => (),
        false => {
            println!("Noooo!");
        },
    };

    match test {
    //~^ nonminimal_bool
        false => {
            println!("Noooo!");
        },
        _ => (),
    };

    match test && test {
    //~^ nonminimal_bool
    //~| nonminimal_bool
    //~| eq_op
        false => {
            println!("Noooo!");
        },
        _ => (),
    };

    match test {
    //~^ nonminimal_bool
        false => {
            println!("Noooo!");
        },
        true => {
            println!("Yes!");
        },
    };

    // Not linted
    match option {
        1..=10 => 1,
        11..=20 => 2,
        _ => 3,
    };

    // Don't lint
    let _ = match test {
        #[cfg(feature = "foo")]
        true if option == 5 => 10,
        true => 0,
        false => 1,
    };
}

fn main() {}
