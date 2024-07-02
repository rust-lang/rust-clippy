use rustc_ast::ast::{Attribute, MacCall};
use rustc_ast::token::{LitKind, TokenKind};
use rustc_ast::tokenstream::{TokenStream, TokenTree};
use rustc_ast::MetaItem;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::{sym, Span, Symbol};

use clippy_utils::diagnostics::{span_lint, span_lint_and_sugg};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `#[cfg]`/`#[cfg_attr]`/`cfg!()` in `build.rs` scripts.
    ///
    /// ### Why is this bad?
    /// It checks the `cfg` values for the *host*, not the target. For example, `cfg!(windows)` is
    /// true when compiling on Windows, so it will give the wrong answer if you are cross compiling.
    /// This is because build scripts run on the machine performing compilation, rather than on the
    /// target.
    ///
    /// ### Example
    /// ```no_run
    /// if cfg!(windows) {}
    /// ```
    /// Use instead:
    /// ```no_run
    /// if std::env::var("CARGO_CFG_WINDOWS").is_ok() {}
    /// ```
    #[clippy::version = "1.80.0"]
    pub INVALID_BUILD_CFG,
    suspicious,
    "invalid use of cfg in `build.rs`"
}

declare_lint_pass!(InvalidBuildCfg => [INVALID_BUILD_CFG]);

#[derive(Debug)]
enum CfgAst {
    Os(Symbol),
    Any(Vec<CfgAst>),
    All(Vec<CfgAst>),
    Not(Box<CfgAst>),
    TargetKeyValue(Symbol, Symbol),
    Feature(Symbol),
    OtherTarget(Symbol, Symbol),
}

impl CfgAst {
    fn has_only_features(&self) -> bool {
        match self {
            Self::Os(_) | Self::TargetKeyValue(_, _) | Self::OtherTarget(_, _) => false,
            Self::Any(v) | Self::All(v) => v.is_empty() || v.iter().all(CfgAst::has_only_features),
            Self::Not(v) => v.has_only_features(),
            Self::Feature(_) => true,
        }
    }

    fn generate_replacement(&self) -> String {
        self.generate_replacement_inner(true, false)
    }

    fn generate_replacement_inner(&self, is_top_level: bool, parent_is_not: bool) -> String {
        match self {
            Self::Os(os) => format!(
                "std::env::var(\"CARGO_CFG_{}\"){}",
                os.as_str().to_uppercase(),
                if parent_is_not { ".is_err()" } else { ".is_ok()" },
            ),
            Self::TargetKeyValue(cfg_target, s) => format!(
                "{}std::env::var(\"CARGO_CFG_{}\").unwrap_or_default() == \"{s}\"",
                if parent_is_not { "!" } else { "" },
                cfg_target.as_str().to_uppercase(),
            ),
            Self::OtherTarget(cfg_target, s) => format!(
                "{}std::env::var(\"CARGO_CFG_{}\").unwrap_or_default().split(',').any(|f| f == \"{s}\")",
                if parent_is_not { "!" } else { "" },
                cfg_target.as_str().to_uppercase(),
            ),
            Self::Any(v) => {
                if v.is_empty() {
                    if parent_is_not { "true" } else { "false" }.to_string()
                } else if v.len() == 1 {
                    v[0].generate_replacement_inner(is_top_level, parent_is_not)
                } else {
                    format!(
                        "{not}{open_paren}{cond}{closing_paren}",
                        not = if parent_is_not { "!" } else { "" },
                        open_paren = if !parent_is_not && is_top_level { "" } else { "(" },
                        cond = v
                            .iter()
                            .map(|i| i.generate_replacement_inner(false, false))
                            .collect::<Vec<_>>()
                            .join(" || "),
                        closing_paren = if !parent_is_not && is_top_level { "" } else { ")" },
                    )
                }
            },
            Self::All(v) => {
                if v.is_empty() {
                    if parent_is_not { "false" } else { "true" }.to_string()
                } else if v.len() == 1 {
                    v[0].generate_replacement_inner(is_top_level, parent_is_not)
                } else {
                    format!(
                        "{not}{open_paren}{cond}{closing_paren}",
                        not = if parent_is_not { "!" } else { "" },
                        open_paren = if !parent_is_not && is_top_level { "" } else { "(" },
                        cond = v
                            .iter()
                            .map(|i| i.generate_replacement_inner(false, false))
                            .collect::<Vec<_>>()
                            .join(" && "),
                        closing_paren = if !parent_is_not && is_top_level { "" } else { ")" },
                    )
                }
            },
            Self::Not(i) => i.generate_replacement_inner(is_top_level, true),
            Self::Feature(s) => format!(
                "cfg!({}feature = {s}{})",
                if parent_is_not { "not(" } else { "" },
                if parent_is_not { ")" } else { "" },
            ),
        }
    }
}

