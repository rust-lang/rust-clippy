#![warn(clippy::pointer_format)]

fn main() {
    let m = &(main as fn());
    let g = &0;
    let o = &format!("{m:p}");
    //~^ pointer_format
    println!("{g:p}");
    //~^ pointer_format
    panic!("{o:p}");
    //~^ pointer_format
}
