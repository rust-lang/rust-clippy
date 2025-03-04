use clippy_utils::{is_ctor_or_promotable_const_function, match_def_path};

use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::match_type;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::intravisit::{Visitor, nested_filter, walk_expr};
use rustc_hir::{Arm, Expr, ExprKind, MatchSource, def};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usages of `option.ok_or_else(|| <..>::from(..))?` or
    /// `result.map_err(|x| <..>::from(..))`.
    ///
    /// ### Why is this bad?
    /// The `?` operator will call `from` in the `Err` case,
    /// so calling it manually is redundant.
    ///
    /// ### Known problems
    /// The suggested fix does not correct any explicitly provided
    /// type arguments in `ok_or_else` or `map_err`.
    ///
    /// ### Example
    /// ```rust
    /// fn bar() -> Result<i32, String> {
    ///     let x = Some(52.3).ok_or_else(|| String::from("foo"))?;
    ///     Ok(42)
    /// }
    /// ```
    /// Could be written:
    ///
    /// ```rust
    /// fn bar() -> Result<i32, String> {
    ///     let x = Some(52.3).ok_or("foo")?;
    ///     Ok(42)
    /// }
    /// ```
    #[clippy::version = "1.83.0"]
    pub UNNEEDED_TRY_CONVERT,
    complexity,
    "unneeded conversion inside `?`"
}

declare_lint_pass!(UnneededTryConvert => [UNNEEDED_TRY_CONVERT]);

impl<'tcx> LateLintPass<'tcx> for UnneededTryConvert {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr) {
        if let ExprKind::Match(match_arg, arms, MatchSource::TryDesugar(_)) = &expr.kind
            && let ExprKind::Call(_, args) = &match_arg.kind
            && let [try_arg] = &**args
            && let Some(fn_error_ty) = get_try_err_ty(cx, arms)
        {
            check_option_ok_or_else(cx, fn_error_ty, try_arg);
            check_result_map_err(cx, fn_error_ty, try_arg);
        }
    }
}

fn get_try_err_ty<'tcx>(cx: &LateContext<'tcx>, match_arms: &[Arm]) -> Option<Ty<'tcx>> {
    if let [err_arm, _] = match_arms
        && let ExprKind::Ret(Some(ret_expr)) = &err_arm.body.kind
        && let ExprKind::Call(_, try_from_error_args) = &ret_expr.kind
        && let [try_from_error_arg] = &**try_from_error_args
    {
        return Some(cx.typeck_results().expr_ty(try_from_error_arg));
    }
    None
}

fn check_option_ok_or_else<'tcx>(cx: &LateContext<'tcx>, fn_error_ty: Ty<'tcx>, expr: &Expr) {
    if let ExprKind::MethodCall(call_path, receiver, call_args, _) = &expr.kind
        && call_path.ident.as_str() == "ok_or_else"
        && let [closure_expr] = &**call_args
        && is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(receiver), sym::Option)
        && let Some((closure_body_span, conv_arg)) = check_closure_expr(cx, fn_error_ty, closure_expr)
    {
        let mut applicability = Applicability::MachineApplicable;
        let conv_arg_snip = snippet_with_applicability(cx, conv_arg.span, "..", &mut applicability);
        let receiver_snip = snippet_with_applicability(cx, receiver.span, "..", &mut applicability);

        // Always use ok_or when we detect a From::from conversion
        let sugg_span = expr.span;
        let sugg_snip = format!("{}.ok_or({})", receiver_snip, conv_arg_snip);

        emit_lint(cx, closure_body_span, sugg_span, sugg_snip, applicability);
    }
}

