use clippy_utils::consts::ConstEvalCtxt;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sym;
use rustc_data_structures::fx::FxIndexMap;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{Body, ExprKind, HirId, Mutability, PatKind, QPath, StmtKind};
use rustc_lint::LateContext;
use rustc_span::Symbol;

use super::REDUNDANT_IDEMPOTENT_CALLS;

// Checks all function code
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
    // If the body of the function is a block
    if let ExprKind::Block(block, _) = &body.value.kind {
        // tracks which local variables have already had an idempotent method applied
        // and the arguments used with them
        let mut map = FxIndexMap::default();
        walk_block(cx, block, &mut map);
    }
}
// Checks blocks of code (ifs, loops, match arms, etc...)
fn walk_block<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    for stmt in block.stmts {
        match &stmt.kind {
            StmtKind::Let(local) => check_let(cx, local, map),
            StmtKind::Semi(expr) | StmtKind::Expr(expr) => {
                check_expr(cx, expr, map);
            },
            StmtKind::Item(_) => {},
        }
    }
    // Covers the final expression the block evaluates to
    if let Some(expr) = block.expr {
        return check_expr(cx, expr, map);
    }
    None
}
// Checking functions for language elements
// Checks any expression
fn check_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    match &expr.kind {
        ExprKind::MethodCall(method, receiver, args, _) => check_method_call(cx, expr, method, receiver, args, map),
        ExprKind::Assign(left_value, right_value, _) | ExprKind::AssignOp(_, left_value, right_value) => {
            check_assign(cx, left_value, right_value, map)
        },
        ExprKind::Loop(block, _, _, _) => check_loop(cx, block, map),
        ExprKind::Match(_, arms, _) => check_match(cx, arms, map),
        ExprKind::Call(_, args) => check_func_args(cx, args, map),
        ExprKind::Closure(closure) => check_closure(cx, closure, map),
        ExprKind::Block(block, _) => walk_block(cx, block, map),
        ExprKind::Ret(Some(expr))
        | ExprKind::Cast(expr, _)
        | ExprKind::Unary(_, expr)
        | ExprKind::Repeat(expr, _)
        | ExprKind::AddrOf(_, _, expr) => check_expr(cx, expr, map),
        ExprKind::DropTemps(inner) => check_expr(cx, inner, map),
        ExprKind::Let(let_expr) => check_expr(cx, let_expr.init, map),
        ExprKind::Index(arr, idx, _) => {
            check_expr(cx, arr, map);
            check_expr(cx, idx, map);
            None
        },
        ExprKind::If(cond, then_block, else_block) => {
            check_expr(cx, cond, map);
            check_if(cx, then_block, *else_block, map)
        },
        ExprKind::Binary(_, left, right) => {
            check_expr(cx, left, map);
            check_expr(cx, right, map);
            None
        },
        ExprKind::Tup(exprs) | ExprKind::Array(exprs) => {
            for expr in *exprs {
                check_expr(cx, expr, map);
            }
            None
        },
        ExprKind::Struct(_, fields, base) => {
            for field in *fields {
                check_expr(cx, field.expr, map);
            }
            if let rustc_hir::StructTailExpr::Base(base_expr) = base {
                check_expr(cx, base_expr, map);
            }
            None
        },
        _ => None,
    }
}

// Checks a let expression
fn check_let<'tcx>(
    cx: &LateContext<'tcx>,
    local: &'tcx rustc_hir::LetStmt<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) {
    if let Some(init) = local.init {
        // inrelevant and inserted by the compiler
        let expr = strip_parens_and_temps(init);

        if let ExprKind::MethodCall(method, receiver, args, _) = &expr.kind
            && is_idempotent(cx, expr, method.ident.name)
        {
            check_method_call(cx, expr, method, receiver, args, map);
            // record the new binding if it is a simple identifier
            if let PatKind::Binding(_, hir_id, _, _) = local.pat.kind {
                map.insert(hir_id, (method.ident.name, args));
            }
        } else if let PatKind::Binding(_, hir_id, _, _) = local.pat.kind {
            // try to inherit the alias symbol otherwise we check the expr
            if !try_inherit_alias(expr, hir_id, map)
                && let Some((symbol, args)) = check_expr(cx, expr, map)
            {
                map.insert(hir_id, (symbol, args));
            }
        } else {
            check_expr(cx, expr, map);
        }
    }

    if let Some(els) = local.els {
        walk_block(cx, els, map);
    }
}

