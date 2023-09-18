#![allow(dead_code)]
#![warn(clippy::ambiguous_method_calls)]

trait MyTrait {
    fn ambiguous(&self);
}

struct Base;

impl Base {
    fn ambiguous(&self) {
        println!("struct impl");
    }

    fn unambiguous(&self) {
        println!("unambiguous struct impl");
    }
}

impl MyTrait for Base {
    fn ambiguous(&self) {
        println!("trait impl");
    }
}

struct Other;

impl MyTrait for Other {
    fn ambiguous(&self) {
        println!("not actually ambiguous")
    }
}

fn main() {
    Base.ambiguous();
    Other.ambiguous();
}
