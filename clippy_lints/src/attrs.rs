//! checks for attributes

use clippy_config::msrvs::{self, Msrv};
use clippy_utils::diagnostics::{span_lint, span_lint_and_help, span_lint_and_sugg, span_lint_and_then};
use clippy_utils::is_from_proc_macro;
use clippy_utils::macros::{is_panic, macro_backtrace};
use clippy_utils::source::{first_line_of_span, is_present_in_source, snippet_opt, without_block_comments};
use rustc_ast::token::{Token, TokenKind};
use rustc_ast::tokenstream::TokenTree;
use rustc_ast::{
    AttrArgs, AttrArgsEq, AttrKind, AttrStyle, Attribute, LitKind, MetaItemKind, MetaItemLit, NestedMetaItem,
};
use rustc_errors::Applicability;
use rustc_hir::{
    Block, Expr, ExprKind, ImplItem, ImplItemKind, Item, ItemKind, StmtKind, TraitFn, TraitItem, TraitItemKind,
};
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass, Level, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, impl_lint_pass};
use rustc_span::symbol::Symbol;
use rustc_span::{sym, Span, DUMMY_SP};
use semver::Version;

static UNIX_SYSTEMS: &[&str] = &[
    "android",
    "dragonfly",
    "emscripten",
    "freebsd",
    "fuchsia",
    "haiku",
    "illumos",
    "ios",
    "l4re",
    "linux",
    "macos",
    "netbsd",
    "openbsd",
    "redox",
    "solaris",
    "vxworks",
];

// NOTE: windows is excluded from the list because it's also a valid target family.
static NON_UNIX_SYSTEMS: &[&str] = &["hermit", "none", "wasi"];

