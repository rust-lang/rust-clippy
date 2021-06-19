use clippy_utils::diagnostics::span_lint;
use rustc_data_structures::fx::FxHashSet;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{Span, Symbol};
use unicode_script::{Script, UnicodeScript};

declare_clippy_lint! {
    /// **What it does:** Checks for usage of mixed locales in the ident name.
    ///
    /// **Why is this bad?** Using symbols that look like ASCII ones can result in
    /// confusing problems when hand-writing the code.
    ///
    /// `rustc` provides `mixed_script_confusables` lint, but it only works if there is
    /// no single non-confusable symbol. See examples to understand this point.
    ///
    /// Additionally, mixed-case locale idents, even if not confusing, may make code
    /// hard to support due to requiring to switch between multiple locales on keyboard,
    /// e.g.
    ///
    /// ```rust
    /// let black_чёрный_黒い_काला = "good luck hand-writing it";
    /// //   ^      ^    ^    ^─── hindi
    /// //   |      |    └─── japanese
    /// //   |      └─── russian
    /// //   └─── english
    /// ```
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// The following code compiles without any warnings:
    ///
    /// ```rust
    /// struct Blоck { // It's not a common `o`, but rather Russian `о`?
    ///     _щука: String, // Usage of 'щ' will suppress `mixed_script_confusables` warning.
    /// }
    ///
    /// fn main() {
    ///     let _block = Blоck { _щука: "pike".to_string() };
    /// }
    /// ```
    ///
    /// The same example, but with `Block` (english `o`) used instead of `Blоck` (russian `о`).
    /// It will not compile
    ///
    /// ```compile_fail
    /// struct Blоck {
    ///     _щука: String,
    /// }
    ///
    /// fn main() {
    ///     let _block = Block { _щука: "pike".to_string() };
    ///
    /// }
    ///
    /// // Compile output:
    /// //
    /// //    error[E0422]: cannot find struct, variant or union type `Block` in this scope
    /// //    --> src/main.rs:6:18
    /// //     |
    /// //   1 | struct Blоck {
    /// //     | ------------ similarly named struct `Blоck` defined here
    /// //   ...
    /// //   6 |     let _block = Block { _щука: "pike".to_string() };
    /// //     |                  ^^^^^
    /// //     |
    /// //   help: a struct with a similar name exists
    /// //     |
    /// //   6 |     let _block = Blоck { _щука: "pike".to_string() };
    /// //     |                  ^^^^^
    /// //   help: consider importing one of these items
    /// //
    /// ```
    pub MIXED_LOCALE_IDENTS,
    style,
    "multiple locales used in a single identifier"
}

#[derive(Clone, Debug)]
pub struct MixedLocaleIdents;

impl_lint_pass!(MixedLocaleIdents => [MIXED_LOCALE_IDENTS]);

impl<'tcx> LateLintPass<'tcx> for MixedLocaleIdents {
    fn check_name(&mut self, cx: &LateContext<'tcx>, span: Span, ident: Symbol) {
        let ident_name = ident.to_string();

        // First fast pass without any expensive actions just to check
        // whether identifier is fully ASCII.
        // Most of identifiers are *expected* to be ASCII to it's better
        // to return early for all of them.
        if ident_name.is_ascii() {
            return;
        }

        let mut used_locales: FxHashSet<Script> = FxHashSet::default();
        for symbol in ident_name.chars() {
            let script = symbol.script();
            if script != Script::Common && script != Script::Unknown {
                used_locales.insert(script);
            }
        }

        if used_locales.len() > 1 {
            let locales: Vec<&'static str> = used_locales.iter().map(|loc| loc.full_name()).collect();

            let message = format!(
                "multiple locales used in identifier {}: {}",
                ident_name,
                locales.join(", "),
            );

            span_lint(cx, MIXED_LOCALE_IDENTS, span, &message);
        }
    }
}
