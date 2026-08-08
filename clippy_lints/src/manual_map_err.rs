use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher::IfLetOrMatch;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _, MaybeResPath as _};
use clippy_utils::source::{indent_of, reindent_multiline, snippet_with_context};
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::{expr_type_is_certain, implements_trait, is_copy};
use clippy_utils::usage::local_used_after_expr;
use clippy_utils::{
    CaptureKind, can_move_expr_to_closure, fn_def_id_with_node_args, is_else_clause, is_in_const_context,
    is_lint_allowed, msrvs, peel_blocks, peel_blocks_with_stmt, span_contains_cfg, span_contains_comment, sym,
};
use rustc_ast::BindingMode;
use rustc_errors::Applicability;
use rustc_hir::LangItem::{ResultErr, ResultOk};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Arm, ByRef, Expr, ExprKind, HirId, MatchSource, Node, Pat, PatKind, QPath, Stmt, StmtKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Span;

use crate::question_mark::QuestionMark;
use crate::question_mark_used::QUESTION_MARK_USED;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `match` and `if let` expressions on a `Result` whose error arm
    /// unconditionally returns a *different* error, and which can therefore be
    /// written with `map`, `map_err` and the `?` operator.
    ///
    /// ### Why is this bad?
    /// The combinator form is shorter, keeps the happy path unindented, and makes
    /// it obvious that the error arm only translates the error rather than doing
    /// arbitrary work.
    ///
    /// ### Known problems
    /// When the error value is discarded, the suggestion produces a
    /// `map_err(|_| ..)` closure, which [`map_err_ignore`] flags in turn. The two
    /// lints disagree by design: this one is about the control flow, that one is
    /// about throwing the source error away.
    ///
    /// [`map_err_ignore`]: https://rust-lang.github.io/rust-clippy/master/index.html#map_err_ignore
    ///
    /// ### Example
    /// ```
    /// # #[derive(Debug)] struct Error;
    /// # fn parse(s: &str) -> Result<u32, std::num::ParseIntError> { s.parse() }
    /// fn f(s: &str) -> Result<Option<u32>, Error> {
    ///     let value = match parse(s) {
    ///         Ok(v) => Some(v + 1),
    ///         Err(_) => return Err(Error),
    ///     };
    ///     Ok(value)
    /// }
    /// ```
    /// Use instead:
    /// ```
    /// # #[derive(Debug)] struct Error;
    /// # fn parse(s: &str) -> Result<u32, std::num::ParseIntError> { s.parse() }
    /// fn f(s: &str) -> Result<Option<u32>, Error> {
    ///     let value = parse(s).map(|v| Some(v + 1)).map_err(|_| Error)?;
    ///     Ok(value)
    /// }
    /// ```
    #[clippy::version = "1.99.0"]
    pub MANUAL_MAP_ERR,
    pedantic,
    "manual implementation of `map_err` combined with the `?` operator"
}

/// The `Ok(..)` arm of the match: the binding it introduces and the body it evaluates to.
struct OkArm<'tcx> {
    binding: HirId,
    binding_span: Span,
    body: &'tcx Expr<'tcx>,
}

/// What the `Err(..)` pattern binds, which becomes the `map_err` closure parameter.
enum ErrPat {
    Binding(HirId, Span),
    Wild,
}

/// The `Err(..)` arm of the match: what it binds, the returned `Err(..)` call, and the error value
/// inside it.
struct ErrArm<'tcx> {
    pat: ErrPat,
    err_call: &'tcx Expr<'tcx>,
    err_expr: &'tcx Expr<'tcx>,
}

/// Checks that `pat` is `Ok(binding)` with a plain by-value binding and no subpattern.
fn parse_ok_pat(cx: &LateContext<'_>, pat: &Pat<'_>) -> Option<(HirId, Span)> {
    if let PatKind::TupleStruct(qpath, [inner], ddpos) = &pat.kind
        && ddpos.as_opt_usize().is_none()
        && cx
            .qpath_res(qpath, pat.hir_id)
            .ctor_parent(cx)
            .is_lang_item(cx, ResultOk)
        && let PatKind::Binding(BindingMode(ByRef::No, _), binding, _, None) = inner.kind
    {
        Some((binding, inner.span))
    } else {
        None
    }
}

