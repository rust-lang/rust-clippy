use crate::utils::{
    is_ctor_or_promotable_const_function, match_def_path, match_type, paths, snippet_with_applicability,
    span_lint_and_then,
};
use if_chain::if_chain;
use rustc::hir::intravisit::Visitor;
use rustc::hir::{self, *};
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::ty::Ty;
use rustc::{declare_lint_pass, declare_tool_lint};
use rustc_errors::Applicability;
use syntax_pos::Span;

declare_clippy_lint! {
    /// **What it does:** Checks for usages of `option.ok_or_else(|| <..>::from(..))?` or
    /// `result.map_err(|x| <..>::from(..))`.
    ///
    /// **Why is this bad?** The `?` operator will call `from` in the `Err` case,
    /// so calling it manually is redundant.
    ///
    /// **Known problems:** The suggested fix does not correct any explicitly provided
    /// type arguments in `ok_or_else` or `map_err`.
    ///
    /// **Example:**
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
    pub UNNEEDED_TRY_CONVERT,
    complexity,
    "unneeded conversion inside `?`"
}

declare_lint_pass!(UnneededTryConvert => [UNNEEDED_TRY_CONVERT]);

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UnneededTryConvert {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        if_chain! {
            if let ExprKind::Match(match_arg, arms, MatchSource::TryDesugar) = &expr.kind;
            if let ExprKind::Call(_, args) = &match_arg.kind;
            if let [try_arg] = &**args;
            if let Some(fn_error_ty) = get_try_err_ty(cx, arms);
            then {
                check_option_ok_or_else(cx, fn_error_ty, try_arg);
                check_result_map_err(cx, fn_error_ty, try_arg);
            }
        }
    }
}

/// Given the arms of a `match` expr from desugaring a `?`, return the error type of the `Try` type
fn get_try_err_ty<'tcx>(cx: &LateContext<'_, 'tcx>, match_arms: &[Arm]) -> Option<Ty<'tcx>> {
    if_chain! {
        if let [err_arm, _] = match_arms;
        if let ExprKind::Ret(Some(ret_expr)) = &err_arm.body.kind;
        if let ExprKind::Call(_, try_from_error_args) = &ret_expr.kind;
        if let [try_from_error_arg] = &**try_from_error_args;
        then {
            return Some(cx.tables.expr_ty(try_from_error_arg));
        }
    }
    None
}

fn check_option_ok_or_else<'tcx>(cx: &LateContext<'_, 'tcx>, fn_error_ty: Ty<'tcx>, expr: &Expr) {
    if_chain! {
        if let ExprKind::MethodCall(call_path, _, call_args) = &expr.kind;
        if call_path.ident.as_str() == "ok_or_else";
        if let [receiver, closure_expr] = &**call_args;
        if match_type(cx, cx.tables.expr_ty(receiver), &paths::OPTION);
        if let Some((closure_body_span, conv_arg)) = check_closure_expr(cx, fn_error_ty, closure_expr);
        then {
            let mut applicability = Applicability::MachineApplicable;
            let conv_arg_snip = snippet_with_applicability(cx, conv_arg.span, "..", &mut applicability);
            let (sugg_span, sugg_snip) = if is_trivial_expr(cx, conv_arg) {
                // suggest inlining the closure and using `ok_or`
                let receiver_snip = snippet_with_applicability(cx, receiver.span, "..", &mut applicability);
                (expr.span, format!("{}.ok_or({})", receiver_snip, conv_arg_snip))
            } else {
                // suggest removing the conversion in the closure
                (closure_body_span, conv_arg_snip.into_owned())
            };
            emit_lint(cx, closure_body_span, sugg_span, sugg_snip, applicability);
        }
    }
}

