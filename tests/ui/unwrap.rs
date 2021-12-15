#![warn(clippy::unwrap_used)]

fn unwrap_option() {
    let opt = Some(0);
    let _ = opt.unwrap();
}

fn unwrap_result() {
    let res: Result<u8, ()> = Ok(0);
    let _ = res.unwrap();
}

fn unwrap_get() {
    let v = vec![1, 2, 3];
    let _ = v.get(0).unwrap();
}

fn unwrap_get_mut() {
    let mut v = vec![1, 2, 3];
    let _ = v.get_mut(0).unwrap();
}

#[warn(clippy::get_unwrap)]
fn unwrap_get2() {
    let v = vec![1, 2, 3];
    let _ = v.get(0).unwrap();
}

#[warn(clippy::get_unwrap)]
fn unwrap_get_mut2() {
    let mut v = vec![1, 2, 3];
    let _ = v.get_mut(0).unwrap();
}

#[warn(clippy::get_unwrap)]
#[allow(clippy::unwrap_used)]
fn unwrap_get3() {
    let v = vec![1, 2, 3];
    let _ = v.get(0).unwrap();
}

#[warn(clippy::get_unwrap)]
#[allow(clippy::unwrap_used)]
fn unwrap_get_mut3() {
    let mut v = vec![1, 2, 3];
    let _ = v.get_mut(0).unwrap();
}

fn main() {
    unwrap_option();
    unwrap_result();
    unwrap_get();
    unwrap_get_mut();
    unwrap_get2();
    unwrap_get_mut2();
    unwrap_get3();
    unwrap_get_mut3();
}
