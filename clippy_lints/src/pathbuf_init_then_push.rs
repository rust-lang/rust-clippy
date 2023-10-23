use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::path_to_local_id;
use clippy_utils::source::{snippet, snippet_opt};
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{BindingAnnotation, Block, Expr, ExprKind, HirId, Local, PatKind, QPath, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{sym, Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to `push` immediately after creating a new `PathBuf`.
    ///
    /// ### Why is this bad?
    /// The `.join()` is easier to read than multiple `push` calls.
    ///
    /// ### Known problems
    /// `.join()` introduces an implicit `clone()`
    ///
    /// ### Example
    /// ```rust
    /// let mut path_buf = PathBuf::new();
    /// path_buf.push("foo");
    /// ```
    /// Use instead:
    /// ```rust
    /// let path_buf = PathBuf::new().join("foo");
    /// ```
    #[clippy::version = "1.75.0"]
    pub PATHBUF_INIT_THEN_PUSH,
    complexity,
    "`push` immediately after `PathBuf` creation"
}

impl_lint_pass!(PathbufThenPush => [PATHBUF_INIT_THEN_PUSH]);

#[derive(Default)]
pub struct PathbufThenPush {
    searcher: Option<PathbufPushSearcher>,
}

struct PathbufPushSearcher {
    local_id: HirId,
    lhs_is_let: bool,
    let_ty_span: Option<Span>,
    init_val_span: Span,
    arg_span: Option<Span>,
    name: Symbol,
    err_span: Span,
}

impl PathbufPushSearcher {
    fn display_err(&self, cx: &LateContext<'_>) {
        let Some(arg_span) = self.arg_span else { return };
        let Some(arg_str) = snippet_opt(cx, arg_span) else {
            return;
        };
        let Some(init_val) = snippet_opt(cx, self.init_val_span) else {
            return;
        };
        let mut s = if self.lhs_is_let {
            String::from("let ")
        } else {
            String::new()
        };
        s.push_str("mut ");
        s.push_str(self.name.as_str());
        if let Some(span) = self.let_ty_span {
            s.push_str(": ");
            s.push_str(&snippet(cx, span, "_"));
        }
        s.push_str(&format!(" = {init_val}.join({arg_str});",));

        span_lint_and_sugg(
            cx,
            PATHBUF_INIT_THEN_PUSH,
            self.err_span,
            "calls to `push` immediately after creation",
            "consider using the `.join()`",
            s,
            Applicability::HasPlaceholders,
        );
    }
}

impl<'tcx> LateLintPass<'tcx> for PathbufThenPush {
    fn check_block(&mut self, _: &LateContext<'tcx>, _: &'tcx Block<'tcx>) {
        self.searcher = None;
    }

    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx Local<'tcx>) {
        if let Some(init_expr) = local.init
            && let PatKind::Binding(BindingAnnotation::MUT, id, name, None) = local.pat.kind
            && !in_external_macro(cx.sess(), local.span)
            && let ty = cx.typeck_results().pat_ty(local.pat)
            && is_type_diagnostic_item(cx, ty, sym::PathBuf)
        {
            self.searcher = Some(PathbufPushSearcher {
                local_id: id,
                lhs_is_let: true,
                name: name.name,
                let_ty_span: local.ty.map(|ty| ty.span),
                err_span: local.span,
                init_val_span: init_expr.span,
                arg_span: None,
            });
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if self.searcher.is_none()
            && let ExprKind::Assign(left, right, _) = expr.kind
            && let ExprKind::Path(QPath::Resolved(None, path)) = left.kind
            && let [name] = &path.segments
            && let Res::Local(id) = path.res
            && !in_external_macro(cx.sess(), expr.span)
            && let ty = cx.typeck_results().expr_ty(left)
            && is_type_diagnostic_item(cx, ty, sym::PathBuf)
        {
            self.searcher = Some(PathbufPushSearcher {
                local_id: id,
                lhs_is_let: false,
                let_ty_span: None,
                name: name.ident.name,
                err_span: expr.span,
                init_val_span: right.span,
                arg_span : None,
            });
        }
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'_>) {
        if let Some(mut searcher) = self.searcher.take() {
            if let StmtKind::Expr(expr) | StmtKind::Semi(expr) = stmt.kind
                && let ExprKind::MethodCall(name, self_arg, [arg_expr], _) = expr.kind
                && path_to_local_id(self_arg, searcher.local_id)
                && name.ident.as_str() == "push"
            {
                searcher.err_span = searcher.err_span.to(stmt.span);
                searcher.arg_span = Some(arg_expr.span);
                searcher.display_err(cx);
            }
        }
    }

    fn check_block_post(&mut self, cx: &LateContext<'tcx>, _: &'tcx Block<'tcx>) {
        if let Some(searcher) = self.searcher.take() {
            searcher.display_err(cx);
        }
    }
}
