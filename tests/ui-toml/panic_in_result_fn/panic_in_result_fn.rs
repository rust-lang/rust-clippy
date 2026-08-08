//@compile-flags: --test
//@check-pass
#![warn(clippy::panic_in_result_fn)]
#![allow(clippy::unnecessary_wraps)]

#[test]
fn test_function() -> Result<(), ()> {
    cfg_test_function()?;
    assert!(std::hint::black_box(false));
    tests::helper()
}

#[cfg(test)]
fn cfg_test_function() -> Result<(), ()> {
    panic!();
}

#[cfg(test)]
mod tests {
    pub(super) fn helper() -> Result<(), ()> {
        assert_eq!(std::hint::black_box(1), 2);
        Ok(())
    }
}
