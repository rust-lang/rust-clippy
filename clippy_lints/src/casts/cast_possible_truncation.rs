use clippy_utils::diagnostics::{span_lint, span_lint_and_then};
use clippy_utils::source::snippet;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::{get_discriminant_value, is_isize_or_usize};
use clippy_utils::{is_in_const_context, rinterval};
use rustc_abi::IntegerType;
use rustc_errors::{Applicability, Diag};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, FloatTy, Ty};
use rustc_span::Span;

use super::{CAST_ENUM_TRUNCATION, CAST_POSSIBLE_TRUNCATION, utils};

pub(super) fn check<'cx>(
    cx: &LateContext<'cx>,
    expr: &Expr<'_>,
    cast_expr: &Expr<'cx>,
    cast_from: Ty<'_>,
    cast_to: Ty<'_>,
    cast_to_span: Span,
) {
    let mut from_interval = None;

    let from_is_size = is_isize_or_usize(cast_from);
    let to_is_size = is_isize_or_usize(cast_to);

    let msg = match (cast_from.kind(), utils::int_ty_to_nbits(cx.tcx, cast_to)) {
        (ty::Int(_) | ty::Uint(_), Some(to_nbits)) => {
            let interval_ctx = rinterval::IntervalCtxt::new(cx);
            from_interval = interval_ctx.eval(cast_expr);

            let to_ty = if !from_is_size && to_is_size {
                // if we cast from a fixed-size integer to a pointer-sized integer,
                // we want assume the worst case of usize being 32-bit
                if cast_to.is_signed() {
                    rinterval::IntType::I32
                } else {
                    rinterval::IntType::U32
                }
            } else {
                interval_ctx
                    .to_int_type(cast_to)
                    .expect("the to cast type should be an integral type")
            };

            if let Some(from_interval) = &from_interval
                && from_interval.fits_into(to_ty)
            {
                // No truncation possible.
                return;
            }

            let from_nbits = utils::int_ty_to_nbits(cx.tcx, cast_from).unwrap();

            let (should_lint, suffix) = match (from_is_size, to_is_size) {
                (true, true) | (false, false) => (to_nbits < from_nbits, ""),
                (true, false) => (
                    to_nbits <= 32,
                    if to_nbits == 32 {
                        " on targets with 64-bit wide pointers"
                    } else {
                        ""
                    },
                ),
                (false, true) => (from_nbits == 64, " on targets with 32-bit wide pointers"),
            };

            if !should_lint {
                return;
            }

            format!("casting `{cast_from}` to `{cast_to}` may truncate the value{suffix}",)
        },

        (ty::Adt(def, _), Some(to_nbits)) if def.is_enum() => {
            let (from_nbits, variant) = if let ExprKind::Path(p) = &cast_expr.kind
                && let Res::Def(DefKind::Ctor(..), id) = cx.qpath_res(p, cast_expr.hir_id)
            {
                let i = def.variant_index_with_ctor_id(id);
                let variant = def.variant(i);
                let nbits = utils::enum_value_nbits(get_discriminant_value(cx.tcx, *def, i));
                (nbits, Some(variant))
            } else {
                (utils::enum_ty_to_nbits(*def, cx.tcx), None)
            };

            let cast_from_ptr_size = def.repr().int.is_none_or(|ty| matches!(ty, IntegerType::Pointer(_),));
            let suffix = match (cast_from_ptr_size, to_is_size) {
                (_, false) if from_nbits > to_nbits => "",
                (false, true) if from_nbits > 64 => "",
                (false, true) if from_nbits > 32 => " on targets with 32-bit wide pointers",
                _ => return,
            };

            if let Some(variant) = variant {
                span_lint(
                    cx,
                    CAST_ENUM_TRUNCATION,
                    expr.span,
                    format!(
                        "casting `{cast_from}::{}` to `{cast_to}` will truncate the value{suffix}",
                        variant.name,
                    ),
                );
                return;
            }
            format!("casting `{cast_from}` to `{cast_to}` may truncate the value{suffix}")
        },

        (ty::Float(_), Some(_)) => {
            format!("casting `{cast_from}` to `{cast_to}` may truncate the value")
        },

        (ty::Float(FloatTy::F64), None) if matches!(cast_to.kind(), &ty::Float(FloatTy::F32)) => {
            "casting `f64` to `f32` may truncate the value".to_string()
        },

        _ => return,
    };

    span_lint_and_then(cx, CAST_POSSIBLE_TRUNCATION, expr.span, msg, |diag| {
        if let Some(from_interval) = from_interval {
            diag.note(utils::format_cast_operand(from_interval));
        }

        diag.help("if this is intentional allow the lint with `#[allow(clippy::cast_possible_truncation)]` ...");
        // TODO: Remove the condition for const contexts when `try_from` and other commonly used methods
        // become const fn.
        if !is_in_const_context(cx) && !cast_from.is_floating_point() {
            offer_suggestion(cx, expr, cast_expr, cast_to_span, diag);
        }
    });
}

fn offer_suggestion(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    cast_expr: &Expr<'_>,
    cast_to_span: Span,
    diag: &mut Diag<'_, ()>,
) {
    let cast_to_snip = snippet(cx, cast_to_span, "..");
    let suggestion = if cast_to_snip == "_" {
        format!("{}.try_into()", Sugg::hir(cx, cast_expr, "..").maybe_paren())
    } else {
        format!("{cast_to_snip}::try_from({})", Sugg::hir(cx, cast_expr, ".."))
    };

    diag.span_suggestion_verbose(
        expr.span,
        "... or use `try_from` and handle the error accordingly",
        suggestion,
        Applicability::Unspecified,
    );
}
