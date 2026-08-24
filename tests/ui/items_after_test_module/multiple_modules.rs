//@ check-pass
#![expect(clippy::test_without_assertions)]

#[cfg(test)]
mod tests {
    #[test]
    fn f() {}
}

#[cfg(test)]
mod more_tests {
    #[test]
    fn g() {}
}
