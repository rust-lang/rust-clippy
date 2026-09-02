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
    let _ = x.div_ceil(3);
    //~^ integer_division_remainder_used

    let _ = x.wrapping_div(3);
    //~^ integer_division_remainder_used

    let _ = x.checked_rem(3);
    //~^ integer_division_remainder_used

    let f: f64 = 7.0;
    let _ = f.div_euclid(3.0);
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
}