declare_clippy_lint! {
    /// ### What it does
    /// Checks for items annotated with `#[inline(always)]`,
    /// unless the annotated function is empty or simply panics.
    ///
    /// ### Why is this bad?
    /// While there are valid uses of this annotation (and once
    /// you know when to use it, by all means `allow` this lint), it's a common
    /// newbie-mistake to pepper one's code with it.
    ///
    /// As a rule of thumb, before slapping `#[inline(always)]` on a function,
    /// measure if that additional function call really affects your runtime profile
    /// sufficiently to make up for the increase in compile time.
    ///
    /// ### Known problems
    /// False positives, big time. This lint is meant to be
    /// deactivated by everyone doing serious performance work. This means having
    /// done the measurement.
    ///
    /// ### Example
    /// ```ignore
    /// #[inline(always)]
    /// fn not_quite_hot_code(..) { ... }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub INLINE_ALWAYS,
    pedantic,
    "use of `#[inline(always)]`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `extern crate` and `use` items annotated with
    /// lint attributes.
    ///
    /// This lint permits lint attributes for lints emitted on the items themself.
    /// For `use` items these lints are:
    /// * deprecated
    /// * unreachable_pub
    /// * unused_imports
    /// * clippy::enum_glob_use
    /// * clippy::macro_use_imports
    /// * clippy::wildcard_imports
    ///
    /// For `extern crate` items these lints are:
    /// * `unused_imports` on items with `#[macro_use]`
    ///
    /// ### Why is this bad?
    /// Lint attributes have no effect on crate imports. Most
    /// likely a `!` was forgotten.
    ///
    /// ### Example
    /// ```ignore
    /// #[deny(dead_code)]
    /// extern crate foo;
    /// #[forbid(dead_code)]
    /// use foo::bar;
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// #[allow(unused_imports)]
    /// use foo::baz;
    /// #[allow(unused_imports)]
    /// #[macro_use]
    /// extern crate baz;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub USELESS_ATTRIBUTE,
    correctness,
    "use of lint attributes on `extern crate` items"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `#[deprecated]` annotations with a `since`
    /// field that is not a valid semantic version. Also allows "TBD" to signal
    /// future deprecation.
    ///
    /// ### Why is this bad?
    /// For checking the version of the deprecation, it must be
    /// a valid semver. Failing that, the contained information is useless.
    ///
    /// ### Example
    /// ```no_run
    /// #[deprecated(since = "forever")]
    /// fn something_else() { /* ... */ }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub DEPRECATED_SEMVER,
    correctness,
    "use of `#[deprecated(since = \"x\")]` where x is not semver"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for empty lines after outer attributes
    ///
    /// ### Why is this bad?
    /// Most likely the attribute was meant to be an inner attribute using a '!'.
    /// If it was meant to be an outer attribute, then the following item
    /// should not be separated by empty lines.
    ///
    /// ### Known problems
    /// Can cause false positives.
    ///
    /// From the clippy side it's difficult to detect empty lines between an attributes and the
    /// following item because empty lines and comments are not part of the AST. The parsing
    /// currently works for basic cases but is not perfect.
    ///
    /// ### Example
    /// ```no_run
    /// #[allow(dead_code)]
    ///
    /// fn not_quite_good_code() { }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// // Good (as inner attribute)
    /// #![allow(dead_code)]
    ///
    /// fn this_is_fine() { }
    ///
    /// // or
    ///
    /// // Good (as outer attribute)
    /// #[allow(dead_code)]
    /// fn this_is_fine_too() { }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub EMPTY_LINE_AFTER_OUTER_ATTR,
    nursery,
    "empty line after outer attribute"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for empty lines after documentation comments.
    ///
    /// ### Why is this bad?
    /// The documentation comment was most likely meant to be an inner attribute or regular comment.
    /// If it was intended to be a documentation comment, then the empty line should be removed to
    /// be more idiomatic.
    ///
    /// ### Known problems
    /// Only detects empty lines immediately following the documentation. If the doc comment is followed
    /// by an attribute and then an empty line, this lint will not trigger. Use `empty_line_after_outer_attr`
    /// in combination with this lint to detect both cases.
    ///
    /// Does not detect empty lines after doc attributes (e.g. `#[doc = ""]`).
    ///
    /// ### Example
    /// ```no_run
    /// /// Some doc comment with a blank line after it.
    ///
    /// fn not_quite_good_code() { }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// /// Good (no blank line)
    /// fn this_is_fine() { }
    /// ```
    ///
    /// ```no_run
    /// // Good (convert to a regular comment)
    ///
    /// fn this_is_fine_too() { }
    /// ```
    ///
    /// ```no_run
    /// //! Good (convert to a comment on an inner attribute)
    ///
    /// fn this_is_fine_as_well() { }
    /// ```
    #[clippy::version = "1.70.0"]
    pub EMPTY_LINE_AFTER_DOC_COMMENTS,
    nursery,
    "empty line after documentation comments"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `warn`/`deny`/`forbid` attributes targeting the whole clippy::restriction category.
    ///
    /// ### Why is this bad?
    /// Restriction lints sometimes are in contrast with other lints or even go against idiomatic rust.
    /// These lints should only be enabled on a lint-by-lint basis and with careful consideration.
    ///
    /// ### Example
    /// ```no_run
    /// #![deny(clippy::restriction)]
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// #![deny(clippy::as_conversions)]
    /// ```
    #[clippy::version = "1.47.0"]
    pub BLANKET_CLIPPY_RESTRICTION_LINTS,
    suspicious,
    "enabling the complete restriction group"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `#[cfg_attr(rustfmt, rustfmt_skip)]` and suggests to replace it
    /// with `#[rustfmt::skip]`.
    ///
    /// ### Why is this bad?
    /// Since tool_attributes ([rust-lang/rust#44690](https://github.com/rust-lang/rust/issues/44690))
    /// are stable now, they should be used instead of the old `cfg_attr(rustfmt)` attributes.
    ///
    /// ### Known problems
    /// This lint doesn't detect crate level inner attributes, because they get
    /// processed before the PreExpansionPass lints get executed. See
    /// [#3123](https://github.com/rust-lang/rust-clippy/pull/3123#issuecomment-422321765)
    ///
    /// ### Example
    /// ```no_run
    /// #[cfg_attr(rustfmt, rustfmt_skip)]
    /// fn main() { }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// #[rustfmt::skip]
    /// fn main() { }
    /// ```
    #[clippy::version = "1.32.0"]
    pub DEPRECATED_CFG_ATTR,
    complexity,
    "usage of `cfg_attr(rustfmt)` instead of tool attributes"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for cfg attributes having operating systems used in target family position.
    ///
    /// ### Why is this bad?
    /// The configuration option will not be recognised and the related item will not be included
    /// by the conditional compilation engine.
    ///
    /// ### Example
    /// ```no_run
    /// #[cfg(linux)]
    /// fn conditional() { }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # mod hidden {
    /// #[cfg(target_os = "linux")]
    /// fn conditional() { }
    /// # }
    ///
    /// // or
    ///
    /// #[cfg(unix)]
    /// fn conditional() { }
    /// ```
    /// Check the [Rust Reference](https://doc.rust-lang.org/reference/conditional-compilation.html#target_os) for more details.
    #[clippy::version = "1.45.0"]
    pub MISMATCHED_TARGET_OS,
    correctness,
    "usage of `cfg(operating_system)` instead of `cfg(target_os = \"operating_system\")`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for attributes that allow lints without a reason.
    ///
    /// (This requires the `lint_reasons` feature)
    ///
    /// ### Why is this bad?
    /// Allowing a lint should always have a reason. This reason should be documented to
    /// ensure that others understand the reasoning
    ///
    /// ### Example
    /// ```no_run
    /// #![feature(lint_reasons)]
    ///
    /// #![allow(clippy::some_lint)]
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// #![feature(lint_reasons)]
    ///
    /// #![allow(clippy::some_lint, reason = "False positive rust-lang/rust-clippy#1002020")]
    /// ```
    #[clippy::version = "1.61.0"]
    pub ALLOW_ATTRIBUTES_WITHOUT_REASON,
    restriction,
    "ensures that all `allow` and `expect` attributes have a reason"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `#[should_panic]` attributes without specifying the expected panic message.
    ///
    /// ### Why is this bad?
    /// The expected panic message should be specified to ensure that the test is actually
    /// panicking with the expected message, and not another unrelated panic.
    ///
    /// ### Example
    /// ```no_run
    /// fn random() -> i32 { 0 }
    ///
    /// #[should_panic]
    /// #[test]
    /// fn my_test() {
    ///     let _ = 1 / random();
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn random() -> i32 { 0 }
    ///
    /// #[should_panic = "attempt to divide by zero"]
    /// #[test]
    /// fn my_test() {
    ///     let _ = 1 / random();
    /// }
    /// ```
    #[clippy::version = "1.74.0"]
    pub SHOULD_PANIC_WITHOUT_EXPECT,
    pedantic,
    "ensures that all `should_panic` attributes specify its expected panic message"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `any` and `all` combinators in `cfg` with only one condition.
    ///
    /// ### Why is this bad?
    /// If there is only one condition, no need to wrap it into `any` or `all` combinators.
    ///
    /// ### Example
    /// ```no_run
    /// #[cfg(any(unix))]
    /// pub struct Bar;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// #[cfg(unix)]
    /// pub struct Bar;
    /// ```
    #[clippy::version = "1.71.0"]
    pub NON_MINIMAL_CFG,
    style,
    "ensure that all `cfg(any())` and `cfg(all())` have more than one condition"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `#[cfg(features = "...")]` and suggests to replace it with
    /// `#[cfg(feature = "...")]`.
    ///
    /// It also checks if `cfg(test)` was misspelled.
    ///
    /// ### Why is this bad?
    /// Misspelling `feature` as `features` or `test` as `tests` can be sometimes hard to spot. It
    /// may cause conditional compilation not work quietly.
    ///
    /// ### Example
    /// ```no_run
    /// #[cfg(features = "some-feature")]
    /// fn conditional() { }
    /// #[cfg(tests)]
    /// mod tests { }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// #[cfg(feature = "some-feature")]
    /// fn conditional() { }
    /// #[cfg(test)]
    /// mod tests { }
    /// ```
    #[clippy::version = "1.69.0"]
    pub MAYBE_MISUSED_CFG,
    suspicious,
    "prevent from misusing the wrong attr name"
}

