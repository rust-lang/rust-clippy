#![allow(unused)]
#![warn(clippy::option_is_some_with)]

#[rustfmt::skip]
fn option_methods() {
    let opt = Some(1);

    // Check for `option.map(_).unwrap_or(_)` use.
    // Single line case.
    let _ = opt.map(|x| x + 1)
        // Should lint even though this call is on a separate line.
        .unwrap_or(0);
    // Multi-line cases.
    let _ = opt.map(|x| {
        x + 1
    }
    ).unwrap_or(0);
    let _ = opt.map(|x| x + 1)
        .unwrap_or({
            0
        });
    // Single line `map(f).unwrap_or(None)` case.
    let _ = opt.map(|x| Some(x + 1)).unwrap_or(None);
    // Multi-line `map(f).unwrap_or(None)` cases.
    let _ = opt.map(|x| {
        Some(x + 1)
    }
    ).unwrap_or(None);
    let _ = opt
        .map(|x| Some(x + 1))
        .unwrap_or(None);

    // Should not lint if not copyable
    let id: String = "identifier".to_string();
    let _ = Some("prefix").map(|p| format!("{}.{}", p, id)).unwrap_or(id);
    // ...but DO lint if the `unwrap_or` argument is not used in the `map`
    let id: String = "identifier".to_string();
    let _ = Some("prefix").map(|p| format!("{}.", p)).unwrap_or(id);

    // Check for `option.map(_).unwrap_or_else(_)` use.
    // Multi-line cases.
    let _ = opt.map(|x| {
        x + 1
    }
    ).unwrap_or_else(|| 0);
    let _ = opt.map(|x| x + 1)
        .unwrap_or_else(||
            0
        );

    // If the argument to unwrap_or is false, suggest is_some_and instead
    let _ = opt.map(|x| x > 5).unwrap_or(false);
}

#[rustfmt::skip]
fn result_methods() {
    let res: Result<i32, ()> = Ok(1);

    // Check for `result.map(_).unwrap_or_else(_)` use.
    // multi line cases
    let _ = res.map(|x| {
        x + 1
    }
    ).unwrap_or_else(|_e| 0);
    let _ = res.map(|x| x + 1)
        .unwrap_or_else(|_e| {
            0
        });
}

fn main() {
    option_methods();
    result_methods();
}

#[clippy::msrv = "1.40"]
fn msrv_1_40() {
    let res: Result<i32, ()> = Ok(1);

    let _ = res.map(|x| x + 1).unwrap_or_else(|_e| 0);
}

#[clippy::msrv = "1.41"]
fn msrv_1_41() {
    let res: Result<i32, ()> = Ok(1);

    let _ = res.map(|x| x + 1).unwrap_or_else(|_e| 0);
}
