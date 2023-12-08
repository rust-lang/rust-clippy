//@no-rustfix

// FIXME(f16_f128): const casting is not yet supported for these types. Add when available.

#![warn(clippy::float_cmp)]
#![allow(clippy::op_ref)]

fn main() {
    {
        fn _f(x: f32, y: f32) {
            let _ = x == y;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x != y;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x == 5.5;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = 5.5 == x;
            //~^ ERROR: strict comparison of `f32` or `f64`

            let _ = x < 5.5;
            let _ = x <= 5.5;
            let _ = x > 5.5;
            let _ = x >= 5.5;
            let _ = 5.5 < x;
            let _ = 5.5 <= x;
            let _ = 5.5 > x;
            let _ = 5.5 >= x;

            let _ = 0.0 == x;
            let _ = -0.0 == x;
            let _ = 1.0 / 0.0 == x;
            let _ = -1.0 / 0.0 == x;
            let _ = x == 0.0;
            let _ = x == -0.0;
            let _ = x == 1.0 / 0.0;
            let _ = x == -1.0 / 0.0;
        }
    }
    {
        fn _f(x: f64, y: f64) {
            let _ = x == y;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x != y;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x == 5.5;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = 5.5 == x;
            //~^ ERROR: strict comparison of `f32` or `f64`

            let _ = x < 5.5;
            let _ = x <= 5.5;
            let _ = x > 5.5;
            let _ = x >= 5.5;
            let _ = 5.5 < x;
            let _ = 5.5 <= x;
            let _ = 5.5 > x;
            let _ = 5.5 >= x;

            let _ = 0.0 == x;
            let _ = -0.0 == x;
            let _ = 1.0 / 0.0 == x;
            let _ = -1.0 / 0.0 == x;
            let _ = x == 0.0;
            let _ = x == -0.0;
            let _ = x == 1.0 / 0.0;
            let _ = x == -1.0 / 0.0;
        }
    }
    {
        fn _f(x: [f32; 4], y: [f32; 4]) {
            let _ = x == y;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x == [5.5; 4];
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = [5.5; 4] == x;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = [0.0, 0.0, 0.0, 5.5] == x;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x == [0.0, 0.0, 0.0, 5.5];
            //~^ ERROR: strict comparison of `f32` or `f64`

            let _ = [0.0; 4] == x;
            let _ = [-0.0; 4] == x;
            let _ = [1.0 / 0.0; 4] == x;
            let _ = [-1.0 / 0.0; 4] == x;
            let _ = [0.0, -0.0, 1.0 / 0.0, -1.0 / 0.0] == x;
            let _ = x == [0.0; 4];
            let _ = x == [-0.0; 4];
            let _ = x == [1.0 / 0.0; 4];
            let _ = x == [-1.0 / 0.0; 4];
            let _ = x == [0.0, -0.0, 1.0 / 0.0, -1.0 / 0.0];
        }
    }
    {
        fn _f(x: [f64; 4], y: [f64; 4]) {
            let _ = x == y;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x == [5.5; 4];
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = [5.5; 4] == x;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = [0.0, 0.0, 0.0, 5.5] == x;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x == [0.0, 0.0, 0.0, 5.5];
            //~^ ERROR: strict comparison of `f32` or `f64`

            let _ = [0.0; 4] == x;
            let _ = [-0.0; 4] == x;
            let _ = [1.0 / 0.0; 4] == x;
            let _ = [-1.0 / 0.0; 4] == x;
            let _ = [0.0, -0.0, 1.0 / 0.0, -1.0 / 0.0] == x;
            let _ = x == [0.0; 4];
            let _ = x == [-0.0; 4];
            let _ = x == [1.0 / 0.0; 4];
            let _ = x == [-1.0 / 0.0; 4];
            let _ = x == [0.0, -0.0, 1.0 / 0.0, -1.0 / 0.0];
        }
    }

    // Reference comparisons
    {
        fn _f(x: &&&f32, y: &&&f32) {
            let _ = x == y;
            //~^ ERROR: strict comparison of `f32` or `f64`

            let _ = x == &&&0.0;
        }
    }
    {
        fn _f(x: &&&[f32; 2], y: &&&[f32; 2]) {
            let _ = x == y;
            //~^ ERROR: strict comparison of `f32` or `f64`

            let _ = x == &&&[0.0, -0.0];
        }
    }

    // Comparisons to named constant
    {
        const C: f32 = 5.5;
        fn _f(x: f32, y: f64) {
            let _ = x == C;
            let _ = C == x;
            let _ = &&x == &&C;
            let _ = &&C == &&x;
            let _ = y == C as f64;
            let _ = C as f64 == y;

            let _ = C * x == x * x;
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x * x == C * x;
            //~^ ERROR: strict comparison of `f32` or `f64`
        }
    }
    {
        const C: [f32; 2] = [5.5, 5.5];
        fn _f(x: [f32; 2]) {
            let _ = x == C;
            let _ = C == x;
            let _ = &&x == &&C;
            let _ = &&C == &&x;
        }
    }

    // Constant comparisons
    {
        const fn f(x: f32) -> f32 {
            todo!()
        }
        let _ = f(1.0) == f(5.0);
        let _ = 1.0 == f(5.0);
        let _ = f(1.0) + 1.0 != 5.0;
    }
    {
        fn f(x: f32) -> f32 {
            todo!()
        }
        let _ = f(1.0) == f(5.0);
        //~^ ERROR: strict comparison of `f32` or `f64`
        let _ = 1.0 == f(5.0);
        //~^ ERROR: strict comparison of `f32` or `f64`
        let _ = f(1.0) + 1.0 != 5.0;
        //~^ ERROR: strict comparison of `f32` or `f64`
    }

    // Pointer equality
    {
        fn _f(x: *const f32, y: *const f32) {
            let _ = x == y;
        }
    }
    {
        fn _f(x: *const [f32; 2], y: *const [f32; 2]) {
            let _ = x == y;
        }
    }

    // `signum`
    {
        fn _f(x: f32, y: f32) {
            let _ = x.signum() == y.signum();
            let _ = x.signum() == -y.signum();
            let _ = -x.signum() == y.signum();
            let _ = -x.signum() == -y.signum();
        }
    }
    {
        fn _f(x: f64, y: f64) {
            let _ = x.signum() == y.signum();
            let _ = x.signum() == -y.signum();
            let _ = -x.signum() == y.signum();
            let _ = -x.signum() == -y.signum();
        }
    }

    // Index constant array
    {
        const C: [f32; 3] = [0.0, 5.5, -0.0];
        fn _f(x: f32) {
            let _ = x == C[0];
            let _ = x == C[2];
            let _ = C[0] == x;
            let _ = C[2] == x;

            let _ = x == C[1];
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = C[1] == x;
            //~^ ERROR: strict comparison of `f32` or `f64`
        }
    }

    // `eq` functions
    {
        fn eq(x: f32, y: f32) -> bool {
            x == y
        }

        fn ne(x: f32, y: f32) -> bool {
            x != y
        }

        fn is_eq(x: f32, y: f32) -> bool {
            x == y
        }

        struct _X(f32);
        impl PartialEq for _X {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        fn eq_fl(x: f32, y: f32) -> bool {
            if x.is_nan() { y.is_nan() } else { x == y }
        }

        fn fl_eq(x: f32, y: f32) -> bool {
            if x.is_nan() { y.is_nan() } else { x == y }
        }
    }

    // Custom types
    {
        struct S;
        impl PartialEq<f32> for S {
            fn eq(&self, _: &f32) -> bool {
                false
            }
        }

        fn _f(x: S, y: f32) {
            let _ = x == y;
        }
    }

    // modified operands
    {
        fn f1(x: f32) -> f32 {
            x + 1.0
        }

        fn f2(x: f32, y: f32) -> f32 {
            x + y
        }

        fn _f(x: f32, y: f32) {
            let _ = x == x + 1.0;
            let _ = x + 1.0 == x;
            let _ = -x == -x + 1.0;
            let _ = -x + 1.0 == -x;
            let _ = x == f1(x);
            let _ = f1(x) == x;
            let _ = x == f2(x, y);
            let _ = f2(x, y) == x;
            let _ = f1(f1(x)) == f1(x);
            let _ = f1(x) == f1(f1(x));

            let z = (x, y);
            let _ = z.0 == z.0 + 1.0;
            let _ = z.0 + 1.0 == z.0;
        }

        fn _f2(x: &f32) {
            let _ = *x + 1.0 == *x;
            let _ = *x == *x + 1.0;
            let _ = *x == f1(*x);
            let _ = f1(*x) == *x;
        }
    }
    {
        fn _f(mut x: impl Iterator<Item = f32>) {
            let _ = x.next().unwrap() == x.next().unwrap() + 1.0;
            //~^ ERROR: strict comparison of `f32` or `f64`
        }
    }
    {
        use core::cell::RefCell;

        struct S(RefCell<f32>);
        impl S {
            fn f(&self) -> f32 {
                let x = *self.0.borrow();
                *self.0.borrow_mut() *= 2.0;
                x
            }
        }

        fn _f(x: S) {
            let _ = x.f() + 1.0 == x.f();
            //~^ ERROR: strict comparison of `f32` or `f64`
            let _ = x.f() == x.f() + 1.0;
            //~^ ERROR: strict comparison of `f32` or `f64`
        }
    }
    {
        let f = |x: f32| -> f32 { x };
        let _ = f(1.0) == f(1.0) + 1.0;

        let mut x = 1.0;
        let mut f = |y: f32| -> f32 { core::mem::replace(&mut x, y) };
        let _ = f(1.0) == f(1.0) + 1.0; //~ float_cmp
    }
}