fn parse_macro_args(tokens: &TokenStream, has_unknown: &mut bool, out: &mut Vec<CfgAst>) {
    let mut tokens = tokens.trees().peekable();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Token(token, _) => {
                match token.kind {
                    TokenKind::Ident(name, _) => {
                        if name == sym::feature || name.as_str().starts_with("target_") {
                            if let Some(next_token) = tokens.next()
                                && let TokenTree::Token(next_token, _) = next_token
                                && matches!(next_token.kind, TokenKind::Eq)
                                && let Some(next_token) = tokens.next()
                                && let TokenTree::Token(next_token, _) = next_token
                                && let TokenKind::Literal(lit) = next_token.kind
                                && matches!(lit.kind, LitKind::Str | LitKind::StrRaw(_))
                            {
                                if [
                                    sym::target_os,
                                    sym::target_endian,
                                    sym::target_pointer_width,
                                    sym::target_vendor,
                                    sym::target_has_atomic,
                                ]
                                .contains(&name)
                                {
                                    out.push(CfgAst::TargetKeyValue(name, lit.symbol));
                                } else if name == sym::feature {
                                    out.push(CfgAst::Feature(lit.symbol));
                                } else {
                                    out.push(CfgAst::OtherTarget(name, lit.symbol));
                                }
                            } else {
                                *has_unknown = true;
                                return;
                            }
                        } else if [sym::windows, sym::unix].contains(&name) {
                            out.push(CfgAst::Os(name));
                        } else if [sym::any, sym::not, sym::all].contains(&name) {
                            let mut sub_out = Vec::new();

                            if let Some(next_token) = tokens.next()
                                && let TokenTree::Delimited(_, _, _, stream) = next_token
                            {
                                parse_macro_args(stream, has_unknown, &mut sub_out);
                            } else {
                                *has_unknown = true;
                            }
                            if *has_unknown {
                                // Could be set inside `parse_macro_args` too.
                                return;
                            }
                            if name == sym::any {
                                out.push(CfgAst::Any(sub_out));
                            } else if name == sym::all {
                                out.push(CfgAst::All(sub_out));
                            } else if sub_out.len() > 1 {
                                *has_unknown = true;
                                return;
                            } else if sub_out.len() == 1 {
                                out.push(CfgAst::Not(Box::new(sub_out.pop().unwrap())));
                            }
                        } else {
                            *has_unknown = true;
                            return;
                        }
                    },
                    TokenKind::Comma => {},
                    _ => {
                        *has_unknown = true;
                        return;
                    },
                }
            },
            TokenTree::Delimited(_, _, _, stream) => parse_macro_args(stream, has_unknown, out),
        }
    }
}

fn get_invalid_cfg_attrs(attr: &MetaItem, spans: &mut Vec<Span>, has_feature: &mut bool) {
    let Some(ident) = attr.ident() else { return };
    if ident.name == sym::feature {
        *has_feature = true;
        return;
    }
    if attr.value_str().is_some() {
        spans.push(attr.span);
    } else if let Some(sub_attrs) = attr.meta_item_list() {
        for sub_attr in sub_attrs {
            if let Some(meta) = sub_attr.meta_item() {
                get_invalid_cfg_attrs(meta, spans, has_feature);
            }
        }
    } else {
        spans.push(attr.span);
    }
}

fn is_build_script(cx: &EarlyContext<'_>) -> bool {
    cx.sess()
        .opts
        .crate_name
        .as_ref()
        .map_or(false, |crate_name| crate_name == "build_script_build")
}

impl EarlyLintPass for InvalidBuildCfg {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        if !is_build_script(cx) {
            return;
        }

        let name = attr.name_or_empty();
        let mut spans = Vec::new();
        let mut has_feature = false;
        if name == sym::cfg {
            if let Some(meta) = attr.meta() {
                get_invalid_cfg_attrs(&meta, &mut spans, &mut has_feature);
            }
        } else if name == sym::cfg_attr {
            if let Some(sub_attrs) = attr.meta_item_list()
                && let Some(meta) = sub_attrs.first().and_then(|a| a.meta_item())
            {
                get_invalid_cfg_attrs(meta, &mut spans, &mut has_feature);
            }
        } else {
            return;
        }
        if !spans.is_empty() {
            if has_feature {
                // If the `cfg`/`cfg_attr` attribute contains not only invalid items, we display
                // spans of all invalid items.
                span_lint(
                    cx,
                    INVALID_BUILD_CFG,
                    spans,
                    format!("`{name}` attribute should not be used in `build.rs`"),
                );
            } else {
                // No "good" item in the `cfg`/`cfg_attr` attribute so we can use the span of the
                // whole attribute directly.
                span_lint(
                    cx,
                    INVALID_BUILD_CFG,
                    attr.span,
                    format!("`{name}` attribute should not be used in `build.rs`"),
                );
            }
        }
    }

    fn check_mac(&mut self, cx: &EarlyContext<'_>, call: &MacCall) {
        if !is_build_script(cx) {
            return;
        }

        if call.path.segments.len() == 1 && call.path.segments[0].ident.name == sym::cfg {
            let mut ast = Vec::new();
            let mut has_unknown = false;
            parse_macro_args(&call.args.tokens, &mut has_unknown, &mut ast);
            if has_unknown || ast.len() > 1 {
                span_lint(
                    cx,
                    INVALID_BUILD_CFG,
                    call.span(),
                    "`cfg!` macro should not be used in `build.rs`",
                );
            } else if !ast[0].has_only_features() {
                span_lint_and_sugg(
                    cx,
                    INVALID_BUILD_CFG,
                    call.span(),
                    "`cfg!` macro should not be used in `build.rs`",
                    "use cargo environment variables if possible",
                    ast[0].generate_replacement(),
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}
