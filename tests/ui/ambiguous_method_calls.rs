#![allow(dead_code)]
#![warn(clippy::ambiguous_method_calls)]

fn main() {
    Base.ambiguous();
    Base.ambiguous();
    Base.also_ambiguous();

    Base.unambiguous();

    Other.ambiguous();
    Other.also_ambiguous();
}

trait MyTrait {
    fn ambiguous(&self);
    fn also_ambiguous(&self);
}

struct Base;

impl Base {
    fn ambiguous(&self) {
        println!("ambiguous struct impl");
    }

    fn also_ambiguous(&self) {}

    fn unambiguous(&self) {
        println!("unambiguous struct impl");
    }
}

impl MyTrait for Base {
    fn ambiguous(&self) {
        println!("ambiguous trait impl");
    }

    fn also_ambiguous(&self) {}
}

struct Other;

impl MyTrait for Other {
    fn ambiguous(&self) {
        println!("not actually ambiguous")
    }

    fn also_ambiguous(&self) {
        println!("not actually ambiguous either")
    }
}
