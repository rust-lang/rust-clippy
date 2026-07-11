use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::eager_or_lazy::switch_to_lazy_eval;
use clippy_utils::res::{MaybeDef, MaybeQPath, MaybeResPath};
use clippy_utils::source::snippet_with_context;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::is_copy;
use clippy_utils::visitors::{Descend, for_each_expr};
use clippy_utils::{
    get_enclosing_loop_or_multi_call_closure, is_in_const_context, span_contains_cfg, span_contains_comment, sym,
};
use core::ops::ControlFlow;
use rustc_errors::Applicability;
use rustc_hir::LangItem::{OptionSome, ResultErr};
use rustc_hir::{
    BindingMode, Block, ByRef, Expr, ExprKind, HirId, HirIdSet, LetStmt, Mutability, PatKind, Stmt, StmtKind,
};
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor, PlaceBase, PlaceWithHirId};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::FakeReadCause;
use rustc_middle::ty::{BorrowKind, TyCtxt};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `let...else` statements that bind the contents of an `Option`
    /// and whose `else` branch just returns an `Err`.
    ///
    /// ### Why is this bad?
    /// `Option::ok_or` combined with the `?` operator expresses the same thing
    /// more concisely.
    ///
    /// ### Example
    /// ```no_run
    /// # fn parse(opt: Option<i32>) -> Result<i32, String> {
    /// let Some(value) = opt else {
    ///     return Err("missing value".to_string());
    /// };
    /// # Ok(value)
    /// # }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # fn parse(opt: Option<i32>) -> Result<i32, String> {
    /// let value = opt.ok_or_else(|| "missing value".to_string())?;
    /// # Ok(value)
    /// # }
    /// ```
    #[clippy::version = "1.98.0"]
    pub LET_ELSE_OK_OR,
    style,
    "`let...else` returning an `Err` that could be written with `Option::ok_or` and `?`"
}

impl_lint_pass!(LetElseOkOr => [LET_ELSE_OK_OR]);

pub struct LetElseOkOr {
    /// Whether the features needed to call `Option::ok_or`/`ok_or_else` and `?` in a `const`
    /// context (`const_option_ops`, `const_trait_impl` and `const_try`) are all enabled. When
    /// they are not, the suggestion would not compile inside a `const` item.
    const_ok_or_available: bool,
}

impl LetElseOkOr {
    pub fn new(tcx: TyCtxt<'_>, _conf: &'static Conf) -> Self {
        let features = tcx.features();
        Self {
            const_ok_or_available: features.enabled(sym::const_option_ops)
                && features.enabled(sym::const_trait_impl)
                && features.enabled(sym::const_try),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for LetElseOkOr {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        // `Option::ok_or`/`ok_or_else` and `?` are not usable in a `const` context unless the
        // relevant unstable features are enabled, so the suggestion would not compile there.
        if is_in_const_context(cx) && !self.const_ok_or_available {
            return;
        }
        if let StmtKind::Let(LetStmt {
            pat,
            init: Some(init),
            els: Some(els),
            ..
        }) = stmt.kind
            // Don't fire on code coming from a macro: the suggestion would not be applicable.
            && !stmt.span.from_expansion()
            // The pattern binds the payload of an `Option`, e.g. `Some(value)`.
            && let PatKind::TupleStruct(qpath, [inner_pat], _) = pat.kind
            && cx.qpath_res(&qpath, pat.hir_id).ctor_parent(cx).is_lang_item(cx, OptionSome)
            // ... to a single, by-value binding (skip `ref`/`mut ref` and nested patterns).
            && let PatKind::Binding(BindingMode(ByRef::No, mutability), _, ident, None) = inner_pat.kind
            // The initializer is really an owned `Option`, so `ok_or` is callable on it.
            && cx.typeck_results().expr_ty(init).is_diag_item(cx, sym::Option)
            // The `else` branch unconditionally does `return Err(<expr>)`.
            && let Some(err_arg) = err_return_arg(cx, els)
            // Don't drop comments or `#[cfg]`-ed code from the `else` branch.
            && !span_contains_comment(cx, els.span)
            && !span_contains_cfg(cx, els.span)
        {
            // `return Err(<expr>)` lets `<expr>` be *coerced* to the function's error type (e.g. an
            // unsizing coercion of `&[u8; N]` to `&[u8]`), whereas `?` routes the value through
            // `From::from`, which may have no matching impl (`From<&[u8; N]>` for `&[u8]` does not
            // exist). When such a type-changing coercion is applied the rewrite might not compile,
            // so only keep the suggestion machine-applicable when the error value already has the
            // function's error type (an identity `From`). Benign adjustments that don't change the
            // type (e.g. a reborrow) are fine, so compare the written type against the adjusted one
            // rather than counting them.
            let typeck = cx.typeck_results();
            let mut applicability = if typeck.expr_ty(err_arg) == typeck.expr_ty_adjusted(err_arg)
                // `ok_or`/`ok_or_else` move the error value unconditionally, whereas the `else`
                // branch only runs on `None`. If the error value moves out a place still used on
                // the `Some` path, the rewrite would reference a moved value and not compile, so
                // keep it out of `--fix`.
                && !err_moves_local_used_elsewhere(cx, err_arg)
            {
                Applicability::MachineApplicable
            } else {
                Applicability::MaybeIncorrect
            };
            let init_sugg = Sugg::hir_with_context(cx, init, stmt.span.ctxt(), "..", &mut applicability).maybe_paren();
            // Use the statement's context so an error value that is itself a macro call (e.g.
            // `format!(..)`) is rendered as written instead of leaking its expansion internals.
            let err_snippet = snippet_with_context(cx, err_arg.span, stmt.span.ctxt(), "..", &mut applicability).0;

            // `let...else` only evaluates the `else` branch on `None`, so keep that laziness
            // by suggesting `ok_or_else` whenever the error is more than a trivial value.
            let method = if switch_to_lazy_eval(cx, err_arg) {
                format!("ok_or_else(|| {err_snippet})")
            } else {
                format!("ok_or({err_snippet})")
            };

            let mut_str = if mutability == Mutability::Mut { "mut " } else { "" };

            span_lint_and_sugg(
                cx,
                LET_ELSE_OK_OR,
                stmt.span,
                "this `let...else` may be rewritten with `Option::ok_or` and `?`",
                "replace it with",
                format!("let {mut_str}{ident} = {init_sugg}.{method}?;"),
                applicability,
            );
        }
    }
}

/// If `block` unconditionally diverges through `return Err(<expr>)`, returns that `<expr>`.
fn err_return_arg<'tcx>(cx: &LateContext<'_>, block: &'tcx Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    let ret_expr = match block {
        Block {
            stmts: [],
            expr: Some(expr),
            ..
        } => expr,
        Block {
            stmts: [stmt],
            expr: None,
            ..
        } if let StmtKind::Semi(expr) = stmt.kind => expr,
        _ => return None,
    };