declare_lint_pass!(Attributes => [
    ALLOW_ATTRIBUTES_WITHOUT_REASON,
    INLINE_ALWAYS,
    DEPRECATED_SEMVER,
    USELESS_ATTRIBUTE,
    BLANKET_CLIPPY_RESTRICTION_LINTS,
    SHOULD_PANIC_WITHOUT_EXPECT,
]);

impl<'tcx> LateLintPass<'tcx> for Attributes {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        for (name, level) in &cx.sess().opts.lint_opts {
            if name == "clippy::restriction" && *level > Level::Allow {
                span_lint_and_then(
                    cx,
                    BLANKET_CLIPPY_RESTRICTION_LINTS,
                    DUMMY_SP,
                    "`clippy::restriction` is not meant to be enabled as a group",
                    |diag| {
                        diag.note(format!(
                            "because of the command line `--{} clippy::restriction`",
                            level.as_str()
                        ));
                        diag.help("enable the restriction lints you need individually");
                    },
                );
            }
        }
    }

    fn check_attribute(&mut self, cx: &LateContext<'tcx>, attr: &'tcx Attribute) {
        if let Some(items) = &attr.meta_item_list() {
            if let Some(ident) = attr.ident() {
                if is_lint_level(ident.name) {
                    check_clippy_lint_names(cx, ident.name, items);
                }
                if matches!(ident.name, sym::allow | sym::expect) {
                    check_lint_reason(cx, ident.name, items, attr);
                }
                if items.is_empty() || !attr.has_name(sym::deprecated) {
                    return;
                }
                for item in items {
                    if let NestedMetaItem::MetaItem(mi) = &item
                        && let MetaItemKind::NameValue(lit) = &mi.kind
                        && mi.has_name(sym::since)
                    {
                        check_deprecated_since(cx, item.span(), lit);
                    }
                }
            }
        }
        if attr.has_name(sym::should_panic) {
            check_should_panic_reason(cx, attr);
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        let attrs = cx.tcx.hir().attrs(item.hir_id());
        if is_relevant_item(cx, item) {
            check_attrs(cx, item.span, item.ident.name, attrs);
        }
        match item.kind {
            ItemKind::ExternCrate(..) | ItemKind::Use(..) => {
                let skip_unused_imports = attrs.iter().any(|attr| attr.has_name(sym::macro_use));

                for attr in attrs {
                    if in_external_macro(cx.sess(), attr.span) {
                        return;
                    }
                    if let Some(lint_list) = &attr.meta_item_list() {
                        if attr.ident().map_or(false, |ident| is_lint_level(ident.name)) {
                            for lint in lint_list {
                                match item.kind {
                                    ItemKind::Use(..) => {
                                        if is_word(lint, sym::unused_imports)
                                            || is_word(lint, sym::deprecated)
                                            || is_word(lint, sym!(unreachable_pub))
                                            || is_word(lint, sym!(unused))
                                            || extract_clippy_lint(lint).map_or(false, |s| {
                                                matches!(
                                                    s.as_str(),
                                                    "wildcard_imports"
                                                        | "enum_glob_use"
                                                        | "redundant_pub_crate"
                                                        | "macro_use_imports"
                                                        | "unsafe_removed_from_name"
                                                        | "module_name_repetitions"
                                                        | "single_component_path_imports"
                                                )
                                            })
                                        {
                                            return;
                                        }
                                    },
                                    ItemKind::ExternCrate(..) => {
                                        if is_word(lint, sym::unused_imports) && skip_unused_imports {
                                            return;
                                        }
                                        if is_word(lint, sym!(unused_extern_crates)) {
                                            return;
                                        }
                                    },
                                    _ => {},
                                }
                            }
                            let line_span = first_line_of_span(cx, attr.span);

                            if let Some(mut sugg) = snippet_opt(cx, line_span) {
                                if sugg.contains("#[") {
                                    span_lint_and_then(
                                        cx,
                                        USELESS_ATTRIBUTE,
                                        line_span,
                                        "useless lint attribute",
                                        |diag| {
                                            sugg = sugg.replacen("#[", "#![", 1);
                                            diag.span_suggestion(
                                                line_span,
                                                "if you just forgot a `!`, use",
                                                sugg,
                                                Applicability::MaybeIncorrect,
                                            );
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            },
            _ => {},
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'_>) {
        if is_relevant_impl(cx, item) {
            check_attrs(cx, item.span, item.ident.name, cx.tcx.hir().attrs(item.hir_id()));
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'_>) {
        if is_relevant_trait(cx, item) {
            check_attrs(cx, item.span, item.ident.name, cx.tcx.hir().attrs(item.hir_id()));
        }
    }
}

/// Returns the lint name if it is clippy lint.
fn extract_clippy_lint(lint: &NestedMetaItem) -> Option<Symbol> {
    if let Some(meta_item) = lint.meta_item()
        && meta_item.path.segments.len() > 1
        && let tool_name = meta_item.path.segments[0].ident
        && tool_name.name == sym::clippy
    {
        let lint_name = meta_item.path.segments.last().unwrap().ident.name;
        return Some(lint_name);
    }
    None
}

fn check_should_panic_reason(cx: &LateContext<'_>, attr: &Attribute) {
    if let AttrKind::Normal(normal_attr) = &attr.kind {
        if let AttrArgs::Eq(_, AttrArgsEq::Hir(_)) = &normal_attr.item.args {
            // `#[should_panic = ".."]` found, good
            return;
        }

        if let AttrArgs::Delimited(args) = &normal_attr.item.args
            && let mut tt_iter = args.tokens.trees()
            && let Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Ident(sym::expected, _),
                    ..
                },
                _,
            )) = tt_iter.next()
            && let Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Eq, ..
                },
                _,
            )) = tt_iter.next()
            && let Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Literal(_),
                    ..
                },
                _,
            )) = tt_iter.next()
        {
            // `#[should_panic(expected = "..")]` found, good
            return;
        }

        span_lint_and_sugg(
            cx,
            SHOULD_PANIC_WITHOUT_EXPECT,
            attr.span,
            "#[should_panic] attribute without a reason",
            "consider specifying the expected panic",
            "#[should_panic(expected = /* panic message */)]".into(),
            Applicability::HasPlaceholders,
        );
    }
}

