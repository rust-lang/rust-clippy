use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet;
use clippy_utils::{as_some_expr, is_no_std_crate, is_none_expr, peel_blocks, sym};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::Res;
use rustc_hir::{self as hir, HirId};
use rustc_lint::{Applicability, LateContext, LateLintPass, impl_lint_pass};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Scanning or collecting `std::env::vars()` when `std::env::var()` will do.
    ///
    /// ### Why is this bad?
    ///
    /// There is already a helper function in the standard library for getting a single env var.
    ///
    /// ### Example
    /// ```no_run
    /// use std::collections::HashMap;
    /// # fn foo() -> Option<()> {
    /// let (_, value) = std::env::vars().find(|(k, _)| k == "FOO")?;
    /// let value = std::env::vars().find_map(|(k, v)| if k == "FOO" { Some(v) } else { None })?;
    /// let value = std::env::vars().collect::<HashMap<_, _>>().get("FOO")?;
    /// # Some(())
    /// # }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # fn foo() -> Option<()> {
    /// let value = std::env::var_os("FOO")?;
    /// # Some(())
    /// # }
    /// ```
    #[clippy::version = "1.101.0"]
    pub MANUAL_ENV_VAR,
    perf,
    "scanning `std::env::vars()` for a single key"
}

impl_lint_pass!(ManualEnvVar => [MANUAL_ENV_VAR]);

#[derive(Default)]
pub struct ManualEnvVar {
    env_containers: FxHashMap<HirId, (Span, bool)>,
}

fn emit_lint<'tcx>(
    cx: &LateContext<'tcx>,
    query_expr: &hir::Expr<'tcx>,
    is_os_str: bool,
    is_existance_check: bool,
    span_sugg: Span,
    span_let: Option<Span>,
    applicability: Applicability,
) {
    let message = if is_os_str {
        "avoid using `env::vars_os()` to find individual values"
    } else {
        "avoid using `env::vars()` to find individual values"
    };
    let core_or_std = if is_no_std_crate(cx) { "core" } else { "std" };
    let var = if is_os_str { "var_os" } else { "var" };
    let query = snippet(cx, query_expr.span, "VARNAME");
    let trailing_sugg = match (is_existance_check, is_os_str) {
        (true, true) => ".is_some()",
        (true, false) => ".is_ok()",
        (false, true) => "",
        (false, false) => ".ok()",
    };
    span_lint_and_then(cx, MANUAL_ENV_VAR, span_sugg, message, |diag| {
        diag.span_suggestion(
            span_sugg,
            "replace with",
            format!("{core_or_std}::env::{var}({query}){trailing_sugg}"),
            applicability,
        );
        if let Some(span_let) = span_let {
            diag.span_label(span_let, "environment was (unnecessarily) collected here");
        }
    });
}

impl<'tcx> LateLintPass<'tcx> for ManualEnvVar {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let hir::ExprKind::Call(fn_, &[]) = expr.kind
            && let hir::ExprKind::Path(fn_qpath) = fn_.kind
            && let fn_res = cx.typeck_results().qpath_res(&fn_qpath, fn_.hir_id)
            && let Some(fn_def_id) = fn_res.opt_def_id()
            && let Some(diag_name) = fn_def_id.opt_diag_name(&cx.tcx)
            && matches!(diag_name, sym::env_vars | sym::env_vars_os)
        {
            check(cx, expr, diag_name == sym::env_vars_os);
        } else if let hir::ExprKind::MethodCall(name, recv, [query_expr], _) = expr.kind
            && matches!(name.ident.name, sym::get | sym::contains_key)
        {
            let (span_sugg, applicability) = if name.ident.name == sym::contains_key {
                (expr.span, Applicability::MachineApplicable)
            } else if name.ident.name == sym::get
                && let hir::Node::Expr(parent) = cx.tcx.parent_hir_node(expr.hir_id)
                && let hir::ExprKind::MethodCall(cloned_name, _recv, [], _) = parent.kind
                && cloned_name.ident.name == sym::cloned
            {
                (parent.span, Applicability::MachineApplicable)
            } else {
                (expr.span, Applicability::MaybeIncorrect)
            };
            if let Some(is_os_str) = maybe_env_collection(cx, recv) {
                emit_lint(
                    cx,
                    query_expr,
                    is_os_str,
                    name.ident.name == sym::contains_key,
                    span_sugg,
                    None,
                    applicability,
                );
            } else if let hir::ExprKind::Path(recv_qpath) = recv.kind
                && let recv_res = cx.typeck_results().qpath_res(&recv_qpath, recv.hir_id)
                && let Res::Local(recv_hir_id) = recv_res
                && let Some((let_span, is_os_str)) = self.env_containers.get(&recv_hir_id)
            {
                emit_lint(
                    cx,
                    query_expr,
                    *is_os_str,
                    name.ident.name == sym::contains_key,
                    span_sugg,
                    Some(*let_span),
                    applicability,
                );
            }
        }
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx hir::Stmt<'tcx>) {
        if let hir::StmtKind::Let(let_stmt) = stmt.kind
            && let_stmt.els.is_none()
            && matches!(let_stmt.source, hir::LocalSource::Normal)
            && let hir::PatKind::Binding(hir::BindingMode::NONE, env_container_hir_id, _, _) = let_stmt.pat.kind
            && let Some(init) = let_stmt.init
            && let Some(is_os_str) = maybe_env_collection(cx, init)
        {
            self.env_containers
                .insert(env_container_hir_id, (let_stmt.span, is_os_str));
        }
    }
}