fn check_result_map_err<'tcx>(cx: &LateContext<'tcx>, fn_error_ty: Ty<'tcx>, expr: &Expr) {
    if let ExprKind::MethodCall(call_path, receiver, call_args, _) = &expr.kind
        && call_path.ident.as_str() == "map_err"
        && let [mapper_expr] = &**call_args
        && let receiver_ty = cx.typeck_results().expr_ty(receiver)
        && is_type_diagnostic_item(cx, receiver_ty, sym::Result)
    {
        if let Some((closure_body_span, conv_arg)) = check_closure_expr(cx, fn_error_ty, mapper_expr) {
            let mut applicability = Applicability::MachineApplicable;
            let receiver_snip = snippet_with_applicability(cx, receiver.span, "..", &mut applicability);
            emit_lint(
                cx,
                closure_body_span,
                expr.span,
                receiver_snip.into_owned(),
                applicability,
            );
            return;
        }

        if let ExprKind::Path(qpath) = &mapper_expr.kind
            && let def::Res::Def(_, def_id) = cx.typeck_results().qpath_res(&qpath, mapper_expr.hir_id)
            && (match_def_path(cx, def_id, &["core", "convert", "From", "from"])
                || match_def_path(cx, def_id, &["core", "convert", "Into", "into"]))
            && cx
                .typeck_results()
                .expr_ty(mapper_expr)
                .fn_sig(cx.tcx)
                .output()
                .skip_binder()
                == fn_error_ty
        {
            let mut applicability = Applicability::MachineApplicable;
            let receiver_snip = snippet_with_applicability(cx, receiver.span, "..", &mut applicability);
            emit_lint(
                cx,
                mapper_expr.span,
                expr.span,
                receiver_snip.into_owned(),
                applicability,
            );
        }
    }
}

fn emit_lint(cx: &LateContext<'_>, lint_span: Span, sugg_span: Span, sugg: String, applicability: Applicability) {
    span_lint_and_then(
        cx,
        UNNEEDED_TRY_CONVERT,
        lint_span,
        "unneeded conversion inside `?`",
        move |db| {
            db.note("the `?` operator will automatically call `from` in the `Err` case");
            db.span_suggestion(sugg_span, "remove the conversion", sugg, applicability);
        },
    );
}

fn check_closure_expr<'tcx>(
    cx: &LateContext<'tcx>,
    fn_error_ty: Ty<'tcx>,
    closure_expr: &Expr,
) -> Option<(Span, &'tcx Expr<'tcx>)> {
    if let ExprKind::Closure(closure) = closure_expr.kind
        && let closure_body = &cx.tcx.hir().body(closure.body).value
        && let Some(conv_arg) = conversion_subject(cx, closure_body)
        && cx.typeck_results().expr_ty(closure_body) == fn_error_ty
    {
        return Some((closure_body.span, conv_arg));
    }
    None
}

fn conversion_subject<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::Call(fn_expr, from_args) = &expr.kind
        && let ExprKind::Path(fn_qpath) = &fn_expr.kind
        && let def::Res::Def(def::DefKind::AssocFn, fn_did) = cx.typeck_results().qpath_res(fn_qpath, fn_expr.hir_id)
        && match_def_path(cx, fn_did, &["core", "convert", "From", "from"])
        && let [from_arg] = &**from_args
    {
        return Some(from_arg);
    }

    if let ExprKind::MethodCall(_, receiver, args, _) = &expr.kind
        && let Some(call_did) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && match_def_path(cx, call_did, &["core", "convert", "Into", "into"])
        && let [receiver] = &**args
    {
        return Some(receiver);
    }
    None
}

/// Is this expression "trivial" such that a closure containing it could be inlined?
/// (currently very conservative)
fn is_trivial_expr<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr) -> bool {
    struct TrivialVisitor<'a, 'tcx> {
        cx: &'a LateContext<'tcx>,
        trivial: bool,
    }

    impl<'a, 'tcx> Visitor<'tcx> for TrivialVisitor<'a, 'tcx> {
        type NestedFilter = rustc_hir::intravisit::nested_filter::None;

        fn visit_expr(&mut self, expr: &'tcx Expr) {
            self.trivial &= match &expr.kind {
                ExprKind::Call(..) => is_ctor_or_promotable_const_function(self.cx, expr),
                ExprKind::Tup(..)
                | ExprKind::Lit(..)
                | ExprKind::Cast(..)
                | ExprKind::Field(..)
                | ExprKind::Index(..)
                | ExprKind::Path(..)
                | ExprKind::AddrOf(..)
                | ExprKind::Struct(..) => true,
                _ => false,
            };

            if self.trivial {
                walk_expr(self, expr);
            }
        }
    }

    let mut visitor = TrivialVisitor { cx, trivial: true };
    visitor.visit_expr(expr);
    visitor.trivial
}
