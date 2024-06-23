use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{fn_def_id, match_def_path};
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{Block, Closure, Expr, ExprKind, FnDecl, HirId, Node, Pat, Path, QPath, Ty};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Detects usage of `Option::and_then` and `bool::then_some` that could
    /// be replaced with `Option::filter`.
    ///
    /// ### Why is this bad?
    /// Needless complexity, uses recent and uncommon stdlib functions instead of
    /// one older function.
    ///
    /// ### Example
    /// ```no_run
    /// let x = Some("foo".to_string());
    /// let _y = x.clone().and_then(|v| v.starts_with('f').then_some(v));
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x = Some("foo".to_string());
    /// let _y = x.clone().filter(|v| v.starts_with('f'));
    /// ```
    #[clippy::version = "1.81.0"]
    pub AND_THEN_THEN_SOME,
    nursery,
    "detects usage of and_then and then_some that can be replaced by filter"
}

// note: `Option::filter` is older than `bool::then_some`,
// so no msrv check is required.
declare_lint_pass!(AndThenThenSome => [AND_THEN_THEN_SOME]);

impl<'tcx> LateLintPass<'tcx> for AndThenThenSome {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::MethodCall(_, selfarg, [arg], _) | ExprKind::Call(_, [selfarg, arg]) => {
                // TODO: check if type of reciever is diagnostic item Option?
                if is_and_then(cx, expr) {
                    if let Some((closure_args, predicate)) = then_some_closure_arg(cx, arg) {
                        show_sugg(cx, expr.span, selfarg, closure_args, predicate);
                    }
                }
            },
            _ => {},
        }
    }
}

// `|v| X.then_some(v)` -> Some((span"|v|", X))
fn then_some_closure_arg<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Option<(Span, &'tcx Expr<'tcx>)> {
    match expr.kind {
        ExprKind::Closure(Closure {
            fn_decl: FnDecl {
                inputs: [Ty { hir_id: arg_id, .. }],
                ..
            },
            body,
            ..
        }) => {
            if let Node::Expr(expr) = cx.tcx.hir_node(body.hir_id) {
                (peel_closure_body(cx, expr, *arg_id)).map(|body| (cx.tcx.hir().span(*arg_id), body))
            } else {
                None
            }
        },
        _ => None,
    }
}

fn peel_closure_body<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    closure_arg_id: HirId,
) -> Option<&'tcx Expr<'tcx>> {
    match expr.kind {
        // it would be nice if we could lift { x; y.a() } into { x; y }.a()
        ExprKind::Block(
            Block {
                stmts: [],
                expr: Some(wrapped_expr),
                ..
            },
            _,
        ) => peel_closure_body(cx, wrapped_expr, closure_arg_id),
        ExprKind::MethodCall(_, pred, [arg], _) | ExprKind::Call(_, [pred, arg]) => {
            if is_then_some(cx, expr) && is_local_defined_at(cx, arg, closure_arg_id) {
                Some(pred)
            } else {
                None
            }
        },
        _ => None,
    }
}

fn is_local_defined_at(cx: &LateContext<'_>, local: &Expr<'_>, arg_hid: HirId) -> bool {
    match local.kind {
        ExprKind::Path(QPath::Resolved(
            _,
            Path {
                res: Res::Local(local_hid),
                ..
            },
        )) => {
            // FIXME: this is the best way i could find to compare if
            // a local refers to a specific closure argument.
            //
            // it breaks if the closure argument has an explicitly declared type,
            // since the spans only align for TyKind::Infer
            if let Node::Pat(Pat { span: local_span, .. }) = (cx.tcx.hir_node(*local_hid))
                && let Node::Ty(Ty { span: arg_span, .. }) = (cx.tcx.hir_node(arg_hid))
                && local_span == arg_span
            {
                true
            } else {
                false
            }
        },
        // is not local at all, so definitly isn't a local defined at the given position
        _ => false,
    }
}

fn show_sugg(cx: &LateContext<'_>, span: Span, selfarg: &Expr<'_>, closure_args: Span, predicate: &Expr<'_>) {
    let mut appl = Applicability::MachineApplicable;
    let sugg = format!(
        "{}.filter(|{}| {})",
        snippet_with_applicability(cx, selfarg.span, "<OPTION>", &mut appl),
        snippet_with_applicability(cx, closure_args, "<ARGS>", &mut appl),
        snippet_with_applicability(cx, predicate.span, "<PREDICATE>", &mut appl)
    );
    span_lint_and_sugg(
        cx,
        AND_THEN_THEN_SOME,
        span,
        "use of `and_then` + `then_some` is equivelent to `filter`",
        "use `Option::filter` instead",
        sugg,
        appl,
    );
}

fn is_then_some(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let Some(def_id) = fn_def_id(cx, expr) {
        match_def_path(cx, def_id, &["core", "bool", "<impl bool>", "then_some"])
    } else {
        false
    }
}

fn is_and_then(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let Some(def_id) = fn_def_id(cx, expr) {
        match_def_path(cx, def_id, &["core", "option", "Option", "and_then"])
    } else {
        false
    }
}
