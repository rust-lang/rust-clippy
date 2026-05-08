use super::{Attribute, MISSING_NO_STD_ATTRIBUTE};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sym;
use rustc_ast::Crate;
use rustc_errors::Applicability;
use rustc_hir::AttrStyle;
use rustc_lint::EarlyContext;

pub fn check(cx: &EarlyContext<'_>, krate: &Crate) {
    if !krate
        .attrs
        .iter()
        .any(|attr| is_no_std_attribute(attr) || is_cfg_attr_no_std_attribute(attr))
    {
        span_lint_and_sugg(
            cx,
            MISSING_NO_STD_ATTRIBUTE,
            krate.spans.inject_use_span,
            "missing `#![no_std]` attribute",
            "use",
            "#![no_std]\nextern crate std;\n".to_string(),
            Applicability::MaybeIncorrect,
        );
    }
}

/// Checks if `attr` is of the form `#![no_std]`
fn is_no_std_attribute(attr: &Attribute) -> bool {
    attr.has_name(sym::no_std) && attr.style == AttrStyle::Inner
}

/// Checks if `attr` is of the form `#![cfg_attr(_, ..., no_std, ...)]`
fn is_cfg_attr_no_std_attribute(attr: &Attribute) -> bool {
    attr.has_any_name(&[sym::cfg_attr, sym::cfg_attr_trace])
        && attr.style == AttrStyle::Inner
        && attr
            .meta_item_list()
            .as_ref()
            .map(|items| items.iter())
            .into_iter()
            .flatten()
            .skip(1)
            .filter_map(|item| item.meta_item())
            .any(|item| item.has_name(sym::no_std))
}