fn check_clippy_lint_names(cx: &LateContext<'_>, name: Symbol, items: &[NestedMetaItem]) {
    for lint in items {
        if let Some(lint_name) = extract_clippy_lint(lint) {
            if lint_name.as_str() == "restriction" && name != sym::allow {
                span_lint_and_help(
                    cx,
                    BLANKET_CLIPPY_RESTRICTION_LINTS,
                    lint.span(),
                    "`clippy::restriction` is not meant to be enabled as a group",
                    None,
                    "enable the restriction lints you need individually",
                );
            }
        }
    }
}

fn check_lint_reason<'cx>(cx: &LateContext<'cx>, name: Symbol, items: &[NestedMetaItem], attr: &'cx Attribute) {
    // Check for the feature
    if !cx.tcx.features().lint_reasons {
        return;
    }

    // Check if the reason is present
    if let Some(item) = items.last().and_then(NestedMetaItem::meta_item)
        && let MetaItemKind::NameValue(_) = &item.kind
        && item.path == sym::reason
    {
        return;
    }

    // Check if the attribute is in an external macro and therefore out of the developer's control
    if in_external_macro(cx.sess(), attr.span) || is_from_proc_macro(cx, &attr) {
        return;
    }

    span_lint_and_help(
        cx,
        ALLOW_ATTRIBUTES_WITHOUT_REASON,
        attr.span,
        &format!("`{}` attribute without specifying a reason", name.as_str()),
        None,
        "try adding a reason at the end with `, reason = \"..\"`",
    );
}

