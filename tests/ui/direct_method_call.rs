#![allow(unused)]
#![warn(clippy::direct_method_call)]
#![allow(clippy::let_unit_value)]

trait Xa {
    fn hi(self) -> u8;
    fn hoi(self, x: u32) -> u8;
}

impl Xa for f32 {
    fn hi(self) -> u8 {
        3
    }
    fn hoi(self, x: u32) -> u8 {
        2
    }
}

fn main() {
    let _ = Xa::hi(4.5f32);
    let _ = (4.5f32).hi();
    let _ = Xa::hoi(4.5f32, 4);
    let _ = f32::floor(4.5f32);
}
