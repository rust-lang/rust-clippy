//@ check-pass

enum A {
    A,
}

enum Never {}

fn issue17790() {
    let _: A = unsafe { std::mem::transmute(issue17790) };
}

fn uninhabited() {
    let _: Never = unsafe { std::mem::transmute(uninhabited) };
}

fn main() {}