    if let ExprKind::Ret(Some(ret_value)) = ret_expr.kind
        && let ExprKind::Call(err_path, [err_arg]) = ret_value.kind
        // A macro-generated `return Err(..)` (e.g. `anyhow::bail!`) would make us reach into the
        // macro for the error value and produce a suggestion leaking its internals.
        && !ret_value.span.from_expansion()
        && err_path.res(cx).ctor_parent(cx).is_lang_item(cx, ResultErr)
    {
        Some(err_arg)
    } else {
        None
    }
}

/// Returns `true` if evaluating `err_arg` moves a non-`Copy` local (or argument) that the rewrite
/// would invalidate.
///
/// The `else` branch only runs on `None`, so the original code never touches the error value on the
/// `Some` path. `ok_or`/`ok_or_else`, by contrast, move the error value unconditionally, so if it
/// moves out a place that is read elsewhere — or that could be moved again on another loop
/// iteration — the rewrite would reference a moved value and fail to compile. Values that are
/// merely *borrowed* (e.g. `Err(format!("{x}"))`) are fine.
fn err_moves_local_used_elsewhere<'tcx>(cx: &LateContext<'tcx>, err_arg: &'tcx Expr<'tcx>) -> bool {
    let mut delegate = MovedLocals {
        cx,
        locals: HirIdSet::default(),
    };
    ExprUseVisitor::for_clippy(cx, err_arg.hir_id.owner.def_id, &mut delegate)
        .consume_expr(err_arg)
        .into_ok();
    if delegate.locals.is_empty() {
        return false;
    }

    // A move inside a loop (or a closure called more than once) can run repeatedly and move the
    // same place twice; the original `else` branch never does, as it only runs on `None`, which
    // diverges via `return`.
    if get_enclosing_loop_or_multi_call_closure(cx, err_arg).is_some() {
        return true;
    }

    // Otherwise the move only breaks compilation if one of the moved locals is read somewhere else
    // in the body (the `Some` path still relying on a value the rewrite has now moved). Reads inside
    // `err_arg` itself are the move we are rewriting, so they don't count. Walking the whole body
    // also covers function parameters, whose scope is the entire body rather than a single block.
    let body = cx.tcx.hir_body_owned_by(err_arg.hir_id.owner.def_id);
    for_each_expr(cx.tcx, body.value, |e| {
        if e.hir_id == err_arg.hir_id {
            ControlFlow::Continue(Descend::No)
        } else if e.res_local_id().is_some_and(|id| delegate.locals.contains(&id)) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(Descend::Yes)
        }
    })
    .is_some()
}

/// Collects the locals that are *moved* (not just borrowed or copied) by the visited expression.
struct MovedLocals<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    locals: HirIdSet,
}

impl<'tcx> Delegate<'tcx> for MovedLocals<'_, 'tcx> {
    fn consume(&mut self, place_with_id: &PlaceWithHirId<'tcx>, _: HirId) {
        // Only a move of a non-`Copy` place invalidates a later use; copies leave the original
        // readable, so they can never break the rewrite.
        if let PlaceBase::Local(local) = place_with_id.place.base
            && !is_copy(self.cx, place_with_id.place.ty())
        {
            self.locals.insert(local);
        }
    }

    fn use_cloned(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn borrow(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId, _: BorrowKind) {}
    fn mutate(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn fake_read(&mut self, _: &PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {}
}
