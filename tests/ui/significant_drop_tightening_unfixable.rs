//@no-rustfix
#![warn(clippy::significant_drop_tightening)]

mod issue_15574 {
    use std::io::{BufRead, Read, stdin};
    use std::process;

    // NOTE: this requires `no_rustfix` for two reasons:
    //
    // There should be two suggestions, one to merge the line:
    // ```
    // let stdin = stdin.lock();
    // ```
    // into:
    // ```
    // let mut stdin = stdin.take(40);
    // ```
    // and one to merge the latter into the `if`.
    //
    // That causes the following problems:
    // - the second suggestion isn't a suggestion but a help message, so the warning isn't gone after
    //   rustfix
    // - when the second help becomes a suggestion, it will overlap with the first one
    fn main() {
        //Let's read from stdin
        println!("Hello, what's your name?");
        let stdin = stdin().lock();
        //~^ significant_drop_tightening
        let mut buffer = String::with_capacity(10);
        //Here we lock stdin and block to 10 bytes
        // Our string is now then only 10 bytes.
        //Even if it overflows like expected, it will reallocate.
        let mut stdin = stdin.take(40);
        //~^ significant_drop_tightening
        if stdin.read_line(&mut buffer).is_err() {
            eprintln!("An error has occured while reading.");
            return;
        } //Now we print the result, our data is safe
        println!("Our string has a capacity of {}", buffer.capacity());
        println!("Hello {}!", buffer);
        //The string is freed automatically.
    }
}

fn main() {}
