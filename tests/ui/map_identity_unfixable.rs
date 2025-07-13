//@no-rustfix
#![warn(clippy::map_identity)]

fn main() {
    let mut index = [true, true, false, false, false, true].iter();
    let subindex = (index.by_ref().take(3), 42);
    let _ = subindex.0.map(|n| n).next();
    //~^ map_identity
}
