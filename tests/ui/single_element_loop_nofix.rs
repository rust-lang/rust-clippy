//@no-rustfix
#![warn(clippy::single_element_loop)]

fn f(print: bool) {
    for _ in [..5] {
        //~^ single_element_loop
        if print {
            println!("Hello from f");
        }
    }
}