#[allow(clippy::too_many_lines)]
fn check<'tcx>(cx: &LateContext<'tcx>, vars_expr: &'tcx hir::Expr<'tcx>, is_os_str: bool) {
    if let hir::Node::Expr(parent) = cx.tcx.parent_hir_node(vars_expr.hir_id) {
        let key_op = if let hir::ExprKind::MethodCall(name, recv, [arg], _) = parent.kind
            && let hir::ExprKind::Closure(closure) = arg.kind
            && recv.hir_id == vars_expr.hir_id
        {
            // `parent` is a method call with a closure parameter
            let body = cx.tcx.hir_body(closure.body);
            if let [hir::Param { pat: var_pat, .. }] = body.params
                && let hir::PatKind::Tuple(&[key_pat, value_pat], _dotdot) = var_pat.kind
                && let hir::PatKind::Binding(_, key_hir_id, _, _) = key_pat.kind
            {
                // `parent` deconstructs with pattern matching
                match name.ident.name {
                    sym::find => {
                        if let Some(query_expr) = get_key_query(cx, body.value, key_hir_id)
                            && let hir::Node::Expr(grandparent) = cx.tcx.parent_hir_node(parent.hir_id)
                            && check_map_to_value_expr(cx, grandparent)
                        {
                            Some((query_expr, false, grandparent.span))
                        } else {
                            None
                        }
                    },
                    sym::find_map => {
                        if let hir::PatKind::Binding(_, value_hir_id, _, _) = value_pat.kind
                            && let hir::ExprKind::If(cond, then, Some(else_)) = body.value.kind
                            && let Some(query_expr) = get_key_query(cx, cond, key_hir_id)
                            && let Some(then_inner) = as_some_expr(cx, peel_blocks(then))
                            && is_none_expr(cx, peel_blocks(else_))
                            && let hir::ExprKind::Path(then_inner_qpath) = then_inner.kind
                            && let then_inner_res = cx.typeck_results().qpath_res(&then_inner_qpath, then_inner.hir_id)
                            && then_inner_res == Res::Local(value_hir_id)
                        {
                            Some((query_expr, false, parent.span))
                        } else {
                            None
                        }
                    },
                    sym::any => {
                        get_key_query(cx, body.value, key_hir_id).map(|query_expr| (query_expr, true, parent.span))
                    },
                    _ => None,
                }
            } else if let [hir::Param { pat: var_pat, .. }] = body.params
                && let hir::PatKind::Binding(_, var_hir_id, _, _) = var_pat.kind
            {
                // parent does not deconstruct, but just directly accepts a tuple arg
                match name.ident.name {
                    sym::find => {
                        if let Some(query_expr) = get_key_query_tuple(cx, body.value, var_hir_id)
                            && let hir::Node::Expr(grandparent) = cx.tcx.parent_hir_node(parent.hir_id)
                            && check_map_to_value_expr(cx, grandparent)
                        {
                            Some((query_expr, false, grandparent.span))
                        } else {
                            None
                        }
                    },
                    sym::find_map => {
                        #[allow(clippy::symbol_as_str, reason = "how to handle digits?")]
                        if let hir::ExprKind::If(cond, then, Some(else_)) = body.value.kind
                            && let Some(query_expr) = get_key_query_tuple(cx, cond, var_hir_id)
                            && let Some(then_inner) = as_some_expr(cx, peel_blocks(then))
                            && is_none_expr(cx, peel_blocks(else_))
                            && let hir::ExprKind::Field(tup, field) = then_inner.kind
                            && field.as_str() == "1"
                            && let hir::ExprKind::Path(then_inner_qpath) = tup.kind
                            && let then_inner_res = cx.typeck_results().qpath_res(&then_inner_qpath, tup.hir_id)
                            && then_inner_res == Res::Local(var_hir_id)
                        {
                            Some((query_expr, false, parent.span))
                        } else {
                            None
                        }
                    },
                    sym::any => get_key_query_tuple(cx, body.value, var_hir_id)
                        .map(|query_expr| (query_expr, true, parent.span)),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };
        if let Some((query_expr, is_existance_check, span_sugg)) = key_op {
            emit_lint(
                cx,
                query_expr,
                is_os_str,
                is_existance_check,
                span_sugg,
                None,
                Applicability::MachineApplicable,
            );
        }
    }
}

fn get_key_query<'tcx>(
    cx: &LateContext<'tcx>,
    comparison_expr: &'tcx hir::Expr<'tcx>,
    key_hir_id: HirId,
) -> Option<&'tcx hir::Expr<'tcx>> {
    if let hir::ExprKind::Binary(binop, lhs, rhs) = comparison_expr.kind
        && binop.node == hir::BinOpKind::Eq
    {
        if let hir::ExprKind::Path(lhs_qpath) = lhs.kind
            && let lhs_res = cx.typeck_results().qpath_res(&lhs_qpath, lhs.hir_id)
            && lhs_res == Res::Local(key_hir_id)
        {
            // env::vars().find(|(key, _)| key == WHATEVER)
            Some(rhs)
        } else if let hir::ExprKind::Path(rhs_qpath) = rhs.kind
            && let rhs_res = cx.typeck_results().qpath_res(&rhs_qpath, rhs.hir_id)
            && rhs_res == Res::Local(key_hir_id)
        {
            // env::vars().find(|(key, _)| WHATEVER == key)
            Some(lhs)
        } else {
            None
        }
    } else {
        None
    }
}

fn get_key_query_tuple<'tcx>(
    cx: &LateContext<'tcx>,
    comparison_expr: &'tcx hir::Expr<'tcx>,
    var_hir_id: HirId,
) -> Option<&'tcx hir::Expr<'tcx>> {
    #[allow(clippy::symbol_as_str, reason = "how to handle digits?")]
    if let hir::ExprKind::Binary(binop, lhs, rhs) = comparison_expr.kind
        && binop.node == hir::BinOpKind::Eq
    {
        if let hir::ExprKind::Field(tup, field) = lhs.kind
            && field.name.as_str() == "0"
            && let hir::ExprKind::Path(lhs_qpath) = tup.kind
            && let lhs_res = cx.typeck_results().qpath_res(&lhs_qpath, lhs.hir_id)
            && lhs_res == Res::Local(var_hir_id)
        {
            // env::vars().find(|entry| entry.0 == WHATEVER)
            Some(rhs)
        } else if let hir::ExprKind::Field(tup, field) = rhs.kind
            && field.name.as_str() == "0"
            && let hir::ExprKind::Path(rhs_qpath) = tup.kind
            && let rhs_res = cx.typeck_results().qpath_res(&rhs_qpath, rhs.hir_id)
            && rhs_res == Res::Local(var_hir_id)
        {
            // env::vars().find(|entry| WHATEVER == entry.0)
            Some(lhs)
        } else {
            None
        }
    } else {
        None
    }
}

