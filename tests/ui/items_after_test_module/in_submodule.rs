//@error-in-other-file:
#[path = "auxiliary/submodule.rs"]
mod submodule;

#[cfg(test)]
mod tests {
    #[expect(clippy::test_without_assertions)]
    #[test]
    fn t() {}
}
