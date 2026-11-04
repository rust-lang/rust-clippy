//@ check-pass
//@aux-build:../auxiliary/proc_macros.rs
extern crate proc_macros;

proc_macros::with_span! {
    span
    #[cfg(test)]
    mod tests {}
}

#[expect(clippy::test_without_assertions)]
#[test]
fn f() {}
