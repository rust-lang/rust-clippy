use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{sym, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for cases where `unwrap_or_else` can be used in lieu of
    /// `get_or_insert_with` followed by `unwrap`/`unwrap_unchecked`/`expect`.
    ///
    /// ### Why is this bad?
    /// It is more concise to use `unwrap_or_else`, and using `unwrap_or_else`
    /// instead of `unwrap_unchecked` eliminates the need for an `unsafe`
    /// block.
    ///
    /// ### Example
    /// ```rust
    /// let mut opt: Option<i32> = None;
    /// opt.get_or_insert(42);
    /// let res = unsafe { opt.unwrap_unchecked() };
    /// ```
    /// Use instead:
    /// ```rust
    /// let opt: Option<i32> = None;
    /// let res: i32 = opt.unwrap_or(42);
    /// ```
    #[clippy::version = "1.74.0"]
    pub MANUAL_OPTION_FOLDING,
    style,
    "manual implementation of `Option::unwrap_or_else`"
}

impl_lint_pass!(ManualOptionFolding<'_> => [MANUAL_OPTION_FOLDING]);

pub struct ManualOptionFolding<'tcx> {
    get_call: Option<&'tcx hir::Expr<'tcx>>,
    recv: Option<&'tcx hir::Expr<'tcx>>,
    get_method_name: Option<Symbol>,
}

impl<'tcx> ManualOptionFolding<'tcx> {
    pub fn new() -> Self {
        Self {
            get_call: None,
            recv: None,
            get_method_name: None,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualOptionFolding<'tcx> {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if !expr.span.from_expansion()
            && let hir::ExprKind::MethodCall(path, recv, ..) = expr.kind
            && let ty = cx.typeck_results().expr_ty(recv).peel_refs()
            && is_type_diagnostic_item(cx, ty, sym::Option)
        {
            if path.ident.name == sym!(get_or_insert)
                || path.ident.name == sym!(get_or_insert_with)
                || path.ident.name == sym!(get_or_insert_default)
            {
                self.get_call = Some(expr);
                self.recv = Some(recv);
                self.get_method_name = Some(path.ident.name);
            } else if let Some(get_call) = self.get_call
                && let Some(get_call_recv) = self.recv
                && let Some(get_method_name) = self.get_method_name
                && (path.ident.name == sym::unwrap
                    || path.ident.name == sym!(unwrap_unchecked)
                    || path.ident.name == sym::expect)
                && let hir::ExprKind::Path(hir::QPath::Resolved(_, recv_path)) = recv.kind
                && let hir::ExprKind::Path(hir::QPath::Resolved(_, get_call_recv_path)) = get_call_recv.kind
                && recv_path.res == get_call_recv_path.res
            {
                let sugg_method = if get_method_name == sym!(get_or_insert) {
                    "unwrap_or".to_string()
                } else if get_method_name == sym!(get_or_insert_with) {
                    "unwrap_or_else".to_string()
                } else {
                    "unwrap_or_default".to_string()
                };

                span_lint_and_then(
                    cx,
                    MANUAL_OPTION_FOLDING,
                    expr.span,
                    &format!("`{}` used after `{get_method_name}`", path.ident.name),
                    |diag| {
                        diag.span_note(get_call.span, format!("`{get_method_name}` used here"));
                        diag.help(format!("try using `{sugg_method}` instead"));
                    }
                );
            }
        }
    }
}
