#![warn(clippy::identity_assign_op)]
#![allow(unused)]

const ZERO_I64: i64 = 0;
const ONE_I64: i64 = 1;
const ZERO_U64: u64 = 0;
const ONE_U64: u64 = 1;
const ZERO_F32: f32 = 0.0;
const NEGATIVE_ZERO_F32: f32 = -0.0;
const ONE_F32: f32 = 1.0;
const ZERO_F64: f64 = 0.0;
const NEGATIVE_ZERO_F64: f64 = -0.0;
const ONE_F64: f64 = 1.0;

#[rustfmt::skip]
fn test_identity_values() {
    // Literals
    let mut signed = 1_i64;

    signed += 0;
    //~^ identity_assign_op

    signed -= 0;
    //~^ identity_assign_op

    signed |= 0;
    //~^ identity_assign_op

    signed ^= 0;
    //~^ identity_assign_op

    signed <<= 0;
    //~^ identity_assign_op

    signed >>= 0;
    //~^ identity_assign_op

    signed *= 1;
    //~^ identity_assign_op

    signed /= 1;
    //~^ identity_assign_op

    let mut unsigned = 1_u64;

    unsigned += 0;
    //~^ identity_assign_op

    unsigned *= 1;
    //~^ identity_assign_op

    let mut float32 = 1.0_f32;

    float32 += 0.0; // no error

    float32 *= 1.0;
    //~^ identity_assign_op

    float32 += -0.0;
    //~^ identity_assign_op

    float32 -= 0.0;
    //~^ identity_assign_op

    let mut float64 = 1.0_f64;

    float64 += 0.0; // no error

    float64 *= 1.0;
    //~^ identity_assign_op

    float64 += -0.0;
    //~^ identity_assign_op

    float64 -= 0.0;
    //~^ identity_assign_op

    let mut subtraction = 1.0_f32;
    subtraction -= -0.0; // no error

    // Constants
    let mut signed = 1_i64;

    signed += ZERO_I64;
    //~^ identity_assign_op

    signed *= ONE_I64;
    //~^ identity_assign_op

    let mut unsigned = 1_u64;

    unsigned += ZERO_U64;
    //~^ identity_assign_op

    unsigned *= ONE_U64;
    //~^ identity_assign_op

    let mut float32 = 1.0_f32;

    float32 += ZERO_F32; // no error

    float32 *= ONE_F32;
    //~^ identity_assign_op

    float32 += NEGATIVE_ZERO_F32;
    //~^ identity_assign_op

    let mut float64 = 1.0_f64;

    float64 += ZERO_F64; // no error

    float64 *= ONE_F64;
    //~^ identity_assign_op

    float64 += NEGATIVE_ZERO_F64;
    //~^ identity_assign_op
}

fn test_non_identity_values() {
    let mut value = 1_i64;

    value += 1;
    value *= 2;
    value -= 1;
    value <<= 1;
}

fn test_series() {
    let mut series = 1_i64;

    series += 0; // no error: part of a series
    series += 1;
    series += 2;

    series <<= 0; // no error: part of a two-statement series
    series <<= 1;

    series *= 2;
    series *= 1; // no error: at the end of a series
}

fn test_user_defined_operators() {
    let mut custom = Custom(1);
    custom += 0; // no error: user-defined operator
    custom *= 1; // no error: user-defined operator
}

fn test_macros() {
    let mut custom = Custom(1);
    custom -= 0; // no error: macro-generated user-defined operator
}

struct Custom(i64);

impl std::ops::AddAssign<i64> for Custom {
    fn add_assign(&mut self, rhs: i64) {
        self.0 += rhs + 1;
    }
}

impl std::ops::MulAssign<i64> for Custom {
    fn mul_assign(&mut self, rhs: i64) {
        self.0 *= rhs + 1;
    }
}

macro_rules! impl_sub_assign {
    () => {
        impl std::ops::SubAssign<i64> for Custom {
            fn sub_assign(&mut self, rhs: i64) {
                self.0 -= rhs + 1;
            }
        }
    };
}

impl_sub_assign!();
