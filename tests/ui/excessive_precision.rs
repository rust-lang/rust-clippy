#![feature(plugin)]



const TOO_PRECISE: f32 = 6.28318530717958647692528676655900;

const PRECISE_ENOUGH: f32 = 6.28;

fn main() {


    let b = TOO_PRECISE + PRECISE_ENOUGH;
    println!("{}", b);

}
