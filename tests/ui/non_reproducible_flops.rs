#![warn(clippy::non_reproducible_flops)]

fn sin(f: f32) -> f32 {
    unimplemented!()
}

fn main() {
    1.0_f32.sin();
    f32::sin(1.0);
    sin(1.0);

    1.0_f32.clamp(2.0, 3.0);
    f32::clamp(1.0, 2.0_f32.hypot(3.0), f32::default());

    f32::from(1u8);

    1.0_f64.abs().mul_add(2.0, 4.0).floor().ln();
}
