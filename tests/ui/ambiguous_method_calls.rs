#![allow(dead_code)]
#![warn(clippy::ambiguous_method_calls)]

fn main() {
    Base.ambiguous();
    Base.ambiguous();
    Base.also_ambiguous();

    Base.unambiguous();

    Other.ambiguous();
    Other.also_ambiguous();

    Base.another();

    ambiguous();
}

fn ambiguous() {}

trait MyTrait {
    fn ambiguous(&self);
    fn also_ambiguous(&self);
}

trait Another {
    fn another(&self);
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

    fn another(&self) {}
}

impl MyTrait for Base {
    fn ambiguous(&self) {
        println!("ambiguous trait impl");
    }

    fn also_ambiguous(&self) {}
}

impl Another for Base {
    fn another(&self) {}
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
