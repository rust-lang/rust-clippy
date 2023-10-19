#![warn(clippy::unusable_matches_binding)]

#[derive(Clone, Copy)]
struct TestingStructure(i32);

impl TestingStructure {
    fn is_valid(&self) -> bool {
        self.0 > 5
    }
}

fn main() {
    let matching_data_source = TestingStructure(5);
    let unrelated_data = 5;

    let _ = matches!(matching_data_source, TestingStructure(4));

    let _ = matches!(matching_data_source, unusable_binding);
    let _ = matches!(matching_data_source, used_binding if used_binding.is_valid());

    let _ = matches!(matching_data_source, unusable_binding if unrelated_data < 4);
}
