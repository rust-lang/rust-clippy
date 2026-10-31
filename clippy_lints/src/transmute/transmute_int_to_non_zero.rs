use super::TRANSMUTE_INT_TO_NON_ZERO;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sugg::Sugg;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::symbol::sym;

/// Checks for `transmute_int_to_non_zero` lint.
/// Returns `true` if it's triggered, otherwise returns `false`.
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    from_ty: Ty<'tcx>,
    to_ty: Ty<'tcx>,
    arg: &'tcx Expr<'_>,
) -> bool {
    if let ty::Int(_) | ty::Uint(_) = from_ty.kind()
        && let ty::Adt(adt, substs) = to_ty.kind()
        && cx.tcx.is_diagnostic_item(sym::NonZero, adt.did())
        && let int_ty = substs.type_at(0)
        && from_ty == int_ty
    {
        // Map the integer type to its concrete NonZero alias.
        // Without this, the suggestion would be `NonZero::new_unchecked`,
        // which leads to type‑inference failures and non‑const `.into()`
        // calls in const contexts.
        let coercible_types = [
            ("NonZeroU8", cx.tcx.types.u8),
            ("NonZeroU16", cx.tcx.types.u16),
            ("NonZeroU32", cx.tcx.types.u32),
            ("NonZeroU64", cx.tcx.types.u64),
            ("NonZeroU128", cx.tcx.types.u128),
            ("NonZeroUsize", cx.tcx.types.usize),
            ("NonZeroI8", cx.tcx.types.i8),
            ("NonZeroI16", cx.tcx.types.i16),
            ("NonZeroI32", cx.tcx.types.i32),
            ("NonZeroI64", cx.tcx.types.i64),
            ("NonZeroI128", cx.tcx.types.i128),
            ("NonZeroIsize", cx.tcx.types.isize),
        ];

        let Some(nonzero_alias) = coercible_types.iter().find_map(|(alias, ty)| {
            if *ty == int_ty && *ty == from_ty {
                Some(*alias)
            } else {
                None
            }
        }) else {
            return false;
        };

        let arg = Sugg::hir(cx, arg, "..");
        span_lint_and_sugg(
            cx,
            TRANSMUTE_INT_TO_NON_ZERO,
            e.span,
            format!("transmute from a `{from_ty}` to a `{nonzero_alias}`"),
            "consider using",
            format!("{nonzero_alias}::new_unchecked({arg})"),
            Applicability::Unspecified,
        );
        true
    } else {
        false
    }
}
