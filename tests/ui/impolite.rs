#![warn(clippy::impolite)]

trait Greeter {
    fn greet(&self);
}

struct HelloWorld;

impl Greeter for HelloWorld {
    fn greet(&self) {
        println!("Hello, world!");
    }
}

fn greet() {
    HelloWorld.greet()
}

fn main() {
    greet();
}