fn check_result_map_err<'tcx>(cx: &LateContext<'_, 'tcx>, fn_error_ty: Ty<'tcx>, expr: &Expr) {
    if_chain! {
        if let ExprKind::MethodCall(call_path, _, call_args) = &expr.kind;
        if call_path.ident.as_str() == "map_err";
        if let [receiver, mapper_expr] = &**call_args;
        let receiver_ty = cx.tables.expr_ty(receiver);
        if match_type(cx, receiver_ty, &paths::RESULT);
        then {
            if let Some((closure_body_span, conv_arg)) = check_closure_expr(cx, fn_error_ty, mapper_expr) {
                // suggest removing just the conversion in the closure
                let mut applicability = Applicability::MachineApplicable;
                let conv_arg_snip = snippet_with_applicability(cx, conv_arg.span, "..", &mut applicability);
                emit_lint(
                    cx,
                    closure_body_span,
                    closure_body_span,
                    conv_arg_snip.into_owned(),
                    applicability,
                );
                return;
            }
            if_chain! {
                if let ExprKind::Path(qpath) = &mapper_expr.kind;
                if let def::Res::Def(_, def_id) = cx.tables.qpath_res(qpath, mapper_expr.hir_id);
                if match_def_path(cx, def_id, &paths::FROM_FROM)
                    || match_def_path(cx, def_id, &paths::INTO_INTO);
                if *cx.tables.expr_ty(mapper_expr).fn_sig(cx.tcx).output().skip_binder() == fn_error_ty;
                then {
                    // suggest removing the entire `map_err(..)` call
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
    }
}

fn emit_lint(cx: &LateContext<'_, '_>, lint_span: Span, sugg_span: Span, sugg: String, applicability: Applicability) {
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

/// If `closure_expr` is a closure whose body is a conversion to `fn_error_ty`,
/// return (the span of the conversion call, the argument of the conversion call)
fn check_closure_expr<'tcx>(
    cx: &LateContext<'_, 'tcx>,
    fn_error_ty: Ty<'tcx>,
    closure_expr: &Expr,
) -> Option<(Span, &'tcx Expr)> {
    if_chain! {
        if let ExprKind::Closure(_, _, body_id, _, _) = closure_expr.kind;
        let closure_body = &cx.tcx.hir().body(body_id).value;
        if let Some(conv_arg) = conversion_subject(cx, closure_body);
        if cx.tables.expr_ty(closure_body) == fn_error_ty;
        then {
            return Some((closure_body.span, conv_arg));
        }
    }
    None
}

/// If `expr` is `From::from(<inner>)` or `(<inner>).into()`, returns `<inner>`.
fn conversion_subject<'tcx>(cx: &LateContext<'_, 'tcx>, expr: &'tcx Expr) -> Option<&'tcx Expr> {
    if_chain! {
        if let ExprKind::Call(fn_expr, from_args) = &expr.kind;
        if let ExprKind::Path(fn_qpath) = &fn_expr.kind;
        if let def::Res::Def(def::DefKind::Method, fn_did) = cx.tables.qpath_res(fn_qpath, fn_expr.hir_id);
        if match_def_path(cx, fn_did, &paths::FROM_FROM);
        if let [from_arg] = &**from_args;
        then {
            return Some(from_arg);
        }
    }
    if_chain! {
        if let ExprKind::MethodCall(_, _, args) = &expr.kind;
        if let Some(call_did) = cx.tables.type_dependent_def_id(expr.hir_id);
        if match_def_path(cx, call_did, &paths::INTO_INTO);
        if let [receiver] = &**args;
        then {
            return Some(receiver);
        }
    }
    None
}

/// Is this expression "trivial" such that a closure containing it could be inlined?
/// (currently very conservative)
fn is_trivial_expr<'tcx>(cx: &LateContext<'_, 'tcx>, expr: &'tcx Expr) -> bool {
    struct TrivialVisitor<'a, 'tcx> {
        cx: &'a LateContext<'a, 'tcx>,
        trivial: bool,
    }

    impl<'a, 'tcx> intravisit::Visitor<'tcx> for TrivialVisitor<'a, 'tcx> {
        fn visit_expr(&mut self, expr: &'tcx hir::Expr) {
            // whitelist of definitely trivial expressions
            self.trivial &= match &expr.kind {
                hir::ExprKind::Call(..) => is_ctor_or_promotable_const_function(self.cx, expr),
                hir::ExprKind::Tup(..)
                | hir::ExprKind::Lit(..)
                | hir::ExprKind::Cast(..)
                | hir::ExprKind::Field(..)
                | hir::ExprKind::Index(..)
                | hir::ExprKind::Path(..)
                | hir::ExprKind::AddrOf(..)
                | hir::ExprKind::Struct(..) => true,
                _ => false,
            };

            if self.trivial {
                intravisit::walk_expr(self, expr);
            }
        }

        fn nested_visit_map<'this>(&'this mut self) -> intravisit::NestedVisitorMap<'this, 'tcx> {
            intravisit::NestedVisitorMap::None
        }
    }

    let mut visitor = TrivialVisitor { cx, trivial: true };
    visitor.visit_expr(expr);
    visitor.trivial
}
