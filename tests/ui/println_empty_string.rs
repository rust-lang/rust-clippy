#![allow(clippy::match_single_binding)]

fn main() {
    println!();
    println!("");
    //~^ println_empty_string

    match "a" {
        _ => println!(""),
        //~^ println_empty_string
    }

    eprintln!();
    eprintln!("");
    //~^ println_empty_string

    match "a" {
        _ => eprintln!(""),
        //~^ println_empty_string
    }
}

fn issue_16167() {
    println!("",);
    //~^ println_empty_string
    match "a" {
        _ => println!("",),
        //~^ println_empty_string
    }

    eprintln!("",);
    //~^ println_empty_string
    match "a" {
        _ => eprintln!("",),
        //~^ println_empty_string
    }

    //~v println_empty_string
    println!(
        "\
                \
                ",
    );
    match "a" {
        //~v println_empty_string
        _ => println!(
            "\
                    \
                    ",
        ),
    }

    //~v println_empty_string
    eprintln!(
        "\
                \
                ",
    );
    match "a" {
        //~v println_empty_string
        _ => eprintln!(
            "\
                    \
                    ",
        ),
    }
}
