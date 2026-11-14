use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::{snippet, snippet_with_applicability};
use clippy_utils::{fn_def_id, sym};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects `std::iter::repeat_n(val, count).collect::<Vec<_>>()` that can
    /// be replaced with `vec![val; count]`.
    ///
    /// ### Why is this bad?
    ///
    /// `vec![val; count]` is more concise, idiomatic, and can be more efficient
    /// because it avoids the iterator overhead.
    ///
    /// ### Example
    /// ```no_run
    /// let _ = std::iter::repeat_n("hello", 3).collect::<Vec<_>>();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let _ = vec!["hello"; 3];
    /// ```
    #[clippy::version = "1.102.0"]
    pub MANUAL_VEC,
    pedantic,
    "detect `repeat_n(val, count).collect::<Vec<_>>()` that can be written as `vec![val; count]`"
}

impl_lint_pass!(ManualVec => [MANUAL_VEC]);

pub struct ManualVec {
    msrv: Msrv,
}

impl ManualVec {
    pub fn new(conf: &'static clippy_config::Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualVec {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(method_name, recv, [], _) = expr.kind
            && method_name.ident.name == sym::collect
            && !expr.span.from_expansion()
        {
            check_collect_from_repeat_n(cx, expr, recv, self.msrv);
        }
    }
}

fn check_collect_from_repeat_n<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, recv: &Expr<'tcx>, msrv: Msrv) {
    let Some(vec_def_id) = cx.typeck_results().expr_ty(expr).opt_def_id() else {
        return;
    };

    if cx.tcx.opt_item_name(vec_def_id) == Some(sym::Vec)
        && msrv.meets(cx, msrvs::REPEAT_N) &&
        !expr.span.from_expansion() &&
        let ExprKind::Call(_, [val_arg, count_arg]) = recv.kind
        && let Some(def_id) = fn_def_id(cx, recv)
        // TODO: replace with cx.tcx.is_diagnostic_item()
        // when repeat_n is added as a diagnostic item in rust
        && cx.tcx.item_name(def_id) == sym::repeat_n
    {
        let mut app = Applicability::HasPlaceholders;
        let val = snippet_with_applicability(cx, val_arg.span, "..", &mut app);
        let count = snippet(cx, count_arg.span, "..");
        span_lint_and_sugg(
            cx,
            MANUAL_VEC,
            expr.span,
            "this `repeat_n().collect::<Vec>()` can be written more concisely",
            "use:",
            format!("vec![{val}; {count}]"),
            app,
        );
    }
}
