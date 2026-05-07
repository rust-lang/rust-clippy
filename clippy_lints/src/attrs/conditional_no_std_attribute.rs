use crate::clippy_utils::source::SpanRangeExt;
use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::msrvs::{self, MsrvStack};
use clippy_utils::sym;
use rustc_ast::{AttrStyle, Attribute};
use rustc_errors::Applicability;
use rustc_lint::EarlyContext;

use super::CONDITIONAL_NO_STD_ATTRIBUTE;

/// Checks for an inner attribute of the form:
///
/// ```rust,ignore
/// #![cfg_attr(configuration, .., no_std, ..)]
/// ```
///
/// And suggests replacing it with:
///
/// ```rust,ignore
/// #![cfg_attr(configuration, ..)]
/// #![no_std]
///
/// #[cfg(not(configuration))]
/// extern crate std;
/// ```
pub(super) fn check(cx: &EarlyContext<'_>, attr: &Attribute, msrv: &MsrvStack) {
    if attr.has_name(sym::cfg_attr)
        && msrv.meets(msrvs::NO_STD_INNER_ATTRIBUTE)
        && attr.style == AttrStyle::Inner
        && let Some(items) = attr.meta_item_list()
        && let [configuration_item, items @ ..] = items.as_slice()
        && let Some(no_std_item) = items
            .iter()
            .filter_map(|item| item.meta_item())
            .find(|item| item.has_name(sym::no_std))
    {
        let side_effects = items.len() > 1;
        let cfg_txt = configuration_item.span().get_source_text(cx).map(|txt| txt.to_string());
        let other_items = items
            .iter()
            .filter(|item| item.meta_item().is_none_or(|item| !item.has_name(sym::no_std)))
            .map(|item| item.span().get_source_text(cx).map(|txt| txt.to_string()))
            .reduce(|prev, next| Some([prev?, next?].join(", ")))
            .flatten();

        let new_attr = match (cfg_txt.as_deref(), other_items) {
            (Some(cfg_txt), Some(other_items)) if side_effects => {
                Some(format!("#![cfg_attr({cfg_txt}, {other_items})]\n"))
            },
            _ if !side_effects => Some(String::new()),
            _ => None,
        };

        let message = format!(
            "'{}' for `{}` is not recommended as it changes the implicit prelude across the crate",
            sym::cfg_attr,
            sym::no_std
        );

        if let Some(cfg_txt) = cfg_txt.as_deref()
            && let Some(new_attr) = new_attr
        {
            span_lint_and_sugg(
                cx,
                CONDITIONAL_NO_STD_ATTRIBUTE,
                attr.span,
                message,
                "use",
                format!("{new_attr}#![no_std]\n#[cfg(not({cfg_txt}))]\nextern crate std;"),
                Applicability::MaybeIncorrect,
            );
        } else {
            span_lint_and_help(
                cx,
                CONDITIONAL_NO_STD_ATTRIBUTE,
                no_std_item.span,
                message,
                None,
                format!(
                    "use `#![no_std]` and `#[cfg(not({}))] extern crate std;`",
                    cfg_txt.unwrap_or("...".to_string()),
                ),
            );
        }
    }
}
