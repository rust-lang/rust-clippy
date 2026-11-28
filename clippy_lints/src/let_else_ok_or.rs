use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _};
use clippy_utils::source::snippet_with_context;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::is_copy;
use clippy_utils::{is_in_const_context, span_contains_cfg, span_contains_comment, sym};
use rustc_ast::{LitFloatType, LitIntType, LitKind};
use rustc_attr_ir::lang_items::LangItem::{OptionSome, ResultErr};
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::{BindingMode, Block, ByRef, Expr, ExprKind, HirId, LetStmt, Mutability, PatKind, Stmt, StmtKind};
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor, PlaceBase, PlaceWithHirId};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::mir::FakeReadCause;
use rustc_middle::ty::{BorrowKind, TyCtxt};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `let...else` statements that bind the contents of an `Option`
    /// and whose `else` branch just returns an `Err`.
    ///
    /// ### Why is this bad?
    /// `Option::ok_or` combined with the `?` operator expresses the same thing
    /// more concisely.
    ///
    /// ### Known problems
    /// The rewrite is only machine-applicable when the error value is a constant
    /// whose type does not depend on the surrounding context and which has no
    /// destructor (e.g. `Err("missing")` or `Err(())`). Otherwise `ok_or_else` is
    /// suggested, which may need adjustments: `?` converts the error through
    /// `From::from`, so the error type may no longer be inferred, and the closure
    /// may conflict with borrows held by the `Option`.
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
    #[clippy::version = "1.102.0"]
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
            ty: None,
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
            // `ok_or`/`ok_or_else` move the error value out of the place it comes from on the
            // `Some` path too, which changes when it is dropped (or breaks later uses of it).
            && !err_moves_local(cx, err_arg)
        {
            // The rewrite is only machine-applicable when the error value is a constant that:
            // - is safe to evaluate eagerly: no side effects, no destructor, no borrow that could overlap with the one
            //   held by the `Option` (the `else` branch only runs on `None`);
            // - has a type that does not depend on the expected type: `?` routes the value through `From::from`, so
            //   e.g. `"..".into()` or an unsuffixed integer literal would no longer be inferred;
            // - is not *coerced* to the function's error type (e.g. an unsizing coercion of `&[u8; N]` to `&[u8]`), as
            //   `From<&[u8; N]>` for `&[u8]` does not exist.
            let typeck = cx.typeck_results();
            let eager = eval_eagerly(cx, err_arg);
            let mut applicability =
                if eager == Eager::ContextFree && typeck.expr_ty(err_arg) == typeck.expr_ty_adjusted(err_arg) {
                    Applicability::MachineApplicable
                } else {
                    Applicability::MaybeIncorrect
                };
            let init_sugg = Sugg::hir_with_context(cx, init, stmt.span.ctxt(), "..", &mut applicability).maybe_paren();
            // Use the statement's context so an error value that is itself a macro call (e.g.
            // `format!(..)`) is rendered as written instead of leaking its expansion internals.
            let err_snippet = snippet_with_context(cx, err_arg.span, stmt.span.ctxt(), "..", &mut applicability).0;

            // `let...else` only evaluates the `else` branch on `None`, so keep that laziness by
            // suggesting `ok_or_else` unless evaluating the error value is known to be harmless.
            let method = if eager == Eager::No {
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Eager {
    /// The value must only be computed on `None`.
    No,
    /// The value is cheap, has no side effects and no destructor, but its type may depend on
    /// the expected type (e.g. an unsuffixed integer literal), or reading it may conflict with a
    /// borrow held by the `Option` (e.g. a `Copy` local).
    Ok,
    /// Same as `Ok`, and the value is a constant whose type does not depend on its context.
    ContextFree,
}

/// Checks whether `expr` can be evaluated eagerly (i.e. on the `Some` path too) without any
/// observable difference: a literal, `()`, a `const` item, a unit struct or unit variant, or a
/// `Copy` local, as long as its type has no destructor.
fn eval_eagerly<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Eager {
    let eager = match expr.kind {
        ExprKind::Lit(lit) => {
            if matches!(
                lit.node,
                LitKind::Int(_, LitIntType::Unsuffixed) | LitKind::Float(_, LitFloatType::Unsuffixed)
            ) {
                Eager::Ok
            } else {
                Eager::ContextFree
            }
        },
        ExprKind::Tup([]) => Eager::ContextFree,
        ExprKind::Path(ref qpath) => match cx.qpath_res(qpath, expr.hir_id) {
            Res::Def(DefKind::Const, _) => Eager::ContextFree,
            Res::Def(DefKind::Ctor(_, CtorKind::Const), id) => {
                if cx.tcx.generics_of(id).count() == 0 {
                    Eager::ContextFree
                } else {
                    Eager::Ok
                }
            },
            Res::Local(_) if is_copy(cx, cx.typeck_results().expr_ty(expr)) => Eager::Ok,
            _ => Eager::No,
        },
        _ => Eager::No,
    };
    // `ok_or` would build (and then drop) the value on the `Some` path too.
    if eager != Eager::No && cx.typeck_results().expr_ty(expr).needs_drop(cx.tcx, cx.typing_env()) {
        Eager::No
    } else {
        eager
    }
}

/// Returns `true` if evaluating `err_arg` moves a non-`Copy` local (or argument).
///
/// The `else` branch only runs on `None`, so the original code never touches the error value on the
/// `Some` path. `ok_or`/`ok_or_else`, by contrast, move the error value unconditionally: the moved
/// value is then dropped earlier than before (which is observable when it has a destructor), and
/// any later use of it would no longer compile. Values that are merely *borrowed* (e.g.
/// `Err(format!("{x}"))`) are fine.
fn err_moves_local<'tcx>(cx: &LateContext<'tcx>, err_arg: &'tcx Expr<'tcx>) -> bool {
    let mut delegate = MovedLocals { cx, moves_local: false };
    ExprUseVisitor::for_clippy(cx, err_arg.hir_id.owner.def_id, &mut delegate)
        .consume_expr(err_arg)
        .into_ok();
    delegate.moves_local
}

/// Collects the locals that are *moved* (not just borrowed or copied) by the visited expression.
struct MovedLocals<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    moves_local: bool,
}

impl<'tcx> Delegate<'tcx> for MovedLocals<'_, 'tcx> {
    fn consume(&mut self, place_with_id: &PlaceWithHirId<'tcx>, _: HirId) {
        // Only a move of a non-`Copy` place invalidates a later use; copies leave the original
        // readable, so they can never break the rewrite.
        if let PlaceBase::Local(_) = place_with_id.place.base
            && !is_copy(self.cx, place_with_id.place.ty())
        {
            self.moves_local = true;
        }
    }

    fn use_cloned(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn borrow(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId, _: BorrowKind) {}
    fn mutate(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}
    fn fake_read(&mut self, _: &PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {}
}
