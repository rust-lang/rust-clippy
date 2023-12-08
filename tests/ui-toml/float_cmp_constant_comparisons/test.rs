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
    {
        let _ = 1.0f32 == 2.0f32;
        let _ = -1.0f32 == -2.0f32;
        let _ = 1.0f64 == 2.0f64;
    }
    {
        fn f(x: f32) {
            let _ = x + 1.0 == x;
            let _ = x == x + 1.0;
        }
    }
}
