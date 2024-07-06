//@aux-build:proc_macros.rs
//@no-rustfix

#[deny(clippy::split_with_space)]
#[allow(unused)]
fn main() {
    let some_space_delimtetered_string = "Hello everyone! How are you doing?";

    for substr in some_space_delimtetered_string.split(' ') {
        println!("{substr}");
    }

    for substr in some_space_delimtetered_string.split(" ") {
        println!("{substr}");
    }
}
