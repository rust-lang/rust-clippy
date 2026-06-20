use super::{Attribute, BLANKET_ALLOW_WARNINGS};

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{is_from_proc_macro, sym};
use rustc_ast::MetaItemInner;
use rustc_lint::{EarlyContext, LintContext};
use rustc_span::symbol::Symbol;

pub(super) fn check<'cx>(cx: &EarlyContext<'cx>, name: Symbol, items: &[MetaItemInner], attr: &'cx Attribute) {
    for lint in items {
        // Check if the attribute is in an external macro and therefore out of the developer's control
        // TODO?: (taken from `allow_attributes_without_reason`; should this be an `attrs` util? )
        if attr.span.in_external_macro(cx.sess().source_map()) || is_from_proc_macro(cx, attr) {
            return;
        }

        if let Some(item) = lint.meta_item()
            && let Some(group) = item.name()
            && group == sym::warnings
        {
            span_lint_and_help(
                cx,
                BLANKET_ALLOW_WARNINGS,
                lint.span(),
                format!("`warnings` group in `{name}` attribute"),
                None,
                format!("`{name}` lints individually or in smaller groups"),
            );
        }
    }
}
