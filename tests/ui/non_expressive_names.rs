#![warn(clippy::just_underscores_and_digits)]
#![expect(non_snake_case)]

#[derive(Clone, Debug)]
enum MaybeInst {
    Split,
    Split1(usize),
    Split2(usize),
}

struct InstSplit {
    uiae: usize,
}

impl MaybeInst {
    fn fill(&mut self) {
        #[allow(non_fmt_panics)]
        let filled = match *self {
            MaybeInst::Split1(goto1) => panic!("1"),
            MaybeInst::Split2(goto2) => panic!("2"),
            _ => unimplemented!(),
        };
        unimplemented!()
    }
}

fn underscores_and_numbers() {
    let _1 = 1;
    //~^ just_underscores_and_digits
    let ____1 = 1;
    //~^ just_underscores_and_digits
    let __1___2 = 12;
    //~^ just_underscores_and_digits
    let _1_ok = 1;
}

fn issue2927() {
    let args = 1;
    format!("{:?}", 2);
}

fn issue3078() {
    #[allow(clippy::single_match)]
    match "a" {
        stringify!(a) => {},
        _ => {},
    }
}

struct Bar;

impl Bar {
    fn bar() {
        let _1 = 1;
        //~^ just_underscores_and_digits
        let ____1 = 1;
        //~^ just_underscores_and_digits
        let __1___2 = 12;
        //~^ just_underscores_and_digits
        let _1_ok = 1;
    }
}

fn outer_allow() {
    #[allow(clippy::just_underscores_and_digits)]
    let _0 = 0;
}

#[allow(clippy::just_underscores_and_digits)]
fn function_allow() {
    let _0 = 0;
}

mod inner_allow {
    #![allow(clippy::just_underscores_and_digits)]

    fn example() {
        let _0 = 0;
    }
}

fn main() {}
