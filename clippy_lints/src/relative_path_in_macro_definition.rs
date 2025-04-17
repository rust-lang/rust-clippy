use clippy_utils::diagnostics::span_lint;
use rustc_ast::ast::{Item, ItemKind};
use rustc_ast::token::TokenKind;
use rustc_ast::tokenstream::{TokenStream, TokenTree};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks that references to the `core` or `kernel` crate in macro definitions use absolute paths (`::core` or `::kernel`).
    ///
    /// ### Why is this bad?
    /// Using relative paths (e.g., `core::...`) in macros can lead to ambiguity if the macro is used in a context
    /// where a user defines a module named `core` or `kernel`. Absolute paths ensure the macro always refers to the intended crate.
    ///
    /// ### Example
    /// ```rust
    /// // Bad
    /// macro_rules! my_macro {
    ///     () => {
    ///         core::mem::drop(0);
    ///     };
    /// }
    ///
    /// // Good
    /// macro_rules! my_macro {
    ///     () => {
    ///         ::core::mem::drop(0);
    ///     };
    /// }
    /// ```
    #[clippy::version = "1.87.0"]
    pub RELATIVE_PATH_IN_MACRO_DEFINITION,
    correctness,
    "using relative paths in declarative macros can lead to context-dependent behavior"
}

declare_lint_pass!(RelativePathInMacroDefinition => [RELATIVE_PATH_IN_MACRO_DEFINITION]);

impl EarlyLintPass for RelativePathInMacroDefinition {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::MacroDef(_, macro_def) = &item.kind {
            check_token_stream(cx, &macro_def.body.tokens);
        }
    }
}

fn check_token_stream(cx: &EarlyContext<'_>, tokens: &TokenStream) {
    let mut iter = tokens.iter().peekable();
    let mut prev_token: Option<&TokenTree> = None;

    while let Some(tree) = iter.next() {
        match tree {
            TokenTree::Token(token, _) => {
                if let TokenKind::Ident(ident, _) = token.kind
                    && (ident.as_str() == "core" || ident.as_str() == "kernel")
                {
                    let is_path_start = iter.peek().is_some_and(|next_tree| {
                        if let TokenTree::Token(next_token, _) = next_tree {
                            next_token.kind == TokenKind::PathSep
                        } else {
                            false
                        }
                    });

                    if is_path_start {
                        let is_absolute = prev_token.is_some_and(|prev| {
                            if let TokenTree::Token(prev_token, _) = prev {
                                prev_token.kind == TokenKind::PathSep
                            } else {
                                false
                            }
                        });

                        if !is_absolute {
                            span_lint(
                                cx,
                                RELATIVE_PATH_IN_MACRO_DEFINITION,
                                token.span,
                                "relative path to `core` or `kernel` used in macro definition",
                            );
                        }
                    }
                }
            },
            TokenTree::Delimited(_open_span, _close_span, _delim, token_stream) => {
                check_token_stream(cx, token_stream);
            },
        }
        prev_token = Some(tree);
    }
}