// Checks for method calls
fn check_method_call<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
    method: &rustc_hir::PathSegment<'tcx>,
    receiver: &'tcx rustc_hir::Expr<'tcx>,
    args: &'tcx [rustc_hir::Expr<'tcx>],
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    let mut applicability = Applicability::MachineApplicable;
    let peeled_receiver = strip_parens_and_temps(receiver);
    let method_name = method.ident.name;
    let is_idemp = is_idempotent(cx, expr, method_name);

    // recurse to catch inner redundancies
    check_expr(cx, receiver, map);

    // invalidate args mutable
    check_func_args(cx, args, map);

    // if the receiver is borrowed with a "&mut self"
    if let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) {
        let fn_sig = cx.tcx.fn_sig(def_id).skip_binder();
        if let Some(first_ty) = fn_sig.inputs().skip_binder().first()
            && let rustc_middle::ty::Ref(_, _, Mutability::Mut) = first_ty.kind()
        {
            invalidate_left_value(peeled_receiver, map);
        }
    }

    if is_idemp
        && let Some(hir_id) = path_to_local(peeled_receiver)
        && let Some((recorded_method, recorded_args)) = map.get(&hir_id)
    {
        if *recorded_method == method_name && are_args_equal(cx, recorded_args, args) {
            span_lint_and_sugg(
                cx,
                REDUNDANT_IDEMPOTENT_CALLS,
                expr.span,
                "redundant call to idempotent method, the result is already the same",
                "replace with",
                snippet_with_applicability(cx, peeled_receiver.span, "..", &mut applicability).to_string(),
                applicability,
            );
        } else {
            map.insert(hir_id, (method_name, args));
            return Some((method_name, args));
        }
    } else if is_idemp && let ExprKind::MethodCall(recursive_method, _, recv_args, _) = peeled_receiver.kind {
        if method_name == recursive_method.ident.name && are_args_equal(cx, recv_args, args) {
            span_lint_and_sugg(
                cx,
                REDUNDANT_IDEMPOTENT_CALLS,
                expr.span,
                "redundant call to idempotent method, the result is already the same",
                "replace with",
                snippet_with_applicability(cx, peeled_receiver.span, "..", &mut applicability).to_string(),
                applicability,
            );
        }
    } else if is_idemp {
        return Some((method_name, args));
    }
    None
}

fn check_if<'tcx>(
    cx: &LateContext<'tcx>,
    then_block: &'tcx rustc_hir::Expr<'tcx>,
    else_block: Option<&'tcx rustc_hir::Expr<'tcx>>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    // each branch gets its own clone to avoid cross contamination
    let mut then_map = map.clone();
    if let ExprKind::Block(block, _) = &then_block.kind {
        walk_block(cx, block, &mut then_map);
    }

    if let Some(else_expr) = else_block {
        if let ExprKind::Block(block, _) = &else_expr.kind {
            let mut else_map = map.clone();
            walk_block(cx, block, &mut else_map);
            // conservative merge
            map.retain(|hir_id, (method, args)| {
                if let Some((obtained_method, obtained_args)) = then_map.get(hir_id)
                    && let Some((obtained_method2, obtained_args2)) = else_map.get(hir_id)
                {
                    return obtained_method == method
                        && are_args_equal(cx, args, obtained_args)
                        && obtained_method2 == method
                        && are_args_equal(cx, args, obtained_args2);
                }
                false
            });
        } else {
            //"else if" or other node diff than block,
            // clear everything that the then branch might have changed
            map.retain(|hir_id, (method, args)| {
                if let Some((obtained_method, obtained_args)) = then_map.get(hir_id) {
                    return obtained_method == method && are_args_equal(cx, args, obtained_args);
                }
                false
            });
            check_expr(cx, else_expr, map);
        }
    } else {
        // no "else" branch
        map.retain(|hir_id, (method, args)| {
            if let Some((obtained_method, obtained_args)) = then_map.get(hir_id) {
                return obtained_method == method && are_args_equal(cx, args, obtained_args);
            }
            false
        });
    }
    None
}

