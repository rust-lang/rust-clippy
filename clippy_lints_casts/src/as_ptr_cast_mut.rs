use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::SpanRangeExt;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::mir::Mutability;
use rustc_middle::ty::{self, Ty};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the result of a `&self`-taking `as_ptr` being cast to a mutable pointer.
    ///
    /// ### Why is this bad?
    /// Since `as_ptr` takes a `&self`, the pointer won't have write permissions unless interior
    /// mutability is used, making it unlikely that having it as a mutable pointer is correct.
    ///
    /// ### Example
    /// ```no_run
    /// let mut vec = Vec::<u8>::with_capacity(1);
    /// let ptr = vec.as_ptr() as *mut u8;
    /// unsafe { ptr.write(4) }; // UNDEFINED BEHAVIOUR
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut vec = Vec::<u8>::with_capacity(1);
    /// let ptr = vec.as_mut_ptr();
    /// unsafe { ptr.write(4) };
    /// ```
    #[clippy::version = "1.66.0"]
    pub AS_PTR_CAST_MUT,
    nursery,
    "casting the result of the `&self`-taking `as_ptr` to a mutable pointer"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, cast_expr: &Expr<'_>, cast_to: Ty<'_>) {
    if let ty::RawPtr(ptrty, Mutability::Mut) = cast_to.kind()
        && let ty::RawPtr(_, Mutability::Not) = cx.typeck_results().node_type(cast_expr.hir_id).kind()
        && let ExprKind::MethodCall(method_name, receiver, [], _) = cast_expr.peel_blocks().kind
        && method_name.ident.name == rustc_span::sym::as_ptr
        && let Some(as_ptr_did) = cx
            .typeck_results()
            .type_dependent_def_id(cast_expr.peel_blocks().hir_id)
        && let as_ptr_sig = cx.tcx.fn_sig(as_ptr_did).instantiate_identity()
        && let Some(first_param_ty) = as_ptr_sig.skip_binder().inputs().iter().next()
        && let ty::Ref(_, _, Mutability::Not) = first_param_ty.kind()
        && let Some(recv) = receiver.span.get_source_text(cx)
    {
        // `as_mut_ptr` might not exist
        let applicability = Applicability::MaybeIncorrect;

        span_lint_and_sugg(
            cx,
            AS_PTR_CAST_MUT,
            expr.span,
            format!("casting the result of `as_ptr` to *mut {ptrty}"),
            "replace with",
            format!("{recv}.as_mut_ptr()"),
            applicability,
        );
    }
}
