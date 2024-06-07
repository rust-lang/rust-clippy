//@no-rustfix

#![deny(clippy::float_cmp)]
#![allow(clippy::op_ref, clippy::eq_op)]

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
    {
        fn _f(x: f32) {
            let _ = x == x;
            let _ = x != x;
            let _ = x == -x;
            let _ = -x == x;
            let _ = x as f64 == x as f64;
            let _ = &&x == &&x;
        }
    }
}
