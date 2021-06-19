#![warn(clippy::mixed_locale_idents)]
#[allow(dead_code)]

// Russian `о`.
pub struct Blоck;

fn main() {
    let black_чёрный_黒い_काला = "good luck hand-writing it";

    // Should not spawn the lint, as it represents valid sequence in a single locale.
    let nutzer_zähler = "user counter";
}
