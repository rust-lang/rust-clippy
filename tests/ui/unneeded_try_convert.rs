#![deny(clippy::unneeded_try_convert)]
#![allow(dead_code, unused_imports, clippy::redundant_closure, clippy::needless_question_mark)]

use std::convert::Into;

fn result_string() -> Result<(), String> {
    let option = Some(3);
    option.ok_or_else(|| String::from("foo"))?;
    option.ok_or_else(|| String::from(complex_computation()))?;
    // type arg not fixed
    // option.ok_or_else::<String, _>(|| From::from(complex_computation()))?;
    // type arg not fixed
    // option.ok_or_else::<String, _>(|| "foo".into())?;

    let result: Result<_, &'static str> = Ok(3);
    result.map_err(|_| String::from("foo"))?;
    result.map_err(|_| String::from(complex_computation()))?;
    // type arg not fixed
    // result.map_err::<String, _>(|_| "foo".into())?;
    result.map_err(|x| String::from(x))?;
    result.map_err(|x| String::from(x.trim()))?;
    result.map_err(String::from)?;
    result.map_err::<String, _>(From::from)?;
    result.map_err::<String, _>(Into::into)?;

    Ok(())
}

fn in_closure() {
    let option = Some(3);
    let _ = || -> Result<_, String> { Ok(option.ok_or_else(|| String::from("foo"))?) };
}

#[allow(clippy::option_option)]
fn trivial_closure() {
    let option = Some(3);
    let _ = || -> Result<_, i32> { Ok(option.ok_or_else(|| i32::from(0_u8))?) };
    let x: u8 = 0;
    let _ = || -> Result<_, i32> { Ok(option.ok_or_else(|| i32::from(x))?) };
    const X: u8 = 0;
    let _ = || -> Result<_, i32> { Ok(option.ok_or_else(|| i32::from(X))?) };
    let _ =
        || -> Result<_, Option<Option<i32>>> { Ok(option.ok_or_else(|| Option::<Option<_>>::from(Some(x as i32)))?) };
}

fn result_opt_string() -> Result<(), Option<String>> {
    // can't convert &str -> Option<String> in one step
    let option = Some(3);
    option.ok_or_else(|| String::from("foo"))?;

    let result: Result<_, &'static str> = Ok(3);
    result.map_err(|_| String::from("foo"))?;
    result.map_err(String::from)?;

    Ok(())
}

fn complex_computation() -> &'static str {
    "bar"
}

fn main() {}
