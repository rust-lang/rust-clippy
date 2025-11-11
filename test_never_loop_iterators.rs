// Test for never_loop iterator functionality
#![allow(unused)]

fn f(x: i32) -> ! {
    panic!();
}

fn main() {
    // This should trigger the never_loop lint because f(x) diverges
    [0, 1].into_iter().for_each(|x| f(x)); // Only calls `f` once
    
    // This should also trigger the never_loop lint 
    vec![1, 2, 3].iter().for_each(|x| panic!());
    
    // This should trigger the lint too
    [1, 2, 3].into_iter().try_for_each(|x| -> Result<(), ()> { panic!(); });
    
    // This should trigger too - fold with diverging closure
    [1, 2, 3].into_iter().fold(0, |acc, x| { panic!(); });
}