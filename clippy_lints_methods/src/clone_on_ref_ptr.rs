use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_context;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::symbol::{Symbol, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.clone()` on a ref-counted pointer,
    /// (`Rc`, `Arc`, `rc::Weak`, or `sync::Weak`), and suggests calling Clone via unified
    /// function syntax instead (e.g., `Rc::clone(foo)`).
    ///
    /// ### Why restrict this?
    /// Calling `.clone()` on an `Rc`, `Arc`, or `Weak`
    /// can obscure the fact that only the pointer is being cloned, not the underlying
    /// data.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::rc::Rc;
    /// let x = Rc::new(1);
    ///
    /// x.clone();
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # use std::rc::Rc;
    /// # let x = Rc::new(1);
    /// Rc::clone(&x);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub CLONE_ON_REF_PTR,
    restriction,
    "using `clone` on a ref-counted pointer"
}

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &hir::Expr<'_>,
    method_name: Symbol,
    receiver: &hir::Expr<'_>,
    args: &[hir::Expr<'_>],
) {
    if !(args.is_empty() && method_name == sym::clone) {
        return;
    }
    let obj_ty = cx.typeck_results().expr_ty(receiver).peel_refs();

    if let ty::Adt(adt, subst) = obj_ty.kind()
        && let Some(name) = cx.tcx.get_diagnostic_name(adt.did())
    {
        let caller_type = match name {
            sym::Rc => "Rc",
            sym::Arc => "Arc",
            sym::RcWeak | sym::ArcWeak => "Weak",
            _ => return,
        };
        span_lint_and_then(
            cx,
            CLONE_ON_REF_PTR,
            expr.span,
            "using `.clone()` on a ref-counted pointer",
            |diag| {
                // Sometimes unnecessary ::<_> after Rc/Arc/Weak
                let mut app = Applicability::Unspecified;
                let snippet = snippet_with_context(cx, receiver.span, expr.span.ctxt(), "..", &mut app).0;
                diag.span_suggestion(
                    expr.span,
                    "try",
                    format!("{caller_type}::<{}>::clone(&{snippet})", subst.type_at(0)),
                    app,
                );
            },
        );
    }
}
