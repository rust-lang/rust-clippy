use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_opt;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::def::CtorOf;
use rustc_hir::def::DefKind::Ctor;
use rustc_hir::def::Res::Def;
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind, Item, ItemKind, Path, QPath, Variant, VariantData};
use rustc_lexer::TokenKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
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

#[derive(Default)]
pub struct EmptyWithBrackets {
    empty_tuple_enum_variants: FxHashSet<(DefId, Span)>,
    enum_variants_used_as_functions: FxHashSet<DefId>,
}

impl_lint_pass!(EmptyWithBrackets => [EMPTY_STRUCTS_WITH_BRACKETS, EMPTY_ENUM_VARIANTS_WITH_BRACKETS]);

impl LateLintPass<'_> for EmptyWithBrackets {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        let span_after_ident = item.span.with_lo(item.ident.span.hi());

        if let ItemKind::Struct(var_data, _) = &item.kind
            && has_brackets(var_data)
            && has_no_fields(cx, var_data, span_after_ident)
        {
            span_lint_and_then(
                cx,
                EMPTY_STRUCTS_WITH_BRACKETS,
                span_after_ident,
                "found empty brackets on struct declaration",
                |diagnostic| {
                    diagnostic.span_suggestion_hidden(
                        span_after_ident,
                        "remove the brackets",
                        ";",
                        Applicability::Unspecified,
                    );
                },
            );
        }
    }

    fn check_variant(&mut self, cx: &LateContext<'_>, variant: &Variant<'_>) {
        // Don't lint pub enums
        if cx.effective_visibilities.is_reachable(variant.def_id) {
            return;
        }

        let span_after_ident = variant.span.with_lo(variant.ident.span.hi());

        if has_no_fields(cx, &variant.data, span_after_ident) {
            match variant.data {
                VariantData::Struct { .. } => {
                    span_lint_and_then(
                        cx,
                        EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
                        span_after_ident,
                        "enum variant has empty brackets",
                        |diagnostic| {
                            diagnostic.span_suggestion_hidden(
                                span_after_ident,
                                "remove the brackets",
                                "",
                                Applicability::MaybeIncorrect,
                            );
                        },
                    );
                },
                VariantData::Tuple(..) => {
                    if let Some(x) = variant.data.ctor_def_id() {
                        self.empty_tuple_enum_variants.insert((x.to_def_id(), span_after_ident));
                    }
                },
                VariantData::Unit(..) => {},
            }
        }
    }

    fn check_expr(&mut self, _cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let Some(def_id) = check_expr_for_enum_as_function(expr) {
            self.enum_variants_used_as_functions.insert(def_id);
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'_>) {
        for &(_, span) in self
            .empty_tuple_enum_variants
            .iter()
            .filter(|(variant, _)| !self.enum_variants_used_as_functions.contains(variant))
        {
            span_lint_and_then(
                cx,
                EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
                span,
                "enum variant has empty brackets",
                |diagnostic| {
                    diagnostic.span_suggestion_hidden(span, "remove the brackets", "", Applicability::MaybeIncorrect);
                },
            );
        }
    }
}

fn has_no_ident_token(braces_span_str: &str) -> bool {
    !rustc_lexer::tokenize(braces_span_str).any(|t| t.kind == TokenKind::Ident)
}

fn has_brackets(var_data: &VariantData<'_>) -> bool {
    !matches!(var_data, VariantData::Unit(..))
}

fn has_no_fields(cx: &LateContext<'_>, var_data: &VariantData<'_>, braces_span: Span) -> bool {
    if !var_data.fields().is_empty() {
        return false;
    }

    // there might still be field declarations hidden from the AST
    // (conditionally compiled code using #[cfg(..)])

    let Some(braces_span_str) = snippet_opt(cx, braces_span) else {
        return false;
    };

    has_no_ident_token(braces_span_str.as_ref())
}

fn check_expr_for_enum_as_function(expr: &Expr<'_>) -> Option<DefId> {
    let ExprKind::Path(QPath::Resolved(
        _,
        Path {
            res: Def(Ctor(CtorOf::Variant, _), def_id),
            ..
        },
    )) = expr.kind
    else {
        return None;
    };
    Some(*def_id)
}

#[cfg(test)]
mod unit_test {
    use super::*;

    #[test]
    fn test_has_no_ident_token() {
        let input = "{ field: u8 }";
        assert!(!has_no_ident_token(input));

        let input = "(u8, String);";
        assert!(!has_no_ident_token(input));

        let input = " {
                // test = 5
        }
        ";
        assert!(has_no_ident_token(input));

        let input = " ();";
        assert!(has_no_ident_token(input));
    }
}
