fn main() {
    let _ = std::fs::write("x", vec![0]); //~ needless_conversion_for_trait
}
