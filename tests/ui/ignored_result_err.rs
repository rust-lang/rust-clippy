#![warn(clippy::ignored_result_err)]

fn some_call() -> Result<i32, String> {
    Ok(42)
}

fn main() {
    // Should lint — if let with else
    if let Ok(res) = some_call() {
        //~^ ignored_result_err
        println!("{res}");
    } else {
        println!("something went wrong");
    }

    // Should lint — if let without else
    if let Ok(res) = some_call() {
        //~^ ignored_result_err
        println!("{res}");
    }

    // Should lint — while let
    while let Ok(res) = some_call() {
        //~^ ignored_result_err
        println!("{res}");
    }

    // Should lint — let else
    let Ok(res) = some_call() else {
        //~^ ignored_result_err
        return;
    };
    println!("{res}");

    // Should NOT lint — match with Err bound
    match some_call() {
        Ok(res) => println!("{res}"),
        Err(e) => println!("failed: {e}"),
    }
}
