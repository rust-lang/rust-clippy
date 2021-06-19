#![warn(clippy::mixed_locale_ident)]
#[allow(dead_code)]

// Russian `о`.
pub struct Blоck;

fn main() {
    let black_чёрный_黒い_काला = "good luck hand-writing it";
}
