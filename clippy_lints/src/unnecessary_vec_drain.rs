use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::{is_type_diagnostic_item, is_type_lang_item};
use rustc_ast::LitKind::Int;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::{LangItem, Stmt};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `vec.drain(..)` with RangeFull that is not bound to a value `let`.
    ///
    /// ### Why is this bad?
    /// This creates an iterator that is dropped immediately.
    ///
    /// `vec.clear()` also shows clearer intention.
    ///
    /// ### Example
    /// ```rust
    /// let mut vec: Vec<i32> = Vec::new();
    //  vec.drain(..);
    /// ```
    /// Use instead:
    /// ```rust
    /// let mut vec: Vec<i32> = Vec::new();
    //  vec.clear();
    /// ```
    #[clippy::version = "1.66.0"]
    pub UNNECESSARY_VEC_DRAIN,
    style,
    "unnecessary `vec.drain(..)`"
}
declare_lint_pass!(UnnecessaryVecDrain => [UNNECESSARY_VEC_DRAIN]);

impl LateLintPass<'_> for UnnecessaryVecDrain {
    fn check_stmt<'tcx>(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if let hir::StmtKind::Semi(semi_expr) = &stmt.kind {
            if let hir::ExprKind::MethodCall(path, rcvr, method_args,drain_span) = &semi_expr.kind
                && path.ident.name == sym!(drain)
            {
                let ty = cx.typeck_results().expr_ty(rcvr);
                if let [expr_element] = &**method_args
                    && is_type_diagnostic_item(cx, ty, sym::Vec)
                {
                    let ty = cx.typeck_results().expr_ty(expr_element);
                    if is_type_lang_item(cx, ty, LangItem::RangeFull)
                    {
                        let mut applicability = Applicability::MachineApplicable;
                        span_lint_and_sugg(
                            cx,
                            UNNECESSARY_VEC_DRAIN,
                            semi_expr.span,
                            "unnecessary iterator `Drain` is created and dropped immedietly",
                            "consider calling `clear()`",
                            format!("{}.clear()", snippet_with_applicability(cx, rcvr.span, "", &mut applicability)),
                            applicability
                        );
                    }
                    if let hir::ExprKind::Struct(_, expr_fields, _) = &expr_element.kind
                        && is_type_lang_item(cx, ty, LangItem::Range)
                    {
                        if_chain! {
                            if let hir::ExprKind::Lit(lit) = &expr_fields[0].expr.kind;
                            if let hir::ExprKind::MethodCall(path_seg, vec_expr, _, _) = expr_fields[1].expr.kind;

                            if let hir::hir_id::HirId{owner: owner_expr,..} = &vec_expr.hir_id;
                            if let hir::hir_id::HirId{owner: owner_rcvr,..} = &rcvr.hir_id;

                            then {
                                if let Int(start,_) = lit.node
                                    && path_seg.ident.name == sym!(len)
                                    && owner_rcvr == owner_expr
                                    && start == 0
                                {
                                    let mut applicability = Applicability::MachineApplicable;
                                    span_lint_and_sugg(
                                        cx,
                                        UNNECESSARY_VEC_DRAIN,
                                        *drain_span,
                                        "unnecessary iterator `Drain` is created and dropped immediately",
                                        "consider calling `clear()`",
                                        format!("{}.clear()", snippet_with_applicability(cx, rcvr.span, "", &mut applicability)),
                                        applicability
                                    );
                                }
                           }
                        }
                    }
                }
            }
        }
    }
}
