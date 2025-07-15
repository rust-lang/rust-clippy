use super::utils::derefs_to_slice;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::get_parent_expr;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.get().unwrap()` (or
    /// `.get_mut().unwrap`) on a standard library type which implements `Index`
    ///
    /// ### Why restrict this?
    /// Using the Index trait (`[]`) is more clear and more
    /// concise.
    ///
    /// ### Known problems
    /// Not a replacement for error handling: Using either
    /// `.unwrap()` or the Index trait (`[]`) carries the risk of causing a `panic`
    /// if the value being accessed is `None`. If the use of `.get().unwrap()` is a
    /// temporary placeholder for dealing with the `Option` type, then this does
    /// not mitigate the need for error handling. If there is a chance that `.get()`
    /// will be `None` in your program, then it is advisable that the `None` case
    /// is handled in a future refactor instead of using `.unwrap()` or the Index
    /// trait.
    ///
    /// ### Example
    /// ```no_run
    /// let mut some_vec = vec![0, 1, 2, 3];
    /// let last = some_vec.get(3).unwrap();
    /// *some_vec.get_mut(0).unwrap() = 1;
    /// ```
    /// The correct use would be:
    /// ```no_run
    /// let mut some_vec = vec![0, 1, 2, 3];
    /// let last = some_vec[3];
    /// some_vec[0] = 1;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub GET_UNWRAP,
    restriction,
    "using `.get().unwrap()` or `.get_mut().unwrap()` when using `[]` would work instead"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &hir::Expr<'_>,
    recv: &'tcx hir::Expr<'tcx>,
    get_arg: &'tcx hir::Expr<'_>,
    is_mut: bool,
) {
    // Note: we don't want to lint `get_mut().unwrap` for `HashMap` or `BTreeMap`,
    // because they do not implement `IndexMut`
    let expr_ty = cx.typeck_results().expr_ty(recv);
    let caller_type = if derefs_to_slice(cx, recv, expr_ty).is_some() {
        "slice"
    } else if is_type_diagnostic_item(cx, expr_ty, sym::Vec) {
        "Vec"
    } else if is_type_diagnostic_item(cx, expr_ty, sym::VecDeque) {
        "VecDeque"
    } else if !is_mut && is_type_diagnostic_item(cx, expr_ty, sym::HashMap) {
        "HashMap"
    } else if !is_mut && is_type_diagnostic_item(cx, expr_ty, sym::BTreeMap) {
        "BTreeMap"
    } else {
        return; // caller is not a type that we want to lint
    };

    let mut span = expr.span;

    // Handle the case where the result is immediately dereferenced,
    // either directly be the user, or as a result of a method call or the like
    // by not requiring an explicit reference
    let needs_ref = if let Some(parent) = get_parent_expr(cx, expr)
        && let hir::ExprKind::Unary(hir::UnOp::Deref, _)
        | hir::ExprKind::MethodCall(..)
        | hir::ExprKind::Field(..)
        | hir::ExprKind::Index(..) = parent.kind
    {
        if let hir::ExprKind::Unary(hir::UnOp::Deref, _) = parent.kind {
            // if the user explicitly dereferences the result, we can adjust
            // the span to also include the deref part
            span = parent.span;
        }
        false
    } else {
        true
    };

    let mut_str = if is_mut { "_mut" } else { "" };

    span_lint_and_then(
        cx,
        GET_UNWRAP,
        span,
        format!("called `.get{mut_str}().unwrap()` on a {caller_type}"),
        |diag| {
            let mut applicability = Applicability::MachineApplicable;
            let get_args_str = snippet_with_applicability(cx, get_arg.span, "..", &mut applicability);

            let borrow_str = if !needs_ref {
                ""
            } else if is_mut {
                "&mut "
            } else {
                "&"
            };

            diag.span_suggestion_verbose(
                span,
                "using `[]` is clearer and more concise",
                format!(
                    "{borrow_str}{}[{get_args_str}]",
                    snippet_with_applicability(cx, recv.span, "..", &mut applicability)
                ),
                applicability,
            );
        },
    );
}
