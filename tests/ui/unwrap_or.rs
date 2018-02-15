#![warn(clippy)]

fn main() {
    let some_string = Some(String::from("this string from the test"));
    let su = some_string.clone().and(some_string).unwrap_or("Fail".to_string());
}

