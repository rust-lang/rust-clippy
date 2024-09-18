#![warn(clippy::unnecessary_semicolon)]
#![feature(postfix_match)]

fn no_lint(mut x: u32) -> Option<u32> {
    Some(())?;

    {
        let y = 3;
        dbg!(x + y)
    };

    {
        let (mut a, mut b) = (10, 20);
        (a, b) = (b + 1, a + 1);
    }

    Some(0)
}

fn main() {
    let mut a = 3;
    if a == 2 {
        println!("This is weird");
    };
    //~^ ERROR: unnecessary semicolon

    a.match {
        3 => println!("three"),
        _ => println!("not three"),
    };
    //~^ ERROR: unnecessary semicolon
}
