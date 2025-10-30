#![warn(clippy::set_env_in_tests)]

use std::env;

fn main() {
    unsafe { env::set_var("CLIPPY_TESTS_THIS_IS_OK", "1") }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::env::set_var;

    #[test]
    fn my_test() {
        unsafe { set_var("CLIPPY_TESTS_THIS_IS_NOT_OK", "1") }
        //~^ set_env_in_tests

        unsafe { env::set_var("CLIPPY_TESTS_THIS_IS_NOT_OK", "1") }
        //~^ set_env_in_tests

        unsafe { std::env::set_var("CLIPPY_TESTS_THIS_IS_NOT_OK", "1") }
        //~^ set_env_in_tests
    }
}
