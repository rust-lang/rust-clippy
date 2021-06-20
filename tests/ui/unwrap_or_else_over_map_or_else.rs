#![warn(clippy::unwrap_or_else_over_map_or_else)]

fn main() {
    let c = func_result(3).unwrap_or_else(|e| {e});
    func_result(2).map_or_else(|e| println!("{:?}", e), |n| println!("{}", n))
}

fn func_result(in_num: u8) -> Result<&'static str, &'static str> {
    if in_num % 2 != 0 {
        return Err("Can't do this because input is odd...");
    }
    Ok("An even number :)")
}
