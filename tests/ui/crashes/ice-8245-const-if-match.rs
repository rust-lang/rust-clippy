// test for https://github.com/rust-lang/rust-clippy/issues/8245
// no enclosing scope for id 7

fn main() {
    let _ = [0; {
        let x = 0;
        if let Some(x) = Some(x) { x } else { 1 }
    }];
}
