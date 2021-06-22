#![warn(clippy::mixed_locale_idents)]
#![allow(dead_code, non_camel_case_types)]

mod should_spawn_warnings {
    // In the examples, cyrillic `о` is used in `Blоck`.

    pub struct Blоck;
    pub const BLОCK: u8 = 42;
    pub fn blоck() {
        let blоck = 42u8;
    }

    // Identifiers that consist of multiple parts.
    pub struct SomeBlоckIdentifier;
    pub const SOME_BLОCK_IDENT: u8 = 42;
    pub fn some_blоck_fn() {
        let some_blоck_var = 42u8;
    }
    pub struct Some_BlоckIdent; // Mixed case

    // Identifiers that have multiple matches.
    // Only the first match should be reported.
    pub struct BlоckClоck;
    pub const BLОCK_CLОCK: u8 = 42;
    pub fn blоck_clоck() {
        let blоck_clоck = 42u8;
    }

    // Identifiers that have both latin & non-latin word, and
    // mixed case word.
    pub struct SomeБлокBlоck;
}

mod should_not_spawn_warnings {
    // In all the examples, `блок` is fully cyrillic.

    pub struct TryБлок;
    pub const TRY_БЛОК: u8 = 42;
    pub fn try_блок() {
        let try_блок_var = 42u8;
    }
    // Mixed case
    pub struct Some_БлокIdent;

    // Using non-confusables in ident together with confusables.
    fn fnъуъ() {
        let try看 = 42u8;
    }

    // Using only lating confusables (`o` is latin).
    fn ooo_блок() {}

    // One-word non-latin identifiers that contain non-confusables.
    fn блок() {}
}

fn main() {
    // Additional examples that should *not* spawn the warning.

    // Should not spawn the lint, as it represents valid sequence in a single locale.
    let nutzer_zähler = "user counter";
}