/// Checks that `pat` is `Err(binding)` or `Err(_)`, both by value.
fn parse_err_pat(cx: &LateContext<'_>, pat: &Pat<'_>) -> Option<ErrPat> {
    if let PatKind::TupleStruct(qpath, [inner], ddpos) = &pat.kind
        && ddpos.as_opt_usize().is_none()
        && cx
            .qpath_res(qpath, pat.hir_id)
            .ctor_parent(cx)
            .is_lang_item(cx, ResultErr)
    {
        match inner.kind {
            PatKind::Binding(BindingMode(ByRef::No, _), binding, _, None) => Some(ErrPat::Binding(binding, inner.span)),
            PatKind::Wild => Some(ErrPat::Wild),
            _ => None,
        }
    } else {
        None
    }
}

/// Checks that `body` is `return Err(<err_expr>)`, returning the `Err(..)` call and `<err_expr>`.
/// Uses `peel_blocks_with_stmt` so that the common `Err(_) => { return Err(e); }` form, where the
/// `return` is a statement rather than a tail expression, is recognised too.
fn parse_return_err<'tcx>(
    cx: &LateContext<'_>,
    body: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    let ret = peel_blocks_with_stmt(body);
    // A macro such as `anyhow::bail!` expands to `return Err(..)`. Rewriting it would put the
    // `return` inside a `map_err` closure, where it would return from the closure instead of the
    // function, so only an explicit `return` written at this call site can be linted.
    if !ret.span.from_expansion()
        && let ExprKind::Ret(Some(returned)) = ret.kind
        && let ExprKind::Call(err_ctor, [err_expr]) = returned.kind
        && err_ctor.res(cx).ctor_parent(cx).is_lang_item(cx, ResultErr)
    {
        Some((returned, err_expr))
    } else {
        None
    }
}

/// Checks if `expr` is `val` or `val.into()`. Those cases belong to `question_mark`, which
/// suggests a plain `?`, so this lint must not fire on them.
fn is_local_or_local_into(cx: &LateContext<'_>, expr: &Expr<'_>, val: HirId) -> bool {
    let is_into_call = fn_def_id_with_node_args(cx, expr)
        .and_then(|(fn_def_id, _)| cx.tcx.trait_of_assoc(fn_def_id))
        .is_some_and(|trait_def_id| cx.tcx.is_diagnostic_item(sym::Into, trait_def_id));
    match expr.kind {
        ExprKind::MethodCall(_, recv, [], _) | ExprKind::Call(_, [recv]) => {
            is_into_call && recv.res_local_id() == Some(val)
        },
        _ => expr.res_local_id() == Some(val),
    }
}

/// In `return Err(e)`, the type of `e` is pinned to the function's error type, and that pin is what
/// picks `Self` for a call to a trait associated function such as `de::Error::invalid_type(..)`, or
/// the output type of a generic trait method such as `.into()`. Behind `?` the pin is gone: `?`
/// inserts a `From` conversion, so the closure's error type only has to satisfy `From<_>` and the
/// call becomes ambiguous (E0790/E0283).
fn err_type_needs_return_context(cx: &LateContext<'_>, err_expr: &Expr<'_>) -> bool {
    let via_trait = match err_expr.kind {
        // Only a call naming the trait itself (`Trait::assoc(..)`) leaves `Self` to inference.
        // `Ty::assoc(..)` and `<Ty as Trait>::assoc(..)` spell it out.
        ExprKind::Call(callee, _) => {
            if let ExprKind::Path(QPath::Resolved(None, path)) = callee.kind
                && let Res::Def(DefKind::AssocFn, def_id) = path.res
            {
                cx.tcx.trait_of_assoc(def_id).is_some()
            } else {
                false
            }
        },
        // A trait method with a generic output, `.into()` above all. This is conservative: some
        // `.into()` calls would still infer behind `?` when only one `From` impl fits, but which
        // ones cannot be told from here.
        ExprKind::MethodCall(..) => cx
            .typeck_results()
            .type_dependent_def_id(err_expr.hir_id)
            .and_then(|def_id| cx.tcx.trait_of_assoc(def_id))
            .is_some(),
        _ => false,
    };
    via_trait && !expr_type_is_certain(cx, err_expr)
}

/// `?` ends the suggestion with a `From` conversion to the function's error type, so a `From` impl
/// must exist from the error expression's own type. `return Err(e)` was not so constrained: there
/// `e` could also rely on an unsize coercion, e.g. `Box<ConcreteError>` returned from a function
/// whose error type is `Box<dyn Error>` with no `From` impl to bridge them.
fn from_conversion_exists<'tcx>(cx: &LateContext<'tcx>, err_call: &Expr<'tcx>, err_expr: &Expr<'tcx>) -> bool {
    let err_ty = cx.typeck_results().expr_ty(err_expr);
    let ty::Adt(adt, args) = cx.typeck_results().expr_ty(err_call).kind() else {
        return false;
    };
    cx.tcx.is_diagnostic_item(sym::Result, adt.did())
        && cx
            .tcx
            .get_diagnostic_item(sym::From)
            .is_some_and(|from_trait| implements_trait(cx, args.type_at(1), from_trait, &[err_ty.into()]))
}

