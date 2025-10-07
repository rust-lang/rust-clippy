#![warn(clippy::unnecessary_collect)]
//@no-rustfix

fn bad() -> Vec<u32> {
    (0..5).collect()
    //~^ unnecessary_collect
}
unsafe fn bad2() -> Vec<(u8, u8)> {
    (0..8).flat_map(|x| (0..8).map(move |y| (x, y))).collect()
    //~^ unnecessary_collect
}
fn okay() -> String {
    ('a'..='z').collect()
}
fn hmm() -> std::collections::HashSet<u32> {
    (0..5).chain(3..12).collect()
}
fn good() -> impl Iterator<Item = u32> {
    (5..10).rev()
}
fn main() {}
