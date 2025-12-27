use super::NON_MINIMAL_CFG;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, MsrvStack};
use clippy_utils::source::SpanRangeExt;
use rustc_ast::{MetaItemInner, MetaItemKind};
use rustc_errors::Applicability;
use rustc_lint::EarlyContext;
use rustc_span::sym;

pub(super) fn check(cx: &EarlyContext<'_>, items: &[MetaItemInner], msrv: &MsrvStack) {
    for item in items {
        if let MetaItemInner::MetaItem(meta) = item
            && let Some(name @ (sym::any | sym::all)) = meta.name()
            && !meta.span.from_expansion()
            && let MetaItemKind::List(list) = &meta.kind
        {
            check(cx, list, msrv);
            if list.len() == 1 {
                span_lint_and_then(
                    cx,
                    NON_MINIMAL_CFG,
                    meta.span,
                    "unneeded sub `cfg` when there is only one condition",
                    |diag| {
                        if let Some(snippet) = list[0].span().get_source_text(cx) {
                            diag.span_suggestion(
                                meta.span,
                                "try",
                                snippet.to_owned(),
                                Applicability::MachineApplicable,
                            );
                        }
                    },
                );
            } else if list.is_empty() && msrv.meets(msrvs::CFG_BOOLEAN_LITERALS) {
                span_lint_and_then(
                    cx,
                    NON_MINIMAL_CFG,
                    meta.span,
                    "unneeded sub `cfg` when there is no condition",
                    |diag| {
                        diag.span_suggestion_verbose(
                            meta.span,
                            "use a boolean literal instead",
                            match name {
                                sym::all => "true",
                                sym::any => "false",
                                _ => return,
                            },
                            Applicability::MachineApplicable,
                        );
                    },
                );
            }
        }
    }
}