/// `.map()` and `.map_err()` take the scrutinee by value, so it must be an owned `Result` with no
/// adjustments applied to it.
fn is_owned_result(cx: &LateContext<'_>, scrutinee: &Expr<'_>) -> bool {
    cx.typeck_results().expr_adjustments(scrutinee).is_empty()
        && matches!(cx.typeck_results().expr_ty(scrutinee).kind(), ty::Adt(adt, _)
            if cx.tcx.is_diagnostic_item(sym::Result, adt.did()))
}

/// Both arm bodies are moved into separate closures, which is only sound if each body can become a
/// closure at all, and if the moves the closures perform are not observable elsewhere.
fn bodies_can_become_closures<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    ok_body: &'tcx Expr<'tcx>,
    err_expr: &'tcx Expr<'tcx>,
) -> bool {
    let Some(ok_captures) = can_move_expr_to_closure(cx, ok_body) else {
        return false;
    };
    let Some(err_captures) = can_move_expr_to_closure(cx, err_expr) else {
        return false;
    };

    let moves = |kind: &CaptureKind| matches!(kind, CaptureKind::Value | CaptureKind::Use);

    // Only one arm of a `match` runs, so a local may be consumed by both arms. Two closures would
    // both have to own it.
    if ok_captures
        .iter()
        .any(|(id, kind)| moves(kind) && err_captures.get(id).is_some_and(moves))
    {
        return false;
    }

    // Each arm consumed a local only on the branch that was taken, and that branch then diverged.
    // A closure consumes it unconditionally, so anything still needed afterwards must not be moved.
    !ok_captures.iter().chain(err_captures.iter()).any(|(&id, kind)| {
        moves(kind) && !is_copy(cx, cx.typeck_results().node_type(id)) && local_used_after_expr(cx, id, expr)
    })
}

fn parse_arms<'tcx>(
    cx: &LateContext<'tcx>,
    arm1: &'tcx Arm<'tcx>,
    arm2: &'tcx Arm<'tcx>,
) -> Option<(OkArm<'tcx>, ErrArm<'tcx>)> {
    if arm1.guard.is_some() || arm2.guard.is_some() {
        return None;
    }

    // Accept the arms in either order.
    let (ok_arm, err_arm) = if parse_ok_pat(cx, arm1.pat).is_some() {
        (arm1, arm2)
    } else {
        (arm2, arm1)
    };

    let (binding, binding_span) = parse_ok_pat(cx, ok_arm.pat)?;
    let err_pat = parse_err_pat(cx, err_arm.pat)?;
    let (err_call, err_expr) = parse_return_err(cx, err_arm.body)?;

    // `Err(e) => return Err(e)` and `Err(e) => return Err(e.into())` are `question_mark`'s job.
    if let ErrPat::Binding(err_id, _) = err_pat
        && is_local_or_local_into(cx, err_expr, err_id)
    {
        return None;
    }

    Some((
        OkArm {
            binding,
            binding_span,
            body: peel_blocks(ok_arm.body),
        },
        ErrArm {
            pat: err_pat,
            err_call,
            err_expr,
        },
    ))
}

/// Destructures either supported shape into a scrutinee and the two arms.
fn parse_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, OkArm<'tcx>, ErrArm<'tcx>)> {
    match IfLetOrMatch::parse(cx, expr)? {
        IfLetOrMatch::Match(scrutinee, [arm1, arm2], MatchSource::Normal | MatchSource::Postfix) => {
            let (ok_arm, err_arm) = parse_arms(cx, arm1, arm2)?;
            Some((scrutinee, ok_arm, err_arm))
        },
        IfLetOrMatch::IfLet(scrutinee, let_pat, if_then, Some(if_else), _) => {
            let (binding, binding_span) = parse_ok_pat(cx, let_pat)?;
            let (err_call, err_expr) = parse_return_err(cx, if_else)?;
            Some((
                scrutinee,
                OkArm {
                    binding,
                    binding_span,
                    body: peel_blocks(if_then),
                },
                // `if let Ok(v) = r { .. } else { return Err(e) }` never binds the error.
                ErrArm {
                    pat: ErrPat::Wild,
                    err_call,
                    err_expr,
                },
            ))
        },
        _ => None,
    }
}

