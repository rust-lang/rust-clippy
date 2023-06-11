//@compile-flags: --crate-name=inherit_config
#![warn(clippy::many_single_char_names, clippy::too_many_lines)]

fn main() {
    // Inherited from outer config
    let (a, b, c) = (1, 2, 3);
    _ = ();
    _ = ();
    _ = ();
    // Too many lines, not 1 but 3 because of inner config
}