fn is_relevant_item(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    if let ItemKind::Fn(_, _, eid) = item.kind {
        is_relevant_expr(cx, cx.tcx.typeck_body(eid), cx.tcx.hir().body(eid).value)
    } else {
        true
    }
}

fn is_relevant_impl(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    match item.kind {
        ImplItemKind::Fn(_, eid) => is_relevant_expr(cx, cx.tcx.typeck_body(eid), cx.tcx.hir().body(eid).value),
        _ => false,
    }
}

fn is_relevant_trait(cx: &LateContext<'_>, item: &TraitItem<'_>) -> bool {
    match item.kind {
        TraitItemKind::Fn(_, TraitFn::Required(_)) => true,
        TraitItemKind::Fn(_, TraitFn::Provided(eid)) => {
            is_relevant_expr(cx, cx.tcx.typeck_body(eid), cx.tcx.hir().body(eid).value)
        },
        _ => false,
    }
}

fn is_relevant_block(cx: &LateContext<'_>, typeck_results: &ty::TypeckResults<'_>, block: &Block<'_>) -> bool {
    block.stmts.first().map_or(
        block
            .expr
            .as_ref()
            .map_or(false, |e| is_relevant_expr(cx, typeck_results, e)),
        |stmt| match &stmt.kind {
            StmtKind::Local(_) => true,
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => is_relevant_expr(cx, typeck_results, expr),
            StmtKind::Item(_) => false,
        },
    )
}

