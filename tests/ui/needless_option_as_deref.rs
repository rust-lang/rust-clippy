// run-rustfix

#[warn(clippy::needless_option_as_deref)]

fn main() {
    // should lint
    let _: Option<&usize> = Some(&1).as_deref();

    // false negative, could lint if the source Option is movable and not used later
    let _: Option<&mut usize> = Some(&mut 1).as_deref_mut();

    // should not lint
    let _ = Some(Box::new(1)).as_deref();
    let _ = Some(Box::new(1)).as_deref_mut();

    // #7846
    let mut i = 0;
    let mut opt_vec = vec![Some(&mut i)];
    opt_vec[0].as_deref_mut().unwrap();

    // #8047
    let mut y = 0;
    let mut x = Some(&mut y);
    x.as_deref_mut();
    println!("{:?}", x);
}
