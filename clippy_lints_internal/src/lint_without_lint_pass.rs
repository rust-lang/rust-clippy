use crate::internal_paths;
use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use rustc_ast::ast::LitKind;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{ExprKind, Item, MutTy, Mutability, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::source_map::Spanned;
use rustc_span::sym;
use rustc_span::symbol::Symbol;

declare_tool_lint! {
    /// ### What it does
    /// Checks for cases of an auto-generated lint without an updated description,
    /// i.e. `default lint description`.
    ///
    /// ### Why is this bad?
    /// Indicates that the lint is not finished.
    ///
    /// ### Example
    /// ```rust,ignore
    /// declare_lint! { pub COOL_LINT, nursery, "default lint description" }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// declare_lint! { pub COOL_LINT, nursery, "a great new lint" }
    /// ```
    pub clippy::DEFAULT_LINT,
    Warn,
    "found 'default lint description' in a lint declaration",
    report_in_external_macro: true
}

declare_tool_lint! {
    /// ### What it does
    /// Checks for invalid `clippy::version` attributes.
    ///
    /// Valid values are:
    /// * "pre 1.29.0"
    /// * any valid semantic version
    pub clippy::INVALID_CLIPPY_VERSION_ATTRIBUTE,
    Warn,
    "found an invalid `clippy::version` attribute",
    report_in_external_macro: true
}

declare_tool_lint! {
    /// ### What it does
    /// Checks for declared clippy lints without the `clippy::version` attribute.
    pub clippy::MISSING_CLIPPY_VERSION_ATTRIBUTE,
    Warn,
    "found clippy lint without `clippy::version` attribute",
    report_in_external_macro: true
}

declare_lint_pass!(LintWithoutLintPass => [
    DEFAULT_LINT,
    INVALID_CLIPPY_VERSION_ATTRIBUTE,
    MISSING_CLIPPY_VERSION_ATTRIBUTE,
]);

impl<'tcx> LateLintPass<'tcx> for LintWithoutLintPass {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        if let hir::ItemKind::Static(Mutability::Not, ident, ty, body_id) = item.kind
            && is_lint_ref_type(cx, ty)
        {
            check_invalid_clippy_version_attribute(cx, item);

            let expr = &cx.tcx.hir_body(body_id).value;
            let fields = if let ExprKind::AddrOf(_, _, inner_exp) = expr.kind
                && let ExprKind::Struct(_, struct_fields, _) = inner_exp.kind
            {
                struct_fields
            } else {
                return;
            };

            let field = fields
                .iter()
                .find(|f| f.ident.as_str() == "desc")
                .expect("lints must have a description field");

            if let ExprKind::Lit(Spanned {
                node: LitKind::Str(sym, _),
                ..
            }) = field.expr.kind
            {
                let sym_str = sym.as_str();
                if sym_str == "default lint description" {
                    span_lint(
                        cx,
                        DEFAULT_LINT,
                        item.span,
                        format!("the lint `{}` has the default lint description", ident.name),
                    );
                }
            }
        }
    }
}

pub(super) fn is_lint_ref_type(cx: &LateContext<'_>, ty: &hir::Ty<'_>) -> bool {
    if let TyKind::Ref(
        _,
        MutTy {
            ty: inner,
            mutbl: Mutability::Not,
        },
    ) = ty.kind
        && let TyKind::Path(ref path) = inner.kind
        && let Res::Def(DefKind::Struct, def_id) = cx.qpath_res(path, inner.hir_id)
    {
        internal_paths::LINT.matches(cx, def_id)
    } else {
        false
    }
}

fn check_invalid_clippy_version_attribute(cx: &LateContext<'_>, item: &'_ Item<'_>) {
    if let Some(value) = extract_clippy_version_value(cx, item) {
        if value.as_str() == "pre 1.29.0" {
            return;
        }

        if rustc_attr_parsing::parse_version(value).is_none() {
            span_lint_and_help(
                cx,
                INVALID_CLIPPY_VERSION_ATTRIBUTE,
                item.span,
                "this item has an invalid `clippy::version` attribute",
                None,
                "please use a valid semantic version, see `doc/adding_lints.md`",
            );
        }
    } else {
        span_lint_and_help(
            cx,
            MISSING_CLIPPY_VERSION_ATTRIBUTE,
            item.span,
            "this lint is missing the `clippy::version` attribute or version value",
            None,
            "please use a `clippy::version` attribute, see `doc/adding_lints.md`",
        );
    }
}

/// This function extracts the version value of a `clippy::version` attribute if the given value has
/// one
pub(super) fn extract_clippy_version_value(cx: &LateContext<'_>, item: &'_ Item<'_>) -> Option<Symbol> {
    let attrs = cx.tcx.hir_attrs(item.hir_id());
    attrs.iter().find_map(|attr| {
        if let hir::Attribute::Unparsed(attr_kind) = &attr
            // Identify attribute
            && let [tool_name, attr_name] = &attr_kind.path.segments[..]
            && tool_name.name == sym::clippy
            && attr_name.name == sym::version
            && let Some(version) = attr.value_str()
        {
            Some(version)
        } else {
            None
        }
    })
}
