#![warn(clippy::floats_used)]

struct Structure {
    a: f32,
    //~^ floats_used
    u: u8,
}

struct GenericStruct<T>(T);
const FLOAT_STRUCT: GenericStruct<f32> = GenericStruct(0.1);
//~^ floats_used

static ST_FLOAT: f64 = 20.0;
//~^ floats_used

static ST_INTEGER: u8 = 8;

fn main() {
    let float = 0.2;
    //~^ floats_used
    let array_float: [f64; 4] = [0.0; 4];
    //~^ floats_used
    let vec_float: Vec<f32> = Vec::new();
    //~^ floats_used

    let integer = 8;

    let expr = 0.3 + float;
    //~^ floats_used
}

fn float_func(a: f32) -> f32 {
    //~^ floats_used
    let mut a_int = a as u32;
    //~^ floats_used

    a_int += 42;

    a_int as f32
    //~^ floats_used
}
