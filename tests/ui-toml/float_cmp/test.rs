//@no-rustfix
//@revisions: change_detect const_cmp named_const
//@[change_detect] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/float_cmp/change_detect
//@[const_cmp] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/float_cmp/const_cmp
//@[named_const] rustc-env:CLIPPY_CONF_DIR=tests/ui-toml/float_cmp/named_const

// FIXME(f16_f128): const casting is not yet supported for these types. Add when available.

#![deny(clippy::float_cmp)]
#![allow(clippy::op_ref, clippy::eq_op, clippy::legacy_numeric_constants)]

const F32_ARRAY: [f32; 2] = [5.5, 5.5];

fn main() {
    {
        fn _f(x: f32, y: f32) {
            let _ = x == y; //~ float_cmp
            let _ = x != y; //~ float_cmp
            let _ = x == 5.5; //~ float_cmp
            let _ = 5.5 == x; //~ float_cmp

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
            let _ = x == y; //~ float_cmp
            let _ = x != y; //~ float_cmp
            let _ = x == 5.5; //~ float_cmp
            let _ = 5.5 == x; //~ float_cmp

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
            let _ = x == y; //~ float_cmp
            let _ = x == [5.5; 4]; //~ float_cmp
            let _ = [5.5; 4] == x; //~ float_cmp
            let _ = [0.0, 0.0, 0.0, 5.5] == x; //~ float_cmp
            let _ = x == [0.0, 0.0, 0.0, 5.5]; //~ float_cmp

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
            let _ = x == y; //~ float_cmp
            let _ = x == [5.5; 4]; //~ float_cmp
            let _ = [5.5; 4] == x; //~ float_cmp
            let _ = [0.0, 0.0, 0.0, 5.5] == x; //~ float_cmp
            let _ = x == [0.0, 0.0, 0.0, 5.5]; //~ float_cmp

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
            let _ = x == y; //~ float_cmp
            let _ = x == &&&0.0;
        }
    }
    {
        fn _f(x: &&&[f32; 2], y: &&&[f32; 2]) {
            let _ = x == y; //~ float_cmp
            let _ = x == &&&[0.0, -0.0];
        }
    }

    // Comparisons to named constant
    {
        fn _f(x: f32, y: f64) {
            let _ = x == f32::EPSILON; //~[named_const] float_cmp
            let _ = f32::EPSILON == x; //~[named_const] float_cmp
            let _ = &&x == &&core::f32::EPSILON; //~[named_const] float_cmp
            let _ = &&core::f32::EPSILON == &&x; //~[named_const] float_cmp
            let _ = y == f32::EPSILON as f64; //~[named_const] float_cmp
            let _ = f32::EPSILON as f64 == y; //~[named_const] float_cmp

            let _ = f32::EPSILON * x == x * x; //~ float_cmp
            let _ = x * x == f32::EPSILON * x; //~ float_cmp
        }
    }
    {
        fn _f(x: [f32; 2]) {
            let _ = x == F32_ARRAY; //~[named_const] float_cmp
            let _ = F32_ARRAY == x; //~[named_const] float_cmp
            let _ = &&x == &&F32_ARRAY; //~[named_const] float_cmp
            let _ = &&F32_ARRAY == &&x; //~[named_const] float_cmp
        }
    }

    // Constant comparisons
    {
        const fn f(x: f32) -> f32 {
            todo!()
        }
        let _ = f(1.0) == f(5.0); //~[const_cmp] float_cmp
        let _ = 1.0 == f(5.0); //~[const_cmp] float_cmp
        let _ = f(1.0) + 1.0 != 5.0; //~[const_cmp] float_cmp
    }
    {
        fn f(x: f32) -> f32 {
            todo!()
        }
        let _ = f(1.0) == f(5.0); //~ float_cmp
        let _ = 1.0 == f(5.0); //~ float_cmp
        let _ = f(1.0) + 1.0 != 5.0; //~ float_cmp
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

            let _ = x == C[1]; //~ float_cmp
            let _ = C[1] == x; //~ float_cmp
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
            let _ = x == x + 1.0; //~[change_detect] float_cmp
            let _ = x + 1.0 == x; //~[change_detect] float_cmp
            let _ = -x == -x + 1.0; //~[change_detect] float_cmp
            let _ = -x + 1.0 == -x; //~[change_detect] float_cmp
            let _ = x == f1(x); //~[change_detect] float_cmp
            let _ = f1(x) == x; //~[change_detect] float_cmp
            let _ = x == f2(x, y); //~[change_detect] float_cmp
            let _ = f2(x, y) == x; //~[change_detect] float_cmp
            let _ = f1(f1(x)) == f1(x); //~[change_detect] float_cmp
            let _ = f1(x) == f1(f1(x)); //~[change_detect] float_cmp

            let z = (x, y);
            let _ = z.0 == z.0 + 1.0; //~[change_detect] float_cmp
            let _ = z.0 + 1.0 == z.0; //~[change_detect] float_cmp
        }

        fn _f2(x: &f32) {
            let _ = *x + 1.0 == *x; //~[change_detect] float_cmp
            let _ = *x == *x + 1.0; //~[change_detect] float_cmp
            let _ = *x == f1(*x); //~[change_detect] float_cmp
            let _ = f1(*x) == *x; //~[change_detect] float_cmp
        }
    }
    {
        fn _f(mut x: impl Iterator<Item = f32>) {
            let _ = x.next().unwrap() == x.next().unwrap() + 1.0; //~ float_cmp
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
            let _ = x.f() + 1.0 == x.f(); //~ float_cmp
            let _ = x.f() == x.f() + 1.0; //~ float_cmp
        }
    }
    {
        let f = |x: f32| -> f32 { x };
        let _ = f(1.0) == f(1.0) + 1.0;

        let mut x = 1.0;
        let mut f = |y: f32| -> f32 { core::mem::replace(&mut x, y) };
        let _ = f(1.0) == f(1.0) + 1.0; //~ float_cmp
    }

    // Self comparisons
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