fn check_map_to_value_expr<'tcx>(cx: &LateContext<'tcx>, grandparent: &hir::Expr<'tcx>) -> bool {
    #[allow(clippy::symbol_as_str, reason = "how to handle digits?")]
    if let hir::ExprKind::MethodCall(name, _, [arg], _) = grandparent.kind
        && name.ident.name == sym::map
        && let hir::ExprKind::Closure(closure) = arg.kind
        && let body = cx.tcx.hir_body(closure.body)
        && let [hir::Param { pat: var_pat, .. }] = body.params
    {
        if let hir::PatKind::Tuple(&[_, value_pat], _dotdot) = var_pat.kind
            && let hir::PatKind::Binding(_, value_hir_id, _, _) = value_pat.kind
            && let hir::ExprKind::Path(body_value_qpath) = body.value.kind
            && let body_value_res = cx.typeck_results().qpath_res(&body_value_qpath, body.value.hir_id)
        {
            body_value_res == Res::Local(value_hir_id)
        } else if let hir::PatKind::Binding(_, var_hir_id, _, _) = var_pat.kind
            && let hir::ExprKind::Field(tup, field) = body.value.kind
            && field.name.as_str() == "1"
            && let hir::ExprKind::Path(body_qpath) = tup.kind
        {
            let rhs_res = cx.typeck_results().qpath_res(&body_qpath, tup.hir_id);
            rhs_res == Res::Local(var_hir_id)
        } else {
            false
        }
    } else {
        false
    }
}

fn maybe_env_collection<'tcx>(cx: &LateContext<'tcx>, collect_expr: &hir::Expr<'tcx>) -> Option<bool> {
    if let hir::ExprKind::MethodCall(collect_name, collect_recv, [], _) = collect_expr.kind
        && collect_name.ident.name == sym::collect
        && let hir::ExprKind::Call(fn_, &[]) = collect_recv.kind
        && let hir::ExprKind::Path(fn_qpath) = fn_.kind
        && let fn_res = cx.typeck_results().qpath_res(&fn_qpath, fn_.hir_id)
        && let Some(fn_def_id) = fn_res.opt_def_id()
        && let Some(diag_name) = fn_def_id.opt_diag_name(&cx.tcx)
        && matches!(diag_name, sym::env_vars | sym::env_vars_os)
        && let Some(collection_ty) = cx.typeck_results().expr_ty_opt(collect_expr)
        && matches!(collection_ty.opt_diag_name(cx), Some(sym::HashMap | sym::BTreeMap))
    {
        Some(diag_name == sym::env_vars_os)
    } else {
        None
    }
}
