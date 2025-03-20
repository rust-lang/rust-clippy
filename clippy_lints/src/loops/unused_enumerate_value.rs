use super::UNUSED_ENUMERATE_VALUE;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::{get_adt_inherent_method, implements_trait};
use clippy_utils::{get_trait_def_id, pat_is_wild, paths};
use rustc_errors::Applicability;
use rustc_hir::def::DefKind;
use rustc_hir::{Expr, ExprKind, Pat, PatKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::sym;

/// Checks for the `UNUSED_ENUMERATE_VALUE` lint.
///
/// TODO: Extend this lint to cover iterator chains.
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, pat: &Pat<'tcx>, arg: &'tcx Expr<'_>, body: &'tcx Expr<'tcx>) {
    if let PatKind::Tuple([index, elem], _) = pat.kind
        && let ExprKind::MethodCall(_method, recv, [], _) = arg.kind
        && pat_is_wild(cx, &elem.kind, body)
        && let arg_ty = cx.typeck_results().expr_ty(arg)
        && let ty::Adt(base, _) = *arg_ty.kind()
        && cx.tcx.is_diagnostic_item(sym::Enumerate, base.did())
        && let Some((DefKind::AssocFn, call_id)) = cx.typeck_results().type_dependent_def(arg.hir_id)
        && cx.tcx.is_diagnostic_item(sym::enumerate_method, call_id)
        && let receiver_ty = cx.typeck_results().expr_ty(recv)
        // TODO: Replace with `sym` when it's available
        && let Some(exact_size_iter) = get_trait_def_id(cx.tcx, &paths::ITER_EXACT_SIZE_ITERATOR)
        && implements_trait(cx, receiver_ty, exact_size_iter, &[])
    {
        let (recv, applicability) = remove_trailing_iter(cx, recv);
        span_lint_and_then(
            cx,
            UNUSED_ENUMERATE_VALUE,
            arg.span,
            "you seem to use `.enumerate()` and immediately discard the value",
            |diag| {
                let range_end = Sugg::hir(cx, recv, "..");
                if applicability != Applicability::MachineApplicable {
                    diag.help(format!("consider using `0..{range_end}.len()` instead"));
                    return;
                }

                diag.multipart_suggestion(
                    format!("replace `{}` with `0..{range_end}.len()`", snippet(cx, arg.span, "..")),
                    vec![
                        (pat.span, snippet(cx, index.span, "..").into_owned()),
                        (arg.span, format!("0..{range_end}.len()")),
                    ],
                    applicability,
                );
            },
        );
    }
}

/// Removes trailing `.iter()`, `.iter_mut()`, or `.into_iter()` calls from the given expression if
/// `len` can be called directly on the receiver. Note that this may be incorrect if the receiver is
/// a user-defined type whose `len` method has a different meaning than the standard library.
fn remove_trailing_iter<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> (&'tcx Expr<'tcx>, Applicability) {
    let mut app = Applicability::MachineApplicable;
    if let ExprKind::MethodCall(iter_path, iter_recv, _, _) = expr.kind
        && matches!(iter_path.ident.name, sym::iter | sym::iter_mut | sym::into_iter)
        && let iter_recv_ty = cx.typeck_results().expr_ty(iter_recv).peel_refs()
        && let ty_is_builtin_container = is_builtin_container(cx, iter_recv_ty)
        && (ty_is_builtin_container || get_adt_inherent_method(cx, iter_recv_ty, sym::len).is_some())
    {
        if !ty_is_builtin_container {
            app = Applicability::MaybeIncorrect;
        }

        return (iter_recv, app);
    }

    (expr, app)
}

/// Return `true` if the given type is a built-in container type (array, slice, or collection).
fn is_builtin_container(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    ty.is_array()
        || ty.is_slice()
        || ty.is_array_slice()
        || (matches!(*ty.kind(), ty::Adt(iter_base, _) 
    if [sym::Vec, sym::VecDeque, sym::LinkedList, sym::BTreeMap, sym::BTreeSet, sym::HashMap, sym::HashSet, sym::BinaryHeap]
        .iter()
        .any(|sym| cx.tcx.is_diagnostic_item(*sym, iter_base.did()))))
}
