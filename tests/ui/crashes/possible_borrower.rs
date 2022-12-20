// https://github.com/rust-lang/rust-clippy/issues/10134

fn meow(_s: impl AsRef<str>) {}

macro_rules! quad {
    ($x:stmt) => {
        $x
        $x
        $x
        $x
    };
}

fn main() {
    let i = 0;
    quad!(quad!(quad!(quad!(quad!(meow(format!("abc{i}")))))));
}

// https://github.com/rust-lang/rust-clippy/issues/10134#issuecomment-1374480660
fn second_testcase() {
    quad!(quad!(quad!(for i in 0..4 {
        quad!(quad!(meow(format!("abc{i}"))));
    })));
}
