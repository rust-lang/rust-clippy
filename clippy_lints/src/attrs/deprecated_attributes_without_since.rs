use super::{Attribute, DEPRECATED_ATTRIBUTES_WITHOUT_SINCE};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_ast::{MetaItemInner, MetaItemKind};
use rustc_errors::Applicability;
use rustc_lint::EarlyContext;
use rustc_span::sym;

pub(super) fn check<'cx>(cx: &EarlyContext<'cx>, items: Option<&[MetaItemInner]>, attr: &'cx Attribute) {
    if let Some(items) = items {
        for item_inner in items {
            if let Some(item) = MetaItemInner::meta_item(item_inner)
                && let MetaItemKind::NameValue(_) = &item.kind
                && item.path == sym::since
            {
                return;
            }
        }
    }

    let mut applicability = Applicability::HasPlaceholders;

    let suggestion = if let Some(eq_note) = attr.value_span() {
        let snippet = snippet_with_applicability(cx, eq_note, "/* note */", &mut applicability);
        format!("#[deprecated(note = {snippet}, since = /* version */)]")
    } else {
        items.map_or_else(
            || "#[deprecated(since = /* version */)]".to_owned(),
            |items| {
                let mut attr_with_fields = String::from("#[deprecated(");
                for item in items {
                    let snippet = snippet_with_applicability(cx, item.span(), "_", &mut applicability);
                    attr_with_fields.push_str(&snippet);
                    attr_with_fields.push_str(", ");
                }
                attr_with_fields.push_str("since = /* version */)]");
                attr_with_fields
            },
        )
    };

    span_lint_and_sugg(
        cx,
        DEPRECATED_ATTRIBUTES_WITHOUT_SINCE,
        attr.span,
        "`deprecated` attribute without specifying what version of the crate deprecated the item",
        "add a version",
        suggestion,
        applicability,
    );
}