impl QuestionMark {
    /// Everything this lint checks, short of emitting. Also used by `manual_let_else` to decide
    /// whether to step aside.
    fn manual_map_err_applies<'tcx>(
        &self,
        cx: &LateContext<'tcx>,
        expr: &'tcx Expr<'tcx>,
    ) -> Option<(&'tcx Expr<'tcx>, OkArm<'tcx>, ErrArm<'tcx>)> {
        // Cheapest first: flag checks, then the HIR shape, then the queries, and only then the
        // source scan and the visitors.
        // `?` is unusable in these three contexts.
        if expr.span.from_expansion() || self.inside_try_block() || !self.msrv.meets(cx, msrvs::QUESTION_MARK_OPERATOR)
        {
            return None;
        }

        let (scrutinee, ok_arm, err_arm) = parse_expr(cx, expr)?;

        if is_in_const_context(cx)
            // Suggesting `?` is pointless for someone who has banned it.
            || !is_lint_allowed(cx, QUESTION_MARK_USED, expr.hir_id)
            || span_contains_cfg(cx, expr.span)
            || !is_owned_result(cx, scrutinee)
            || !bodies_can_become_closures(cx, expr, ok_arm.body, err_arm.err_expr)
            || err_type_needs_return_context(cx, err_arm.err_expr)
            || !from_conversion_exists(cx, err_arm.err_call, err_arm.err_expr)
        {
            return None;
        }

        Some((scrutinee, ok_arm, err_arm))
    }

    pub(crate) fn check_manual_map_err<'tcx>(&self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some((scrutinee, ok_arm, err_arm)) = self.manual_map_err_applies(cx, expr) {
            emit(cx, expr, scrutinee, &ok_arm, &err_arm);
        }
    }

    /// Whether `manual_map_err` will fire on `expr`, so that `manual_let_else` can defer to it
    /// rather than have both pedantic lints report the same statement.
    pub(crate) fn manual_map_err_will_lint<'tcx>(&self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
        self.manual_map_err_applies(cx, expr).is_some()
    }
}

fn emit<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    scrutinee: &'tcx Expr<'tcx>,
    ok_arm: &OkArm<'tcx>,
    err_arm: &ErrArm<'tcx>,
) {
    // Rewriting would drop any comment inside the removed arms, so downgrade the suggestion.
    let mut app = if span_contains_comment(cx, expr.span) {
        Applicability::MaybeIncorrect
    } else {
        Applicability::MachineApplicable
    };
    let ctxt = expr.span.ctxt();

    // The scrutinee already starts at the column the suggestion will occupy, so its continuation
    // lines are left alone. The arm bodies, on the other hand, are pulled out of the arms and so
    // need shifting from the arm indentation to the expression's.
    let indent = indent_of(cx, expr.span);
    let arm_snippet = |span, app: &mut Applicability| {
        reindent_multiline(&snippet_with_context(cx, span, ctxt, "..", app).0, true, indent)
    };

    let scrut = Sugg::hir_with_context(cx, scrutinee, ctxt, "..", &mut app).maybe_paren();

    // `Ok(v) => v` is the identity, so no `.map()` is needed.
    let map = if ok_arm.body.res_local_id() == Some(ok_arm.binding) {
        String::new()
    } else {
        format!(
            ".map(|{}| {})",
            arm_snippet(ok_arm.binding_span, &mut app),
            arm_snippet(ok_arm.body.span, &mut app)
        )
    };

    let err_binding = match err_arm.pat {
        ErrPat::Binding(_, span) => arm_snippet(span, &mut app),
        ErrPat::Wild => "_".to_string(),
    };
    let err_expr = arm_snippet(err_arm.err_expr.span, &mut app);

    // A `match`/`if let` used as a statement is a complete statement on its own, but `expr?` is
    // not, so it has to grow the semicolon the original did not need.
    let semi = if matches!(
        cx.tcx.parent_hir_node(expr.hir_id),
        Node::Stmt(Stmt {
            kind: StmtKind::Expr(_),
            ..
        })
    ) {
        ";"
    } else {
        ""
    };

    let sugg = format!("{scrut}{map}.map_err(|{err_binding}| {err_expr})?{semi}");

    // An `else` must be followed by a block, so an `if let` that is itself an else clause needs
    // its replacement wrapped rather than spliced in bare.
    let sugg = if is_else_clause(cx.tcx, expr) {
        reindent_multiline(&format!("{{\n    {sugg}\n}}"), true, indent)
    } else {
        sugg
    };

    span_lint_and_sugg(
        cx,
        MANUAL_MAP_ERR,
        expr.span,
        "this can be replaced with `map`, `map_err` and `?`",
        "try",
        sugg,
        app,
    );
}
