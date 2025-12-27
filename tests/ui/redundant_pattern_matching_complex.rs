#![warn(clippy::redundant_pattern_matching_complex)]
#![allow(
    clippy::needless_bool,
    clippy::needless_ifs,
    clippy::match_like_matches_macro,
    clippy::equatable_if_let,
    clippy::if_same_then_else
)]

fn main() {
    // test code goes here
}
fn issue16235() {
    #![allow(clippy::disallowed_names)]
    enum Baz {
        Qux,
    }

    enum Ban {
        Foo,
        Bar,
    }
    struct Quux {
        #[allow(dead_code)]
        corge: bool,
    }

    let foo = Some((2, 4));
    let bar = Some(Baz::Qux);
    let grault = Some(Quux { corge: true });
    let ban = Some(Ban::Foo);

    if let Some((_, _)) = foo {}
    //~^ redundant_pattern_matching_complex
    if let Some(Baz::Qux) = bar {}
    //~^ redundant_pattern_matching_complex
    if let Some(Quux { corge: _ }) = grault {}
    //~^ redundant_pattern_matching_complex
    if let Some(Ban::Bar) = ban {}
}

fn slices() {
    if let Some([_, _]) = Some([1, 3]) {}
    if let Some([..]) = Some([1, 3]) {}
    //~^ redundant_pattern_matching_complex
    if let Some([]) = Some(&[1, 3] as &[i32]) {}
    //~^ redundant_pattern_matching_complex
}
