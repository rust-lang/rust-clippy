#![allow(unused)]
#![warn(clippy::direct_method_call)]

trait MyTrait {
    fn function(self) -> u8;
    fn function_with_args(self, x: u32) -> u8;
}

impl MyTrait for f32 {
    fn function(self) -> u8 {
        3
    }
    fn function_with_args(self, x: u32) -> u8 {
        2
    }
}

fn main() {
    // Should warn
    let _ = MyTrait::function(4.5f32);
    let _ = MyTrait::function_with_args(4.5f32, 4);
    let _ = f32::floor(4.5f32);
    
    // Should not warn
    let _ = (4.5f32).function();
    let _ = (4.5f32).function_with_args(4);
    let _ = (4.5f32).floor();
}
