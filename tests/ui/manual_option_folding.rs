#![feature(option_get_or_insert_default)]
#![allow(dead_code, clippy::unnecessary_lazy_evaluations, clippy::unnecessary_literal_unwrap)]
#![warn(clippy::manual_option_folding)]

fn main() {
    let mut opt: Option<i32> = Some(42);
    opt.get_or_insert_with(|| 21);

    let opt: Option<i32> = Some(42);
    opt.unwrap();

    let mut opt: Option<i32> = Some(42);
    opt.get_or_insert_with(|| 21);
    let _res: i32 = unsafe { opt.unwrap_unchecked() };

    let mut opt: Option<i32> = Some(42);
    opt.get_or_insert_with(|| 21);
    let _res: i32 = opt.unwrap();

    let mut opt: Option<i32> = Some(42);
    opt.get_or_insert_with(|| 21);
    let _res: i32 = opt.expect("msg");

    let mut opt: Option<i32> = Some(42);
    opt.get_or_insert(21);
    let _res: i32 = opt.unwrap();

    let mut opt: Option<i32> = Some(42);
    opt.get_or_insert_default();
    let _res: i32 = opt.unwrap();
}
