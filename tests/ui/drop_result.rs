#[warn(clippy::drop_result)]

fn make_result<T>(t: T) -> Result<T, ()> {
    Ok(t)
}

fn main() {
    drop(Ok::<String, ()>("a".to_string())); //~ ERROR: using `drop()` on a `Result` type
    let x = Err::<(), String>("b".to_string());
    drop(x); //~ ERROR: using `drop()` on a `Result` type
    drop(make_result("a".to_string())); //~ ERROR: using `drop()` on a `Result` type
}
