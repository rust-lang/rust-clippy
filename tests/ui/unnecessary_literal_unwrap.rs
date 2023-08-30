#![warn(clippy::unnecessary_literal_unwrap)]
#![allow(unreachable_code)]
#![allow(
    clippy::unnecessary_lazy_evaluations,
    clippy::diverging_sub_expression,
    clippy::let_unit_value,
    clippy::no_effect
)]

fn unwrap_option_some() {
    let _val = Some(1).unwrap();
    //~^ ERROR: used `unwrap()` on `Some` value
    //~| NOTE: `-D clippy::unnecessary-literal-unwrap` implied by `-D warnings`
    let _val = Some(1).expect("this never happens");
    //~^ ERROR: used `expect()` on `Some` value

    Some(1).unwrap();
    //~^ ERROR: used `unwrap()` on `Some` value
    Some(1).expect("this never happens");
    //~^ ERROR: used `expect()` on `Some` value
}

#[rustfmt::skip] // force rustfmt not to remove braces in `|| { 234 }`
fn unwrap_option_none() {
    let _val = None::<()>.unwrap();
    //~^ ERROR: used `unwrap()` on `None` value
    let _val = None::<()>.expect("this always happens");
    //~^ ERROR: used `expect()` on `None` value
    let _val: String = None.unwrap_or_default();
    //~^ ERROR: used `unwrap_or_default()` on `None` value
    let _val: u16 = None.unwrap_or(234);
    //~^ ERROR: used `unwrap_or()` on `None` value
    let _val: u16 = None.unwrap_or_else(|| 234);
    //~^ ERROR: used `unwrap_or_else()` on `None` value
    let _val: u16 = None.unwrap_or_else(|| { 234 });
    //~^ ERROR: used `unwrap_or_else()` on `None` value
    let _val: u16 = None.unwrap_or_else(|| -> u16 { 234 });
    //~^ ERROR: used `unwrap_or_else()` on `None` value

    None::<()>.unwrap();
    //~^ ERROR: used `unwrap()` on `None` value
    None::<()>.expect("this always happens");
    //~^ ERROR: used `expect()` on `None` value
    None::<String>.unwrap_or_default();
    //~^ ERROR: used `unwrap_or_default()` on `None` value
    None::<u16>.unwrap_or(234);
    //~^ ERROR: used `unwrap_or()` on `None` value
    None::<u16>.unwrap_or_else(|| 234);
    //~^ ERROR: used `unwrap_or_else()` on `None` value
    None::<u16>.unwrap_or_else(|| { 234 });
    //~^ ERROR: used `unwrap_or_else()` on `None` value
    None::<u16>.unwrap_or_else(|| -> u16 { 234 });
    //~^ ERROR: used `unwrap_or_else()` on `None` value
}

fn unwrap_result_ok() {
    let _val = Ok::<_, ()>(1).unwrap();
    //~^ ERROR: used `unwrap()` on `Ok` value
    let _val = Ok::<_, ()>(1).expect("this never happens");
    //~^ ERROR: used `expect()` on `Ok` value
    let _val = Ok::<_, ()>(1).unwrap_err();
    //~^ ERROR: used `unwrap_err()` on `Ok` value
    let _val = Ok::<_, ()>(1).expect_err("this always happens");
    //~^ ERROR: used `expect_err()` on `Ok` value

    Ok::<_, ()>(1).unwrap();
    //~^ ERROR: used `unwrap()` on `Ok` value
    Ok::<_, ()>(1).expect("this never happens");
    //~^ ERROR: used `expect()` on `Ok` value
    Ok::<_, ()>(1).unwrap_err();
    //~^ ERROR: used `unwrap_err()` on `Ok` value
    Ok::<_, ()>(1).expect_err("this always happens");
    //~^ ERROR: used `expect_err()` on `Ok` value
}

