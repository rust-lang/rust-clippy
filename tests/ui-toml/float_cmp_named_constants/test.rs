//@no-rustfix

#![deny(clippy::float_cmp)]

fn main() {
    {
        const C: f64 = 1.0;
        fn f(x: f64) {
            let _ = x == C;
        }
    }
    {
        const fn f(x: f64) -> f64 {
            todo!()
        }
        let _ = f(1.0) == f(2.0);
    }
}
