//@no-rustfix
#![allow(unused)]

fn f(x: i32) -> ! {
    panic!();
}

fn diverge() -> ! {
    panic!();
}

fn main() {
    // Test cases that should trigger the NEVER_ITER lint
    [0, 1].into_iter().for_each(|x| f(x)); //~ NEVER_ITER
    vec![1, 2, 3].iter().for_each(|x| diverge());
    
    // Test try_for_each with diverging closure
    [1, 2, 3].into_iter().try_for_each(|x| -> Result<(), ()> { diverge(); }); //~ NEVER_ITER
    
    // Test fold with diverging closure
    [1, 2, 3].into_iter().fold(0, |acc, x| { diverge(); }); //~ NEVER_ITER
    
    // Test reduce with diverging closure
    [1, 2, 3].into_iter().reduce(|acc, x| { diverge(); }); //~ NEVER_ITER
    
    // Test all/any with diverging closure 
    [1, 2, 3].into_iter().all(|x| { diverge(); }); //~ NEVER_ITER
    [1, 2, 3].into_iter().any(|x| { diverge(); }); //~ NEVER_ITER
    
    // Test find with diverging closure
    [1, 2, 3].into_iter().find(|x| { diverge(); }); //~ NEVER_ITER
    
    // Test find_map with diverging closure
    [1, 2, 3].into_iter().find_map(|x| { diverge(); }); //~ NEVER_ITER
    
    // Test position with diverging closure
    [1, 2, 3].into_iter().position(|x| { diverge(); }); //~ NEVER_ITER
    
    // Test rposition with diverging closure
    [1, 2, 3].into_iter().rposition(|x| { diverge(); }); //~ NEVER_ITER

    // Test cases that should NOT trigger (non-diverging)
    [1, 2, 3].into_iter().for_each(|x| println!("{}", x));
    [1, 2, 3].into_iter().fold(0, |acc, x| acc + x);
}