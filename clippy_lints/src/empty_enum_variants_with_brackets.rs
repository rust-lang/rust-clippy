use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_opt;
use rustc_ast::ast::{Variant, VariantData};
use rustc_errors::Applicability;
use rustc_lexer::TokenKind;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Finds enum variants without fields that are declared with brackets.
    /// ### Why is this bad?
    /// Empty brackets while defining enum variants are redundant and can be omitted.
    /// ### Example
    /// ```no_run
    /// enum Season {
    ///     Summer(), // redundant parentheses
    ///     Winter{}, // redundant braces
    ///     Spring
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// enum Season {
    ///     Summer,
    ///     Winter,
    ///     Spring
    /// }
    /// ```
    #[clippy::version = "1.76.0"]
    pub EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
    restriction,
    "enum variant has empty body with redundant brackets"
}

declare_lint_pass!(EmptyEnumVariantsWithBrackets => [EMPTY_ENUM_VARIANTS_WITH_BRACKETS]);

impl EarlyLintPass for EmptyEnumVariantsWithBrackets {
    fn check_variant(&mut self, cx: &EarlyContext<'_>, variant: &Variant) {
        let span_after_ident = variant.span.with_lo(variant.ident.span.hi());
        if has_brackets(&variant.data) && has_no_fields(cx, &variant.data, span_after_ident) {
            span_lint_and_then(
                cx,
                EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
                span_after_ident,
                "enum variant with brackets has empty body",
                |diagnostic| {
                    diagnostic.span_suggestion_hidden(
                        span_after_ident,
                        "remove the brackets",
                        "",
                        Applicability::MachineApplicable,
                    );
                },
            );
        }
    }
}

fn has_brackets(var_data: &VariantData) -> bool {
    !matches!(var_data, VariantData::Unit(_))
}

fn has_no_fields(cx: &EarlyContext<'_>, var_data: &VariantData, braces_span: Span) -> bool {
    if !var_data.fields().is_empty() {
        return false;
    }
    let Some(braces_span_str) = snippet_opt(cx, braces_span) else {
        return false;
    };

    has_no_ident_token(braces_span_str.as_ref())
}

fn has_no_ident_token(braces_span_str: &str) -> bool {
    !rustc_lexer::tokenize(braces_span_str).any(|t| t.kind == TokenKind::Ident)
}
