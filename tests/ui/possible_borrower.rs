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
