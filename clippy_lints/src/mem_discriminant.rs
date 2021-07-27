use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use clippy_utils::ty::walk_ptrs_ty_depth;
use clippy_utils::{match_def_path, paths};
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{BorrowKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls of `mem::discriminant()` on a non-enum type.
    ///
    /// ### Why is this bad?
    /// The value of `mem::discriminant()` on non-enum types
    /// is unspecified.
    ///
    /// ### Example
    /// ```rust
    /// use std::mem;
    ///
    /// mem::discriminant(&"hello");
    /// mem::discriminant(&&Some(2));
    /// ```
    pub MEM_DISCRIMINANT_NON_ENUM,
    correctness,
    "calling `mem::descriminant` on non-enum type"
}

declare_lint_pass!(MemDiscriminant => [MEM_DISCRIMINANT_NON_ENUM]);

impl<'tcx> LateLintPass<'tcx> for MemDiscriminant {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            if let ExprKind::Call(func, func_args) = expr.kind;
            // is `mem::discriminant`
            if let ExprKind::Path(ref func_qpath) = func.kind;
            if let Some(def_id) = cx.qpath_res(func_qpath, func.hir_id).opt_def_id();
            if match_def_path(cx, def_id, &paths::MEM_DISCRIMINANT);
            // type is non-enum
            let ty_param = cx.typeck_results().node_substs(func.hir_id).type_at(0);
            if !ty_param.is_enum();

            then {
                span_lint_and_then(
                    cx,
                    MEM_DISCRIMINANT_NON_ENUM,
                    expr.span,
                    &format!("calling `mem::discriminant` on non-enum type `{}`", ty_param),
                    |diag| {
                        // if this is a reference to an enum, suggest dereferencing
                        let (base_ty, ptr_depth) = walk_ptrs_ty_depth(ty_param);
                        if ptr_depth >= 1 && base_ty.is_enum() {
                            let param = &func_args[0];

                            // cancel out '&'s first
                            let mut derefs_needed = ptr_depth;
                            let mut cur_expr = param;
                            while derefs_needed > 0  {
                                if let ExprKind::AddrOf(BorrowKind::Ref, _, inner_expr) = cur_expr.kind {
                                    derefs_needed -= 1;
                                    cur_expr = inner_expr;
                                } else {
                                    break;
                                }
                            }

                            let derefs = "*".repeat(derefs_needed);
                            diag.span_suggestion(
                                param.span,
                                "try dereferencing",
                                format!("{}{}", derefs, snippet(cx, cur_expr.span, "<param>")),
                                Applicability::MachineApplicable,
                            );
                        }
                    },
                )
            }
        }
    }
}
