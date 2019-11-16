#![warn(clippy::result_map_unwrap_or)]

fn main() {
    let res: Result<usize, ()> = Ok(1);

    // Check `RESULT_MAP_OR_NONE`.
    // Single line case.
    let _ = res.map(|x| x + 1).unwrap_or(0);
    // Multi-line case.
    #[rustfmt::skip]
    let _ = res.map(|x| {
                        x + 1
                       }).unwrap_or(0);
}