fn is_relevant_expr(cx: &LateContext<'_>, typeck_results: &ty::TypeckResults<'_>, expr: &Expr<'_>) -> bool {
    if macro_backtrace(expr.span).last().map_or(false, |macro_call| {
        is_panic(cx, macro_call.def_id) || cx.tcx.item_name(macro_call.def_id) == sym::unreachable
    }) {
        return false;
    }
    match &expr.kind {
        ExprKind::Block(block, _) => is_relevant_block(cx, typeck_results, block),
        ExprKind::Ret(Some(e)) => is_relevant_expr(cx, typeck_results, e),
        ExprKind::Ret(None) | ExprKind::Break(_, None) => false,
        _ => true,
    }
}

fn check_attrs(cx: &LateContext<'_>, span: Span, name: Symbol, attrs: &[Attribute]) {
    if span.from_expansion() {
        return;
    }

    for attr in attrs {
        if let Some(values) = attr.meta_item_list() {
            if values.len() != 1 || !attr.has_name(sym::inline) {
                continue;
            }
            if is_word(&values[0], sym::always) {
                span_lint(
                    cx,
                    INLINE_ALWAYS,
                    attr.span,
                    &format!("you have declared `#[inline(always)]` on `{name}`. This is usually a bad idea"),
                );
            }
        }
    }
}

fn check_deprecated_since(cx: &LateContext<'_>, span: Span, lit: &MetaItemLit) {
    if let LitKind::Str(is, _) = lit.kind {
        if is.as_str() == "TBD" || Version::parse(is.as_str()).is_ok() {
            return;
        }
    }
    span_lint(
        cx,
        DEPRECATED_SEMVER,
        span,
        "the since field must contain a semver-compliant version",
    );
}

fn is_word(nmi: &NestedMetaItem, expected: Symbol) -> bool {
    if let NestedMetaItem::MetaItem(mi) = &nmi {
        mi.is_word() && mi.has_name(expected)
    } else {
        false
    }
}

pub struct EarlyAttributes {
    pub msrv: Msrv,
}

impl_lint_pass!(EarlyAttributes => [
    DEPRECATED_CFG_ATTR,
    MISMATCHED_TARGET_OS,
    EMPTY_LINE_AFTER_OUTER_ATTR,
    EMPTY_LINE_AFTER_DOC_COMMENTS,
    NON_MINIMAL_CFG,
    MAYBE_MISUSED_CFG,
]);

impl EarlyLintPass for EarlyAttributes {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &rustc_ast::Item) {
        check_empty_line_after_outer_attr(cx, item);
    }

    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        check_deprecated_cfg_attr(cx, attr, &self.msrv);
        check_mismatched_target_os(cx, attr);
        check_minimal_cfg_condition(cx, attr);
        check_misused_cfg(cx, attr);
    }

    extract_msrv_attr!(EarlyContext);
}

