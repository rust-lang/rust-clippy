#![warn(clippy::manual_ilog2)]

fn main() {
    let a: u32 = 5;
    let _ = 31 - a.leading_zeros();
    let _ = a.ilog(2);

    let b: u64 = 543534;
    let _ = 63 - b.leading_zeros();

    let _ = 64 - b.leading_zeros(); // No lint because manual ilog2 is BIT_WIDTH - 1 - x.leading_zeros()
}
