use rustc_errors::Applicability;
use rustc_hir::{BorrowKind, Expr, ExprKind, LangItem};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, ClauseKind, ImplPolarity, Mutability, Ty};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_diag_trait_item;
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::{is_type_diagnostic_item, is_type_lang_item};

declare_clippy_lint! {
    /// ### What it does
    /// Checks if an unneeded allocation is performed when trying to get information
    /// related to a given key is a `HashMap`-like type.
    ///
    /// ### Why is this bad?
    /// Using less resources is generally a good idea.
    ///
    /// ### Example
    /// ```no_run
    /// let mut s = HashSet::from(["a".to_string()]);
    /// s.remove(&"b".to_owned());
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut s = HashSet::from(["a".to_string()]);
    /// s.remove("b");
    /// ```
    #[clippy::version = "1.78.0"]
    pub USELESS_ALLOCATION,
    suspicious,
    "default lint description"
}

declare_lint_pass!(UselessAllocation => [USELESS_ALLOCATION]);

fn is_a_std_map_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    is_type_diagnostic_item(cx, ty, sym::HashSet)
        || is_type_diagnostic_item(cx, ty, sym::HashMap)
        || is_type_diagnostic_item(cx, ty, sym::BTreeMap)
        || is_type_diagnostic_item(cx, ty, sym::BTreeSet)
}

fn is_str_and_string(cx: &LateContext<'_>, arg_ty: Ty<'_>, original_arg_ty: Ty<'_>) -> bool {
    original_arg_ty.is_str() && is_type_lang_item(cx, arg_ty, LangItem::String)
}

fn is_slice_and_vec(cx: &LateContext<'_>, arg_ty: Ty<'_>, original_arg_ty: Ty<'_>) -> bool {
    (original_arg_ty.is_slice() || original_arg_ty.is_array() || original_arg_ty.is_array_slice())
        && is_type_diagnostic_item(cx, arg_ty, sym::Vec)
}

fn check_if_applicable_to_argument(cx: &LateContext<'_>, arg: &Expr<'_>) {
    if let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, expr) = arg.kind
        && let ExprKind::MethodCall(method_path, caller, &[], _) = expr.kind
        && let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && let method_name = method_path.ident.name.as_str()
        && match method_name {
            "to_owned" => is_diag_trait_item(cx, method_def_id, sym::ToOwned),
            "to_string" => is_diag_trait_item(cx, method_def_id, sym::ToString),
            "to_vec" => cx
                .tcx
                .impl_of_method(method_def_id)
                .filter(|&impl_did| {
                    cx.tcx.type_of(impl_did).instantiate_identity().is_slice()
                        && cx.tcx.impl_trait_ref(impl_did).is_none()
                })
                .is_some(),
            _ => false,
        }
        && let original_arg_ty = cx.typeck_results().node_type(caller.hir_id)
        && let arg_ty = cx.typeck_results().expr_ty(arg)
        && let ty::Ref(_, arg_ty, Mutability::Not) = arg_ty.kind()
        && let arg_ty = arg_ty.peel_refs()
        // For now we limit this lint to `String` and `Vec`.
        && let is_str = is_str_and_string(cx, arg_ty, original_arg_ty.peel_refs())
        && (is_str || is_slice_and_vec(cx, arg_ty, original_arg_ty))
        && let Some(snippet) = snippet_opt(cx, caller.span)
    {
        span_lint_and_sugg(
            cx,
            USELESS_ALLOCATION,
            arg.span,
            "unneeded allocation",
            "replace it with",
            if is_str {
                snippet
            } else {
                format!("{}.as_slice()", snippet)
            },
            Applicability::MaybeIncorrect,
        );
    }
}

impl<'tcx> LateLintPass<'tcx> for UselessAllocation {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(_, caller, &[arg], _) = expr.kind
            && let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && let Some(borrow_id) = cx.tcx.get_diagnostic_item(sym::Borrow)
            && cx.tcx.predicates_of(method_def_id).predicates.iter().any(|(pred, _)| {
                if let ClauseKind::Trait(trait_pred) = pred.kind().skip_binder()
                    && trait_pred.polarity == ImplPolarity::Positive
                    && trait_pred.trait_ref.def_id == borrow_id
                {
                    true
                } else {
                    false
                }
            })
            && let caller_ty = cx.typeck_results().expr_ty(caller)
            && is_a_std_map_type(cx, caller_ty)
        {
            check_if_applicable_to_argument(cx, &arg);
        }
    }
}
