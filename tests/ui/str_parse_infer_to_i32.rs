#![warn(clippy::str_parse_infer_to_i32)]

/*
Just here for easy copy/pasting to add more tests
"111111111111111111"
*/
fn main() {
    let closure = || "111111111111111111".parse();

    fn f() -> i32 {
        "111111111111111111".parse().unwrap_or(0)
    }

    let a_var_too_big_for_i32 = "111111111111111111";

    // bad ones
    let _x = a_var_too_big_for_i32.parse::<_>().unwrap_or(0);
    //~^ str_parse_infer_to_i32
    let _x = a_var_too_big_for_i32.parse().unwrap_or(0);
    //~^ str_parse_infer_to_i32
    let _x = "111111111111111111".parse().unwrap_or(0);
    //~^ str_parse_infer_to_i32
    let _x = "111111111111111111".parse::<_>().unwrap_or(0);
    //~^ str_parse_infer_to_i32
    // Check deref from string works
    let _x = String::from("111111111111111111").parse().unwrap_or(0);
    //~^ str_parse_infer_to_i32
    let _x = closure().unwrap_or(0);
    //~^ str_parse_infer_to_i32

    // good ones
    let _x = "111111111111111111".parse().unwrap_or(0i32);
    let _x: i32 = "111111111111111111".parse().unwrap_or(0);
    let _x = "111111111111111111".parse::<i32>().unwrap_or(0);
    let _x = "111111111111111111".parse::<i64>().unwrap_or(0);
    let _x = f();
    let _x: i32 = String::from("111111111111111111").parse().unwrap_or(0);
    let _x: i32 = closure().unwrap_or(0);
}