fn unwrap_result_err() {
    let _val = Err::<(), _>(1).unwrap_err();
    //~^ ERROR: used `unwrap_err()` on `Err` value
    let _val = Err::<(), _>(1).expect_err("this never happens");
    //~^ ERROR: used `expect_err()` on `Err` value
    let _val = Err::<(), _>(1).unwrap();
    //~^ ERROR: used `unwrap()` on `Err` value
    let _val = Err::<(), _>(1).expect("this always happens");
    //~^ ERROR: used `expect()` on `Err` value

    Err::<(), _>(1).unwrap_err();
    //~^ ERROR: used `unwrap_err()` on `Err` value
    Err::<(), _>(1).expect_err("this never happens");
    //~^ ERROR: used `expect_err()` on `Err` value
    Err::<(), _>(1).unwrap();
    //~^ ERROR: used `unwrap()` on `Err` value
    Err::<(), _>(1).expect("this always happens");
    //~^ ERROR: used `expect()` on `Err` value
}

fn unwrap_methods_option() {
    let _val = Some(1).unwrap_or(2);
    //~^ ERROR: used `unwrap_or()` on `Some` value
    let _val = Some(1).unwrap_or_default();
    //~^ ERROR: used `unwrap_or_default()` on `Some` value
    let _val = Some(1).unwrap_or_else(|| 2);
    //~^ ERROR: used `unwrap_or_else()` on `Some` value

    Some(1).unwrap_or(2);
    //~^ ERROR: used `unwrap_or()` on `Some` value
    Some(1).unwrap_or_default();
    //~^ ERROR: used `unwrap_or_default()` on `Some` value
    Some(1).unwrap_or_else(|| 2);
    //~^ ERROR: used `unwrap_or_else()` on `Some` value
}

fn unwrap_methods_result() {
    let _val = Ok::<_, ()>(1).unwrap_or(2);
    //~^ ERROR: used `unwrap_or()` on `Ok` value
    let _val = Ok::<_, ()>(1).unwrap_or_default();
    //~^ ERROR: used `unwrap_or_default()` on `Ok` value
    let _val = Ok::<_, ()>(1).unwrap_or_else(|_| 2);
    //~^ ERROR: used `unwrap_or_else()` on `Ok` value

    Ok::<_, ()>(1).unwrap_or(2);
    //~^ ERROR: used `unwrap_or()` on `Ok` value
    Ok::<_, ()>(1).unwrap_or_default();
    //~^ ERROR: used `unwrap_or_default()` on `Ok` value
    Ok::<_, ()>(1).unwrap_or_else(|_| 2);
    //~^ ERROR: used `unwrap_or_else()` on `Ok` value
}

fn unwrap_from_binding() {
    macro_rules! from_macro {
        () => {
            Some("")
        };
    }
    let val = from_macro!();
    let _ = val.unwrap_or("");
}

fn unwrap_unchecked() {
    let _ = unsafe { Some(1).unwrap_unchecked() };
    //~^ ERROR: used `unwrap_unchecked()` on `Some` value
    let _ = unsafe { Some(1).unwrap_unchecked() + *(&1 as *const i32) }; // needs to keep the unsafe block
    //~^ ERROR: used `unwrap_unchecked()` on `Some` value
    let _ = unsafe { Some(1).unwrap_unchecked() } + 1;
    //~^ ERROR: used `unwrap_unchecked()` on `Some` value
    let _ = unsafe { Ok::<_, ()>(1).unwrap_unchecked() };
    //~^ ERROR: used `unwrap_unchecked()` on `Ok` value
    let _ = unsafe { Ok::<_, ()>(1).unwrap_unchecked() + *(&1 as *const i32) };
    //~^ ERROR: used `unwrap_unchecked()` on `Ok` value
    let _ = unsafe { Ok::<_, ()>(1).unwrap_unchecked() } + 1;
    //~^ ERROR: used `unwrap_unchecked()` on `Ok` value
    let _ = unsafe { Err::<(), i32>(123).unwrap_err_unchecked() };
    //~^ ERROR: used `unwrap_err_unchecked()` on `Err` value
}

fn main() {
    unwrap_option_some();
    unwrap_option_none();
    unwrap_result_ok();
    unwrap_result_err();
    unwrap_methods_option();
    unwrap_methods_result();
    unwrap_unchecked();
}