// Checks assignments
fn check_assign<'tcx>(
    cx: &LateContext<'tcx>,
    left_value: &'tcx rustc_hir::Expr<'tcx>,
    right_value: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    if let Some(dst_hir_id) = path_to_local(left_value) {
        if !try_inherit_alias(right_value, dst_hir_id, map) {
            if let Some((symbol, args)) = check_expr(cx, right_value, map) {
                map.insert(dst_hir_id, (symbol, args));
            } else {
                invalidate_left_value(left_value, map);
            }
        }
    } else {
        check_expr(cx, right_value, map);
    }
    None
}

// Checks loops
fn check_loop<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx rustc_hir::Block<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    let mut loop_map = map.clone();
    walk_block(cx, block, &mut loop_map);

    // A state survives only if it existed before the loop and if it was left completely identical
    // inside the loop body.
    map.retain(|hir_id, (method, args)| {
        if let Some((obtained_method, obtained_args)) = loop_map.get(hir_id) {
            return obtained_method == method && are_args_equal(cx, args, obtained_args);
        }
        false
    });
    None
}

// Checks matches
fn check_match<'tcx>(
    cx: &LateContext<'tcx>,
    arms: &'tcx [rustc_hir::Arm<'tcx>],
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    if arms.is_empty() {
        return None;
    }

    let mut arm_maps = Vec::new();
    for arm in arms {
        let mut arm_map = map.clone();

        if let Some(guard) = arm.guard {
            check_expr(cx, guard, &mut arm_map);
        }

        check_expr(cx, arm.body, &mut arm_map);
        arm_maps.push(arm_map);
    }
    // A variable keeps its tracking state if it is present and identical across all match arms.
    map.retain(|hir_id, (method, args)| {
        arm_maps.iter().all(|arm_map| {
            if let Some((obtained_method, obtained_args)) = arm_map.get(hir_id) {
                return obtained_method == method && are_args_equal(cx, args, obtained_args);
            }
            false
        })
    });

    None
}

// Checks closures
fn check_closure<'tcx>(
    cx: &LateContext<'tcx>,
    closure: &'tcx rustc_hir::Closure<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &[rustc_hir::Expr<'_>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    // invalidate any variables captured mutably
    for capture in cx.typeck_results().closure_min_captures_flattened(closure.def_id) {
        if capture.mutability == Mutability::Mut {
            map.shift_remove(&capture.get_root_variable());
        }
    }
    None
}

// Auxiliary functions

// Removes a value from the tracked list
fn invalidate_left_value<'tcx>(
    expr: &'tcx rustc_hir::Expr<'tcx>,
    map: &mut FxIndexMap<HirId, (Symbol, &[rustc_hir::Expr<'_>])>,
) {
    if let Some(hir_id) = path_to_local(expr) {
        map.shift_remove(&hir_id);

        // if any entry that its args reference the variable
        map.retain(|_, (_, args)| !args.iter().any(|arg| path_to_local(arg) == Some(hir_id)));
    }
}

// This is a known list of idempotent functions, that can either be expanded or replaced by some
// primitive static analysis.
fn is_idempotent<'tcx>(cx: &LateContext<'tcx>, expr: &rustc_hir::Expr<'tcx>, name: Symbol) -> bool {
    if !matches!(
        name,
        sym::trim
            | sym::trim_start
            | sym::trim_end
            | sym::to_lowercase
            | sym::to_uppercase
            | sym::to_ascii_lowercase
            | sym::to_ascii_uppercase
            | sym::abs
            | sym::floor
            | sym::ceil
            | sym::round
            | sym::signum
            | sym::max
            | sym::min
            | sym::clamp
            | sym::to_vec
            | sym::and
            | sym::or
    ) {
        return false;
    }
    // check if the method comes from the stdlib
    if let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) {
        let crate_name = cx.tcx.crate_name(def_id.krate);
        crate_name == sym::std || crate_name == sym::alloc || crate_name == sym::core
    } else {
        false
    }
}

