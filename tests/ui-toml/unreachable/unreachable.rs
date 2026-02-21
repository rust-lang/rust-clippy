//@compile-flags: --test
#![warn(clippy::unreachable)]

fn main() {
    unreachable!();
    //~^ unreachable
}

#[test]
fn allowed_in_test_fn() {
    unreachable!();
}

#[cfg(test)]
mod tests {
    #[test]
    fn nested_test() {
        unreachable!();
    }

    fn helper() {
        // still test context should be allowed
        unreachable!();
    }
}