/// Check for empty lines after outer attributes.
///
/// Attributes and documentation comments are both considered outer attributes
/// by the AST. However, the average user likely considers them to be different.
/// Checking for empty lines after each of these attributes is split into two different
/// lints but can share the same logic.
fn check_empty_line_after_outer_attr(cx: &EarlyContext<'_>, item: &rustc_ast::Item) {
    let mut iter = item.attrs.iter().peekable();
    while let Some(attr) = iter.next() {
        if (matches!(attr.kind, AttrKind::Normal(..)) || matches!(attr.kind, AttrKind::DocComment(..)))
            && attr.style == AttrStyle::Outer
            && is_present_in_source(cx, attr.span)
        {
            let begin_of_attr_to_item = Span::new(attr.span.lo(), item.span.lo(), item.span.ctxt(), item.span.parent());
            let end_of_attr_to_next_attr_or_item = Span::new(
                attr.span.hi(),
                iter.peek().map_or(item.span.lo(), |next_attr| next_attr.span.lo()),
                item.span.ctxt(),
                item.span.parent(),
            );

            if let Some(snippet) = snippet_opt(cx, end_of_attr_to_next_attr_or_item) {
                let lines = snippet.split('\n').collect::<Vec<_>>();
                let lines = without_block_comments(lines);

                if lines.iter().filter(|l| l.trim().is_empty()).count() > 2 {
                    let (lint_msg, lint_type) = match attr.kind {
                        AttrKind::DocComment(..) => (
                            "found an empty line after a doc comment. \
                            Perhaps you need to use `//!` to make a comment on a module, remove the empty line, or make a regular comment with `//`?",
                            EMPTY_LINE_AFTER_DOC_COMMENTS,
                        ),
                        AttrKind::Normal(..) => (
                            "found an empty line after an outer attribute. \
                            Perhaps you forgot to add a `!` to make it an inner attribute?",
                            EMPTY_LINE_AFTER_OUTER_ATTR,
                        ),
                    };

                    span_lint(cx, lint_type, begin_of_attr_to_item, lint_msg);
                }
            }
        }
    }
}

fn check_deprecated_cfg_attr(cx: &EarlyContext<'_>, attr: &Attribute, msrv: &Msrv) {
    if msrv.meets(msrvs::TOOL_ATTRIBUTES)
        // check cfg_attr
        && attr.has_name(sym::cfg_attr)
        && let Some(items) = attr.meta_item_list()
        && items.len() == 2
        // check for `rustfmt`
        && let Some(feature_item) = items[0].meta_item()
        && feature_item.has_name(sym::rustfmt)
        // check for `rustfmt_skip` and `rustfmt::skip`
        && let Some(skip_item) = &items[1].meta_item()
        && (skip_item.has_name(sym!(rustfmt_skip))
            || skip_item
                .path
                .segments
                .last()
                .expect("empty path in attribute")
                .ident
                .name
                == sym::skip)
        // Only lint outer attributes, because custom inner attributes are unstable
        // Tracking issue: https://github.com/rust-lang/rust/issues/54726
        && attr.style == AttrStyle::Outer
    {
        span_lint_and_sugg(
            cx,
            DEPRECATED_CFG_ATTR,
            attr.span,
            "`cfg_attr` is deprecated for rustfmt and got replaced by tool attributes",
            "use",
            "#[rustfmt::skip]".to_string(),
            Applicability::MachineApplicable,
        );
    }
}

fn check_nested_cfg(cx: &EarlyContext<'_>, items: &[NestedMetaItem]) {
    for item in items {
        if let NestedMetaItem::MetaItem(meta) = item {
            if !meta.has_name(sym::any) && !meta.has_name(sym::all) {
                continue;
            }
            if let MetaItemKind::List(list) = &meta.kind {
                check_nested_cfg(cx, list);
                if list.len() == 1 {
                    span_lint_and_then(
                        cx,
                        NON_MINIMAL_CFG,
                        meta.span,
                        "unneeded sub `cfg` when there is only one condition",
                        |diag| {
                            if let Some(snippet) = snippet_opt(cx, list[0].span()) {
                                diag.span_suggestion(meta.span, "try", snippet, Applicability::MaybeIncorrect);
                            }
                        },
                    );
                } else if list.is_empty() && meta.has_name(sym::all) {
                    span_lint_and_then(
                        cx,
                        NON_MINIMAL_CFG,
                        meta.span,
                        "unneeded sub `cfg` when there is no condition",
                        |_| {},
                    );
                }
            }
        }
    }
}

