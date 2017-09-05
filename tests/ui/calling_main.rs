#![feature(plugin)]
#![plugin(clippy)]
#![deny(calling_main)]

fn main() {}

#[allow(dead_code)]
fn calling_main() {
    main(); //~ERROR calling into main() detected
}

