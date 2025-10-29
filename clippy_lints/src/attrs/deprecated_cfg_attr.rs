use super::{Attribute, DEPRECATED_CFG_ATTR, DEPRECATED_CLIPPY_CFG_ATTR};
use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_and_then, span_lint_hir_and_then};
use clippy_utils::msrvs::{self, MsrvStack};
use clippy_utils::sym;
use rustc_ast::AttrStyle;
use rustc_errors::Applicability;
use rustc_hir::attrs::CfgEntry;
use rustc_hir::def_id::{LOCAL_CRATE, LocalDefId};
use rustc_lint::{EarlyContext, LateContext};

pub(super) fn check_rustfmt(cx: &EarlyContext<'_>, attrs: &[Attribute], msrv: &MsrvStack) {
    for attr in attrs {
        check_rustfmt_attr(cx, attr, msrv);
    }
}

fn check_rustfmt_attr(cx: &EarlyContext<'_>, attr: &Attribute, msrv: &MsrvStack) {
    // check cfg_attr
    if attr.has_name(sym::cfg_attr_trace)
        && let Some(items) = attr.meta_item_list()
        && items.len() == 2
        && let Some(feature_item) = items[0].meta_item()
        // check for `rustfmt`
        && feature_item.has_name(sym::rustfmt)
        && msrv.meets(msrvs::TOOL_ATTRIBUTES)
        // check for `rustfmt_skip` and `rustfmt::skip`
        && let Some(skip_item) = &items[1].meta_item()
        && (skip_item.has_name(sym::rustfmt_skip)
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
        span_lint_and_then(
            cx,
            DEPRECATED_CFG_ATTR,
            attr.span,
            "`cfg_attr` is deprecated for rustfmt",
            |diag| {
                diag.span_suggestion_verbose(
                    attr.span,
                    "use the `rustfmt::skip` tool attribute instead",
                    "#[rustfmt::skip]",
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

pub(super) fn check(cx: &EarlyContext<'_>, attr: &rustc_ast::MetaItem) {
    if let Some(ident) = attr.ident() {
        if matches!(ident.name, sym::any | sym::all | sym::not) {
            let Some(list) = attr.meta_item_list() else { return };
            for item in list.iter().filter_map(|item| item.meta_item()) {
                check(cx, item);
            }
        } else {
            check_cargo_clippy_attr(cx, attr);
        }
    }
}

fn check_cargo_clippy_attr(cx: &EarlyContext<'_>, item: &rustc_ast::MetaItem) {
    if item.has_name(sym::feature) && item.value_str() == Some(sym::cargo_clippy) {
        span_lint_and_sugg(
            cx,
            DEPRECATED_CLIPPY_CFG_ATTR,
            item.span,
            "`feature = \"cargo-clippy\"` was replaced by `clippy`",
            "replace with",
            "clippy".to_string(),
            Applicability::MachineApplicable,
        );
    }
}

pub fn check_stripped(cx: &LateContext<'_>) {
    for stripped in cx.tcx.stripped_cfg_items(LOCAL_CRATE) {
        if let Some(parent_module) = stripped.parent_module.as_local() {
            check_cfg_entry(cx, &stripped.cfg.0, parent_module);
        }
    }
}

fn check_cfg_entry(cx: &LateContext<'_>, cfg: &CfgEntry, parent_module: LocalDefId) {
    match cfg {
        &CfgEntry::NameValue {
            name: sym::feature,
            name_span: _,
            value: Some((sym::cargo_clippy, _)),
            span,
        } => {
            span_lint_hir_and_then(
                cx,
                DEPRECATED_CLIPPY_CFG_ATTR,
                cx.tcx.local_def_id_to_hir_id(parent_module),
                span,
                "`feature = \"cargo-clippy\"` was replaced by `clippy`",
                |diag| {
                    diag.span_suggestion(span, "replace with", "clippy", Applicability::MachineApplicable);
                },
            );
        },
        CfgEntry::All(children, _) | CfgEntry::Any(children, _) => {
            for child in children {
                check_cfg_entry(cx, child, parent_module);
            }
        },
        CfgEntry::Not(child, _) => check_cfg_entry(cx, child, parent_module),
        _ => {},
    }
}
