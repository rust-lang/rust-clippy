#![warn(clippy::absurd_extreme_comparisons)]
#![allow(
    unused,
    clippy::eq_op,
    clippy::no_effect,
    clippy::unnecessary_operation,
    clippy::needless_pass_by_value
)]

use std::time::Duration;

#[rustfmt::skip]
fn main() {
    const Z: u32 = 0;
    let u: u32 = 42;
    u <= 0;
    //~^ absurd_extreme_comparisons

    u <= Z;
    //~^ absurd_extreme_comparisons

    u < Z;
    //~^ absurd_extreme_comparisons

    Z >= u;
    //~^ absurd_extreme_comparisons

    Z > u;
    //~^ absurd_extreme_comparisons

    u > u32::MAX;
    //~^ absurd_extreme_comparisons

    u >= u32::MAX;
    //~^ absurd_extreme_comparisons

    u32::MAX < u;
    //~^ absurd_extreme_comparisons

    u32::MAX <= u;
    //~^ absurd_extreme_comparisons

    1-1 > u;
    //~^ absurd_extreme_comparisons

    u >= !0;
    //~^ absurd_extreme_comparisons

    u <= 12 - 2*6;
    //~^ absurd_extreme_comparisons

    let i: i8 = 0;
    i < -127 - 1;
    //~^ absurd_extreme_comparisons

    i8::MAX >= i;
    //~^ absurd_extreme_comparisons

    3-7 < i32::MIN;
    //~^ absurd_extreme_comparisons

    let b = false;
    b >= true;
    //~^ absurd_extreme_comparisons

    false > b;
    //~^ absurd_extreme_comparisons

    u > 0; // ok
    // this is handled by clippy::unit_cmp
    () < {};
    //~^ unit_cmp

    Duration::from_secs(0) > Duration::new(5, 0);
    //~^ absurd_extreme_comparisons
    Duration::new(5, 0) < Duration::from_secs(0);
    //~^ absurd_extreme_comparisons
    Duration::from_secs(0) < Duration::new(5, 0); // ok

    let d = Duration::new(5, 0);

    d < Duration::from_secs(0);
    //~^ absurd_extreme_comparisons
    Duration::from_secs(0) > d;
    //~^ absurd_extreme_comparisons
    
    d < Duration::from_millis(0);
    //~^ absurd_extreme_comparisons
    Duration::from_micros(0) > d; 
    //~^ absurd_extreme_comparisons
    
    d <= Duration::from_nanos(0);
    //~^ absurd_extreme_comparisons
    Duration::from_nanos(0) >= d;
    //~^ absurd_extreme_comparisons

    d < Duration::default();
    //~^ absurd_extreme_comparisons
    Duration::default() > d;
    //~^ absurd_extreme_comparisons

    d < Duration::from_secs_f32(0.0);
    //~^ absurd_extreme_comparisons
    Duration::from_secs_f64(0.0) > d;
    //~^ absurd_extreme_comparisons

    let zero_secs: u64 = 0;
    let zero_nanos: u32 = 0;
    d < Duration::new(zero_secs, zero_nanos);
    //~^ absurd_extreme_comparisons
    Duration::new(zero_secs, zero_nanos) > d;
    //~^ absurd_extreme_comparisons

    d > Duration::from_secs(0); // OK
    Duration::from_secs(0) < d; // OK
    d == Duration::from_secs(0); // OK

    let d = Duration::new(5, 0);
    d < one_second_plus(0); // OK
    one_second_plus(0) > d; // OK

    let n = 0;
    d < one_second_plus(n); // OK

    d < Duration::from_secs(0);
    //~^ absurd_extreme_comparisons
    Duration::from_secs(0) > d;
    //~^ absurd_extreme_comparisons


    let d_zero = Duration::from_secs(0);
    let d_one = one_second_plus(0);
    d_zero < d_one; // OK
    d_one > d_zero; // OK
    d_zero < d_zero; // OK
}

use std::cmp::{Ordering, PartialEq, PartialOrd};

#[derive(PartialEq, Eq, PartialOrd)]
pub struct U(u64);

impl PartialEq<u32> for U {
    fn eq(&self, other: &u32) -> bool {
        self.eq(&U(u64::from(*other)))
    }
}
impl PartialOrd<u32> for U {
    fn partial_cmp(&self, other: &u32) -> Option<Ordering> {
        self.partial_cmp(&U(u64::from(*other)))
    }
}

pub fn foo(val: U) -> bool {
    val > u32::MAX
}

pub fn bar(len: u64) -> bool {
    // This is OK as we are casting from target sized to fixed size
    len >= usize::MAX as u64
}

fn one_second_plus(n: u64) -> Duration {
    Duration::from_secs(n + 1)
}
