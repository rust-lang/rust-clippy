#![feature(plugin)]
#![plugin(clippy)]

#![allow(dead_code)]
#![allow(unreadable_literal)]
#![allow(inconsistent_digit_grouping)]

const TOO_PRECISE1: f32 = 6.283185307;
const TOO_PRECISE2: f32 = 6.283_185_307;
const TOO_PRECISE3: f32 = 6.283185568f32;
const TOO_PRECISE4: f32 = 6.283_185_568f32;

const PRECISE_ENOUGH1: f32 = 6.280989;
const PRECISE_ENOUGH2: f32 = 6.280989f32;
const PRECISE_ENOUGH3: f32 = 6.280_989;
const PRECISE_ENOUGH4: f32 = 6.280_989f32;

const OK1: f64 = 27.123456789012345;
const OK2: f64 = 27.123_456_789_012_345;
const OK3: f64 = 27.123456789012345f64;
const OK4: f64 = 27.123_456_789_012_345f64;

const NOT_OK1: f64 = 0.1234567890123456;
const NOT_OK2: f64 = 0.1_234_567_890_123_456;
const NOT_OK3: f64 = 0.1234567890123456f64;
const NOT_OK4: f64 = 0.1_234_567_890_123_456f64;


fn main() {}
