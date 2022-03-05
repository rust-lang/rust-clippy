#![warn(clippy::map_then_identity_transformer)]
#![allow(clippy::redundant_closure)]

fn main() {
    let a = [1, 2, 3];

    // _.map(Path).transformerp(Closure)
    // should lint
    let _ = a.into_iter().map(func1).all(|y| y > 0);
    let _ = a.into_iter().map(func1).any(|y| y > 0);
    let _ = a.into_iter().map(func1).find_map(|y| y.checked_add(1));
    let _ = a.into_iter().map(func1).flat_map(|y| func2(y));
    let _ = a.into_iter().map(func1).filter_map(|y| y.checked_add(1));
    let _ = a.into_iter().map(func1).fold(1, |pd, x| pd * x + 1);
    let _ = a.into_iter().map(func1).map(|y| func1(y));
    let _ = a.into_iter().map(func1).position(|y| y > 0);

    // _.map(Path).transformer(Closure)
    // should lint
    let _ = a.into_iter().map(func1).all(func3);
    let _ = a.into_iter().map(func1).any(func3);

    // _.map(Path).transformer(Closure)
    // should lint
    let _ = a.into_iter().map(|x| func1(x) + 1).all(|y| y > 0);
    let _ = a.into_iter().map(|x| func1(x) * func1(x)).any(|y| y > 0);
    let _ = a.into_iter().map(|x| func1(x) * func1(x)).fold(1, |pd, x| pd * x + 1);

    // _.map(Closure).transformer(Path)
    // should lint
    let _ = a.into_iter().map(|x| func1(x) + 1).all(func3);
    let _ = a.into_iter().map(|x| func1(x) + 1).any(func3);
    let _ = a.into_iter().map(|x| func1(x) + 1).fold(1, func4);

    // should not when the transformer is `find`
    let _ = a.into_iter().map(func1).find(|&y| y > 0);

    // should not lint this because the last param of the closure occurs more than once
    let _ = a.into_iter().map(func1).all(|y| func1(y) * func1(y) > 10);
    let _ = a.into_iter().map(|x| func1(x) + 1).any(|y| func1(y) * func1(y) > 10);
    let _ = a.into_iter().map(func1).fold(1, |pd, x| pd * x * x);

    // should not lint this because the param of the `map` is not within one line
    let _ = a
        .into_iter()
        .map(|x| {
            // This comment has no special meaning:)
            x * x
        })
        .any(func3);
}

fn func1(a: i32) -> i32 {
    unimplemented!();
}

fn func2(a: i32) -> Vec<i32> {
    unimplemented!();
}

fn func3(a: i32) -> bool {
    unimplemented!();
}

fn func4(a: i32, b: i32) -> i32 {
    unimplemented!();
}
