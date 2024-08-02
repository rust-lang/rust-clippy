#[path = "auxiliary/submodule.rs"]
mod submodule;
//~^ items_after_test_module

#[cfg(test)]
mod tests {
    #[test]
    fn t() {}
}