fn check_nested_misused_cfg(cx: &EarlyContext<'_>, items: &[NestedMetaItem]) {
    for item in items {
        if let NestedMetaItem::MetaItem(meta) = item {
            if let Some(ident) = meta.ident()
                && ident.name.as_str() == "features"
                && let Some(val) = meta.value_str()
            {
                span_lint_and_sugg(
                    cx,
                    MAYBE_MISUSED_CFG,
                    meta.span,
                    "'feature' may be misspelled as 'features'",
                    "did you mean",
                    format!("feature = \"{val}\""),
                    Applicability::MaybeIncorrect,
                );
            }
            if let MetaItemKind::List(list) = &meta.kind {
                check_nested_misused_cfg(cx, list);
            // If this is not a list, then we check for `cfg(test)`.
            } else if let Some(ident) = meta.ident()
                && matches!(ident.name.as_str(), "tests" | "Test")
            {
                span_lint_and_sugg(
                    cx,
                    MAYBE_MISUSED_CFG,
                    meta.span,
                    &format!("'test' may be misspelled as '{}'", ident.name.as_str()),
                    "did you mean",
                    "test".to_string(),
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}

fn check_minimal_cfg_condition(cx: &EarlyContext<'_>, attr: &Attribute) {
    if attr.has_name(sym::cfg)
        && let Some(items) = attr.meta_item_list()
    {
        check_nested_cfg(cx, &items);
    }
}

fn check_misused_cfg(cx: &EarlyContext<'_>, attr: &Attribute) {
    if attr.has_name(sym::cfg)
        && let Some(items) = attr.meta_item_list()
    {
        check_nested_misused_cfg(cx, &items);
    }
}

fn check_mismatched_target_os(cx: &EarlyContext<'_>, attr: &Attribute) {
    fn find_os(name: &str) -> Option<&'static str> {
        UNIX_SYSTEMS
            .iter()
            .chain(NON_UNIX_SYSTEMS.iter())
            .find(|&&os| os == name)
            .copied()
    }

    fn is_unix(name: &str) -> bool {
        UNIX_SYSTEMS.iter().any(|&os| os == name)
    }

    fn find_mismatched_target_os(items: &[NestedMetaItem]) -> Vec<(&str, Span)> {
        let mut mismatched = Vec::new();

        for item in items {
            if let NestedMetaItem::MetaItem(meta) = item {
                match &meta.kind {
                    MetaItemKind::List(list) => {
                        mismatched.extend(find_mismatched_target_os(list));
                    },
                    MetaItemKind::Word => {
                        if let Some(ident) = meta.ident()
                            && let Some(os) = find_os(ident.name.as_str())
                        {
                            mismatched.push((os, ident.span));
                        }
                    },
                    MetaItemKind::NameValue(..) => {},
                }
            }
        }

        mismatched
    }

    if attr.has_name(sym::cfg)
        && let Some(list) = attr.meta_item_list()
        && let mismatched = find_mismatched_target_os(&list)
        && !mismatched.is_empty()
    {
        let mess = "operating system used in target family position";

        span_lint_and_then(cx, MISMATCHED_TARGET_OS, attr.span, mess, |diag| {
            // Avoid showing the unix suggestion multiple times in case
            // we have more than one mismatch for unix-like systems
            let mut unix_suggested = false;

            for (os, span) in mismatched {
                let sugg = format!("target_os = \"{os}\"");
                diag.span_suggestion(span, "try", sugg, Applicability::MaybeIncorrect);

                if !unix_suggested && is_unix(os) {
                    diag.help("did you mean `unix`?");
                    unix_suggested = true;
                }
            }
        });
    }
}

fn is_lint_level(symbol: Symbol) -> bool {
    matches!(symbol, sym::allow | sym::expect | sym::warn | sym::deny | sym::forbid)
}
