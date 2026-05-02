#![warn(clippy::unnecessary_as_slice)]
#![allow(clippy::const_is_empty, unused)]
#![allow(clippy::ptr_arg)]

trait SliceExt {
    fn my_len(&self) -> usize;
}

impl SliceExt for [u32] {
    fn my_len(&self) -> usize {
        self.len()
    }
}

fn by_ref(vals: &mut Vec<u32>) {
    let _ = vals.as_slice().len();
    //~^ unnecessary_as_slice
    let _ = vals.as_slice().is_empty();
    //~^ unnecessary_as_slice
    let _ = vals.as_slice().iter();
    //~^ unnecessary_as_slice
    let _ = vals.as_slice().first();
    //~^ unnecessary_as_slice

    let _ = vals.as_mut_slice().len();
    //~^ unnecessary_as_slice
    let _ = vals.as_mut_slice().is_empty();
    //~^ unnecessary_as_slice
    let _ = vals.as_mut_slice().iter_mut();
    //~^ unnecessary_as_slice
}

fn by_value(mut vals: Vec<u32>) {
    let _ = vals.as_slice().len();
    //~^ unnecessary_as_slice
    let _ = vals.as_mut_slice().len();
    //~^ unnecessary_as_slice
}

fn custom_trait_method(mut vals: Vec<u32>) {
    let _ = vals.as_slice().my_len();
    //~^ unnecessary_as_slice
    let _ = vals.as_mut_slice().my_len();
    //~^ unnecessary_as_slice
}

fn no_lint_cases(vals: &mut Vec<u32>) {
    // Don't lint as_slice() received by a value
    let s = vals.as_slice();
    let _ = s.len();

    fn takes_slice(_: &[u32]) {}
    takes_slice(vals.as_slice());

    let _ = vals.len();
    let _ = vals.is_empty();
}

fn no_lint_non_vec(vals: &[u32]) {
    // vec_as_slice diagnostic item should only applies to Vec::as_slice
    let _ = vals.len();
    let _ = vals.is_empty();
}

fn main() {}