// Resolves an expression into its respective HirId
fn path_to_local(expr: &rustc_hir::Expr<'_>) -> Option<HirId> {
    if let ExprKind::Path(QPath::Resolved(None, path)) = &expr.kind
        && let Res::Local(hir_id) = path.res
    {
        Some(hir_id)
    } else {
        None
    }
}

// Checks if any mutable references are passed down a function and if so, remove
// those variables from the tracking Map
fn check_func_args<'tcx>(
    cx: &LateContext<'tcx>,
    args: &'tcx [rustc_hir::Expr<'tcx>],
    map: &mut FxIndexMap<HirId, (Symbol, &'tcx [rustc_hir::Expr<'tcx>])>,
) -> Option<(Symbol, &'tcx [rustc_hir::Expr<'tcx>])> {
    for arg in args {
        if let ExprKind::AddrOf(_, Mutability::Mut, inner) = &arg.kind
            && let Some(hir_id) = path_to_local(inner)
        {
            map.shift_remove(&hir_id);
        } else {
            check_expr(cx, arg, map);
        }
    }
    None
}

fn is_expr_safe_to_compare(expr: &rustc_hir::Expr<'_>) -> bool {
    match &expr.kind {
        ExprKind::Lit(_) | ExprKind::Path(_) => true,
        ExprKind::Unary(_, inner) => is_expr_safe_to_compare(inner),
        // to cover constructors just like Some(1), Ok(2), None (but only literals)
        ExprKind::Call(func, args) => {
            matches!(func.kind, ExprKind::Path(_)) && args.iter().all(is_expr_safe_to_compare)
        },
        _ => false,
    }
}

// In the case of some functions, they are idempotent as long as their arguments
// are used in the same manner
fn are_args_equal<'tcx>(
    cx: &LateContext<'tcx>,
    args1: &'tcx [rustc_hir::Expr<'tcx>],
    args2: &'tcx [rustc_hir::Expr<'tcx>],
) -> bool {
    for (arg1, arg2) in args1.iter().zip(args2.iter()) {
        let const_context = ConstEvalCtxt::new(cx);
        if let Some(const_1) = const_context.eval(arg1)
            && let Some(const_2) = const_context.eval(arg2)
        {
            if const_1 != const_2 {
                return false;
            }
        } else if let Some(id1) = path_to_local(arg1)
            && let Some(id2) = path_to_local(arg2)
        {
            if id1 != id2 {
                return false;
            }
        } else if is_expr_safe_to_compare(arg1) && is_expr_safe_to_compare(arg2) {
            if !clippy_utils::SpanlessEq::new(cx).eq_expr(arg1.span.ctxt(), arg1, arg2) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn try_inherit_alias<'a>(
    expr: &rustc_hir::Expr<'_>,
    pat_hir_id: HirId,
    map: &mut FxIndexMap<HirId, (Symbol, &'a [rustc_hir::Expr<'a>])>,
) -> bool {
    if let Some(src_hir_id) = path_to_local(expr)
        && let Some(&(symbol, args)) = map.get(&src_hir_id)
    {
        map.insert(pat_hir_id, (symbol, args));
        true
    } else {
        false
    }
}

fn strip_parens_and_temps<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>) -> &'tcx rustc_hir::Expr<'tcx> {
    let mut e = expr;
    loop {
        match &e.kind {
            ExprKind::DropTemps(inner) => e = inner,
            ExprKind::Block(block, _) if block.stmts.is_empty() => {
                if let Some(tail) = block.expr {
                    e = tail;
                } else {
                    break;
                }
            },
            _ => break,
        }
    }
    e
}
