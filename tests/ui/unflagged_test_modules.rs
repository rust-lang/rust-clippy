// run-rustfix
// compile-flags: --test
#![allow(unused)]
#![warn(clippy::unflagged_test_modules)]

fn main() {
    // test code goes here
}

#[cfg(test)]
mod tests {
    fn my_test() {}
}

mod test {
    fn my_other_test() {}
}
