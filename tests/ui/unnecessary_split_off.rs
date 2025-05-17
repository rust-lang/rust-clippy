//@no-rustfix
#![warn(clippy::unnecessary_split_off)]
#![allow(unused)]

struct A;
impl A {
    fn split_off(&mut self, _: usize) {}
}

const ZERO: usize = 0;

fn main() {
    let mut vec1 = vec![1, 2, 3];

    let vec2: Vec<_> = vec1.split_off(0);
    //~^ unnecessary_split_off

    let vec3: Vec<_> = vec1.split_off(1);

    let vec4: Vec<_> = vec1.split_off(ZERO);
    //~^ unnecessary_split_off

    let vec5: Vec<_> = vec1.split_off(const { 0 });
    //~^ unnecessary_split_off

    let mut a = A;
    a.split_off(0);
}
