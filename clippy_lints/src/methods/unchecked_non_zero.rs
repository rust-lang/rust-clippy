use clippy_utils::consts::{ConstEvalCtxt, FullInt};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::{is_from_proc_macro, sym};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{Span, Symbol};

use super::UNCHECKED_NON_ZERO;

/// A value that makes a call panic when it is below `min`.
struct Precondition<'tcx> {
    /// The expression producing the value.
    value: &'tcx Expr<'tcx>,
    /// Names the value, e.g. `"the chunk size"`.
    what: &'static str,
    /// Describes a violating value, e.g. `"zero"`.
    bad: &'static str,
    min: u32,
    help: String,
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    recv: &'tcx Expr<'tcx>,
    args: &'tcx [Expr<'tcx>],
    call_span: Span,
    method_name: Symbol,
) {
    let non_zero_arg = |value, what| Precondition {
        value,
        what,
        bad: "zero",
        min: 1,
        help: format!("consider taking {what} as a `NonZero<usize>`, or checking it before this call"),
    };

    // This runs for every method call in the crate, so match on the name before looking anything
    // up. `recv_ty` is only needed once an arm has matched.
    match (method_name, args) {
        // `chunk size must be non-zero` / `window size must be non-zero`. These names are also used
        // by iterator adapters and third-party traits, so check that this is the inherent slice
        // method. Autoderef means `Vec`, arrays and `Box<[T]>` all land here too.
        (
            sym::chunks
            | sym::chunks_mut
            | sym::chunks_exact
            | sym::chunks_exact_mut
            | sym::rchunks
            | sym::rchunks_mut
            | sym::rchunks_exact
            | sym::rchunks_exact_mut
            | sym::windows,
            [arg],
        ) => {
            let recv_ty = cx.typeck_results().expr_ty_adjusted(recv);
            if expr.span.from_expansion() || !matches!(recv_ty.kind(), ty::Ref(_, inner, _) if inner.is_slice()) {
                return;
            }
            let what = if method_name == sym::windows {
                "the window size"
            } else {
                "the chunk size"
            };
            emit(cx, expr, call_span, method_name, &non_zero_arg(arg, what), true);
        },

        // `Iterator::step_by` asserts `step != 0`.
        (sym::step_by, [arg]) => {
            if expr.span.from_expansion() || !cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::Iterator) {
                return;
            }
            // A literal `0` is already covered by `iterator_step_by_zero`, which is warn-by-default.
            emit(cx, expr, call_span, method_name, &non_zero_arg(arg, "the step"), false);
        },

        // `ilog2`/`ilog10`/`ilog` panic on a receiver of zero, or, when signed, on a negative one.
        // `NonZero` has its own infallible `ilog2`/`ilog10`, and is an ADT rather than `is_integral`.
        (sym::ilog | sym::ilog2 | sym::ilog10, _) => {
            let recv_ty = cx.typeck_results().expr_ty_adjusted(recv);
            if expr.span.from_expansion() || !recv_ty.is_integral() {
                return;
            }
            let checked = format!("consider using `checked_{method_name}`, which returns `None` instead of panicking");
            let receiver_is_valid = Precondition {
                value: recv,
                what: "the value",
                bad: if recv_ty.is_signed() {
                    "zero or negative"
                } else {
                    "zero"
                },
                min: 1,
                help: checked.clone(),
            };
            if emit(cx, expr, call_span, method_name, &receiver_is_valid, true) {
                return;
            }

            // `ilog` additionally panics when the base is less than 2.
            if let [base] = args {
                let base_is_valid = Precondition {
                    value: base,
                    what: "the base",
                    bad: "less than `2`",
                    min: 2,
                    help: checked,
                };
                emit(cx, expr, call_span, method_name, &base_is_valid, true);
            }
        },

        _ => {},
    }
}

/// Emits the lint when `precondition` is violated or unproven. Returns whether it was emitted.
///
/// `report_known_violations` reports values proven to panic; pass `false` where another lint
/// already owns that case.
fn emit<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    call_span: Span,
    method_name: Symbol,
    precondition: &Precondition<'tcx>,
    report_known_violations: bool,
) -> bool {
    let &Precondition {
        value,
        what,
        bad,
        min,
        ref help,
    } = precondition;

    let holds = known_at_least(cx, value, min);
    match holds {
        Some(true) => return false,
        Some(false) if !report_known_violations => return false,
        _ => {},
    }

    if is_from_proc_macro(cx, expr) {
        return false;
    }

    let msg = if holds == Some(false) {
        format!("`{method_name}` will panic, as {what} is {bad}")
    } else {
        format!("`{method_name}` will panic if {what} is {bad}")
    };

    span_lint_and_then(cx, UNCHECKED_NON_ZERO, call_span, msg, |diag| {
        if holds.is_none() {
            diag.span_note(value.span, format!("this may be {bad}"));
            diag.help(help.clone());
        }
    });
    true
}

/// Whether `e` is known to evaluate to at least `min`.
///
/// `Some(true)` when proven, `Some(false)` when proven violated, `None` when either is possible.
fn known_at_least<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>, min: u32) -> Option<bool> {
    if let Some(int) = ConstEvalCtxt::new(cx)
        .eval(e)
        .and_then(|c| c.int_value(cx.tcx, cx.typeck_results().expr_ty(e)))
    {
        return Some(match int {
            FullInt::S(v) => v >= i128::from(min),
            FullInt::U(v) => v >= u128::from(min),
        });
    }

    match e.kind {
        // `n.get()` on an unsigned `NonZero` is at least 1. A signed one may still be negative.
        ExprKind::MethodCall(name, recv, [], _) if name.ident.name == sym::get && min <= 1 => {
            let ty = cx.typeck_results().expr_ty_adjusted(recv).peel_refs();
            (ty.is_diag_item(cx, sym::NonZero) && matches!(ty.kind(), ty::Adt(_, args) if !args.type_at(0).is_signed()))
                .then_some(true)
        },
        // `n.max(k)` is at least `k`, the usual way of guarding these calls by hand.
        ExprKind::MethodCall(name, _, [other], _) if name.ident.name == sym::max => {
            (known_at_least(cx, other, min) == Some(true)).then_some(true)
        },
        _ => None,
    }
}
