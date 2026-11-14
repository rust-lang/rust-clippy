//@aux-build: proc_macros.rs
#![warn(clippy::integer_division_remainder_used)]
#![expect(clippy::op_ref)]

struct CustomOps(pub i32);
impl std::ops::Div for CustomOps {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
        //~^ integer_division_remainder_used
    }
}
impl std::ops::Rem for CustomOps {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self(self.0 % rhs.0)
        //~^ integer_division_remainder_used
    }
}

fn issue17603() {
    let x: u16 = 7;

    let _ = x.checked_div(3);
    //~^ integer_division_remainder_used
    let _ = x.checked_div_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.checked_rem(3);
    //~^ integer_division_remainder_used
    let _ = x.checked_rem_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.div_ceil(3);
    //~^ integer_division_remainder_used
    let _ = x.div_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.overflowing_div(3);
    //~^ integer_division_remainder_used
    let _ = x.overflowing_div_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.overflowing_rem(3);
    //~^ integer_division_remainder_used
    let _ = x.overflowing_rem_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.rem_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.strict_div(3);
    //~^ integer_division_remainder_used
    let _ = x.strict_div_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.saturating_div(3);
    //~^ integer_division_remainder_used
    let _ = x.strict_rem(3);
    //~^ integer_division_remainder_used
    let _ = x.strict_rem_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.wrapping_div(3);
    //~^ integer_division_remainder_used
    let _ = x.wrapping_div_euclid(3);
    //~^ integer_division_remainder_used
    let _ = x.wrapping_rem(3);
    //~^ integer_division_remainder_used
    let _ = x.wrapping_rem_euclid(3);
    //~^ integer_division_remainder_used

    // methods that also exist on floats should not trigger there
    let f: f64 = 7.0;
    let _ = f.div_euclid(3.0);
    let _ = f.rem_euclid(3.0);
}

fn main() {
    // should trigger
    let a = 10;
    let b = 5;
    let c = a / b;
    //~^ integer_division_remainder_used
    let d = a % b;
    //~^ integer_division_remainder_used
    let e = &a / b;
    //~^ integer_division_remainder_used
    let f = a % &b;
    //~^ integer_division_remainder_used
    let g = &a / &b;
    //~^ integer_division_remainder_used
    let h = &10 % b;
    //~^ integer_division_remainder_used
    let i = a / &4;
    //~^ integer_division_remainder_used

    // should trigger on DivAssign and RemAssign
    let mut j = 10;
    j /= 2;
    //~^ integer_division_remainder_used
    j %= 3;
    //~^ integer_division_remainder_used

    // should not trigger on custom Div and Rem
    let w = CustomOps(3);
    let x = CustomOps(4);
    let y = w / x;

    let w = CustomOps(3);
    let x = CustomOps(4);
    let z = w % x;

    macro_rules! mac {
        ($a:expr, $b:expr) => {
            $a % $b
            //~^ integer_division_remainder_used
        };
    }
    // should not trigger if from expansion in external macro
    let issue17048 = mac!(a, b);
    let issue17048 = proc_macros::external! {{
        let a = 10;
        let b = 5;
        a % b
    }};
}
