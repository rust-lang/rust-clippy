#![warn(clippy::map_then_identity_transformer)]
#![allow(clippy::redundant_closure)]

fn main() {
    let a = [1, 2, 3];

    // should lint
    let _ = a.into_iter().map(|x| func1(x)).all(|y| y > 0);
    let _ = a.into_iter().map(|x| func1(x)).any(|y| y > 0);
    let _ = a.into_iter().map(|x| func1(x)).find(|&y| y > 0);
    let _ = a.into_iter().map(|x| func1(x)).find_map(|y| y.checked_add(1));
    let _ = a.into_iter().map(|x| func1(x)).flat_map(|y| func2(y));
    let _ = a.into_iter().map(|x| func1(x)).filter_map(|y| y.checked_add(1));
    let _ = a.into_iter().map(|x| func1(x)).fold(1, |pd, x| pd * x + 1);
    let _ = a.into_iter().map(|x| func1(x)).map(|y| func1(y));
    let _ = a.into_iter().map(|x| func1(x)).position(|y| y > 0);

    // should lint
    let _ = a.into_iter().map(|x| func1(x) * func1(x)).all(|y| y > 0);

    // should not lint
    let _ = a.into_iter().map(|x| func1(x)).all(|y| func1(y) * func1(y) > 0);
    let _ = a.into_iter().map(|x| func1(x)).any(|y| func1(y) * func1(y) > 0);
    let _ = a.into_iter().map(|x| func1(x)).fold(1, |pd, x| pd * x * x);

    // should not lint
    let _ = a
        .into_iter()
        .map(|x| {
            // This comment has no special meaning:)
            x * x
        })
        .any(|y| y > 10);
}

fn func1(a: i32) -> i32 {
    unimplemented!();
}

fn func2(a: i32) -> Vec<i32> {
    unimplemented!();
}
