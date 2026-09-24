//@no-rustfix: the suggestion can change borrow-check results for boxed types carrying a lifetime
#![warn(clippy::replace_box)]

// Replacing the box in place keeps any borrow already stored in it alive across the assignment, so
// for a boxed type that carries a lifetime the suggestion is only `MaybeIncorrect`, not applied.

fn mut_ref() {
    let mut a = 300;
    let mut b = Box::new(&mut a);
    **b += 1;
    let mut c = 900;
    b = Box::new(&mut c);
    //~^ replace_box
    **b += 1;
}

fn shared_ref() {
    let x = 1;
    let mut b = Box::new(&x);
    let y = 2;
    b = Box::new(&y);
    //~^ replace_box
    let _ = **b;
}

struct Wrapper<'a>(&'a u32);

fn lifetime_struct() {
    let x = 1;
    let mut b = Box::new(Wrapper(&x));
    let y = 2;
    b = Box::new(Wrapper(&y));
    //~^ replace_box
    let _ = *b.0;
}

fn main() {}
