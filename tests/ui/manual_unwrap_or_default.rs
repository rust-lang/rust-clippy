#![warn(clippy::manual_unwrap_or_default)]
#![allow(clippy::unnecessary_literal_unwrap)]

fn main() {
    let x: Option<Vec<String>> = None;
    //~v ERROR: match can be simplified with `.unwrap_or_default()`
    match x {
        Some(v) => v,
        None => Vec::default(),
    };

    let x: Option<Vec<String>> = None;
    //~v ERROR: match can be simplified with `.unwrap_or_default()`
    match x {
        Some(v) => v,
        _ => Vec::default(),
    };

    let x: Option<String> = None;
    //~v ERROR: match can be simplified with `.unwrap_or_default()`
    match x {
        Some(v) => v,
        None => String::new(),
    };

    let x: Option<Vec<String>> = None;
    //~v ERROR: match can be simplified with `.unwrap_or_default()`
    match x {
        None => Vec::default(),
        Some(v) => v,
    };

    let x: Option<Vec<String>> = None;
    //~v ERROR: if let can be simplified with `.unwrap_or_default()`
    if let Some(v) = x {
        v
    } else {
        Vec::default()
    };
}

// Issue #12531
unsafe fn no_deref_ptr(a: Option<i32>, b: *const Option<i32>) -> i32 {
    // `*b` being correct depends on `a == Some(_)`
    match a {
        Some(_) => match *b {
            Some(v) => v,
            _ => 0,
        },
        _ => 0,
    }
}

const fn issue_12568(opt: Option<bool>) -> bool {
    match opt {
        Some(s) => s,
        None => false,
    }
}

fn issue_12569() {
    let match_none_se = match 1u32.checked_div(0) {
        Some(v) => v,
        None => {
            println!("important");
            0
        }
    };
    let match_some_se = match 1u32.checked_div(0) {
        Some(v) => {
            println!("important");
            v
        },
        None => 0
    };
    #[allow(clippy::manual_unwrap_or)]
    let match_none_cm = match 1u32.checked_div(0) {
        Some(v) => v,
        None => {
            // Important
            0
        }
    };
    let match_middle_cm = match 1u32.checked_div(0) {
        Some(v) => v,
        // Important
        None => 0
    };
    let match_middle_doc_cm = match 1u32.checked_div(0) {
        /// Important
        Some(v) => v,
        None => 0
    };
    let iflet_else_se = if let Some(v) = 1u32.checked_div(0) {
        v
    } else {
        println!("important");
        0
    };
    let iflet_then_cm = if let Some(v) = 1u32.checked_div(0) {
        // Important
        v
    } else {
        0
    };
}
