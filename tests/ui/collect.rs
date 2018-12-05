// Copyright 2014-2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![warn(clippy::possible_shortcircuiting_collect)]

use std::iter::FromIterator;

pub fn div(a: i32, b: &[i32]) -> Result<Vec<i32>, String> {
    let option_vec: Vec<_> = b.iter()
        .cloned()
        .map(|i| if i != 0 {
            Ok(a / i)
        } else {
            Err("Division by zero!".to_owned())
        })
        .collect();
    let mut int_vec = Vec::new();
    for opt in option_vec {
        int_vec.push(opt?);
    }
    Ok(int_vec)
}

pub fn generic<T>(a: &[T]) {
    // Make sure that our lint also works for generic functions.
    let _result: Vec<_> = a.iter().map(Some).collect();
}

pub fn generic_collection<T, C: FromIterator<T> + FromIterator<Option<T>>>(elem: T) -> C {
    Some(Some(elem)).into_iter().collect()
}

fn main() {
    // We're collecting into an `Option`. Do not trigger lint.
    let _sup: Option<Vec<_>> = (0..5).map(Some).collect();
}
