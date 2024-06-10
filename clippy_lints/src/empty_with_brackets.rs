use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{IntoSpan, SpanRangeExt};
use rustc_ast::ast::{Item, ItemKind, Variant, VariantData};
use rustc_errors::Applicability;
use rustc_lexer::TokenKind;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Finds structs without fields (a so-called "empty struct") that are declared with brackets.
    ///
    /// ### Why restrict this?
    /// Empty brackets after a struct declaration can be omitted,
    /// and it may be desirable to do so consistently for style.
    ///
    /// However, removing the brackets also introduces a public constant named after the struct,
    /// so this is not just a syntactic simplification but an an API change, and adding them back
    /// is a *breaking* API change.
    ///
    /// ### Example
    /// ```no_run
    /// struct Cookie {}
    /// struct Biscuit();
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct Cookie;
    /// struct Biscuit;
    /// ```
    #[clippy::version = "1.62.0"]
    pub EMPTY_STRUCTS_WITH_BRACKETS,
    restriction,
    "finds struct declarations with empty brackets"
}

declare_clippy_lint! {
    /// ### What it does
    /// Finds enum variants without fields that are declared with empty brackets.
    ///
    /// ### Why restrict this?
    /// Empty brackets after a enum variant declaration are redundant and can be omitted,
    /// and it may be desirable to do so consistently for style.
    ///
    /// However, removing the brackets also introduces a public constant named after the variant,
    /// so this is not just a syntactic simplification but an an API change, and adding them back
    /// is a *breaking* API change.
    ///
    /// ### Example
    /// ```no_run
    /// enum MyEnum {
    ///     HasData(u8),
    ///     HasNoData(),       // redundant parentheses
    ///     NoneHereEither {}, // redundant braces
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// enum MyEnum {
    ///     HasData(u8),
    ///     HasNoData,
    ///     NoneHereEither,
    /// }
    /// ```
    #[clippy::version = "1.77.0"]
    pub EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
    restriction,
    "finds enum variants with empty brackets"
}

declare_lint_pass!(EmptyWithBrackets => [EMPTY_STRUCTS_WITH_BRACKETS, EMPTY_ENUM_VARIANTS_WITH_BRACKETS]);

impl EarlyLintPass for EmptyWithBrackets {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Struct(var_data, _) = &item.kind {
            check(
                cx,
                var_data,
                item.span,
                item.ident.span,
                EMPTY_STRUCTS_WITH_BRACKETS,
                "non-unit struct contains no fields",
                true,
            );
        }
    }

    fn check_variant(&mut self, cx: &EarlyContext<'_>, variant: &Variant) {
        check(
            cx,
            &variant.data,
            variant.span,
            variant.ident.span,
            EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
            "non-unit variant contains no fields",
            false,
        );
    }
}

fn check(
    cx: &EarlyContext<'_>,
    data: &VariantData,
    item_sp: Span,
    name_sp: Span,
    lint: &'static Lint,
    msg: &'static str,
    needs_semi: bool,
) {
    let (fields, has_semi, start_char, end_char, help_msg) = match &data {
        VariantData::Struct { fields, .. } => (fields, false, '{', '}', "remove the braces"),
        VariantData::Tuple(fields, _) => (fields, needs_semi, '(', ')', "remove the parentheses"),
        VariantData::Unit(_) => return,
    };
    if fields.is_empty()
        && !item_sp.from_expansion()
        && !name_sp.from_expansion()
        && let name_hi = name_sp.hi()
        && let Some(err_range) = (name_hi..item_sp.hi()).clone().map_range(cx, |src, range| {
            let src = src.get(range.clone())?;
            let (src, end) = if has_semi {
                (src.strip_suffix(';')?, range.end - 1)
            } else {
                (src, range.end)
            };
            let trimmed = src.trim_start();
            let start = range.start + (src.len() - trimmed.len());
            // Proc-macro check.
            let trimmed = trimmed.strip_prefix(start_char)?.strip_suffix(end_char)?;
            // Check for anything inside the brackets, including comments.
            rustc_lexer::tokenize(trimmed)
                .all(|tt| matches!(tt.kind, TokenKind::Whitespace))
                .then_some(start..end)
        })
    {
        span_lint_and_then(cx, lint, err_range.clone().into_span(), msg, |diagnostic| {
            diagnostic.span_suggestion_hidden(
                (name_hi..err_range.end).into_span(),
                help_msg,
                if has_semi || !needs_semi {
                    String::new()
                } else {
                    ";".into()
                },
                Applicability::MaybeIncorrect,
            );
        });
    }
}
