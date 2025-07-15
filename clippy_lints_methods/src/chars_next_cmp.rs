use clippy_utils::sym;
use rustc_lint::LateContext;

use super::CHARS_NEXT_CMP;

/// Checks for the `CHARS_NEXT_CMP` lint.
pub(super) fn check(cx: &LateContext<'_>, info: &crate::BinaryExprInfo<'_>) -> bool {
    crate::chars_cmp::check(cx, info, &[sym::chars, sym::next], CHARS_NEXT_CMP, "starts_with")
}
