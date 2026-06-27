use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher::If;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::implements_trait;
use clippy_utils::{eq_expr_value, is_in_const_context, peel_blocks, peel_blocks_with_stmt, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for a single `if`/`else` (or guarding `if`) that picks the greater of
    /// two values, where [`Ord::max`] would be clearer.
    ///
    /// ### Why is this bad?
    /// `a.max(b)` is shorter, has no branch, and states the intent directly. Unlike
    /// [`MANUAL_CLAMP`](https://rust-lang.github.io/rust-clippy/master/index.html#manual_clamp),
    /// which only triggers when both a lower *and* an upper bound are applied, this lint
    /// catches the common single-sided "floor" case.
    ///
    /// ### Known problems
    /// On a tie the two forms can return different operands: `Ord::max` returns the
    /// *second* argument when the operands compare equal, whereas `if a < b { b } else { a }`
    /// returns the *first* (`a`). For types whose `Eq`-equal values are observationally
    /// distinct (e.g. ordered by a key field), the rewrite changes which value is selected,
    /// so the suggestion is `MaybeIncorrect`.
    ///
    /// ### Example
    /// ```no_run
    /// # let (a, b) = (1, 2);
    /// let _ = if a < b { b } else { a };
    ///
    /// let mut cores = a;
    /// if cores < b {
    ///     cores = b;
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # let (a, b) = (1, 2);
    /// let _ = a.max(b);
    ///
    /// let mut cores = a;
    /// cores = cores.max(b);
    /// ```
    #[clippy::version = "1.98.0"]
    pub MANUAL_MAX,
    complexity,
    "an `if`/`else` that could be written as a call to `Ord::max`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for a single `if`/`else` (or guarding `if`) that picks the lesser of
    /// two values, where [`Ord::min`] would be clearer.
    ///
    /// ### Why is this bad?
    /// `a.min(b)` is shorter, has no branch, and states the intent directly. Unlike
    /// [`MANUAL_CLAMP`](https://rust-lang.github.io/rust-clippy/master/index.html#manual_clamp),
    /// which only triggers when both a lower *and* an upper bound are applied, this lint
    /// catches the common single-sided "ceiling" case.
    ///
    /// ### Example
    /// ```no_run
    /// # let (a, b) = (1, 2);
    /// let _ = if a > b { b } else { a };
    ///
    /// let mut cores = a;
    /// if cores > b {
    ///     cores = b;
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # let (a, b) = (1, 2);
    /// let _ = a.min(b);
    ///
    /// let mut cores = a;
    /// cores = cores.min(b);
    /// ```
    #[clippy::version = "1.98.0"]
    pub MANUAL_MIN,
    complexity,
    "an `if`/`else` that could be written as a call to `Ord::min`"
}

impl_lint_pass!(ManualMax => [MANUAL_MAX, MANUAL_MIN]);

pub struct ManualMax {
    msrv: Msrv,
}

impl ManualMax {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

#[derive(Clone, Copy)]
enum MinMax {
    Max,
    Min,
}

impl MinMax {
    fn method(self) -> &'static str {
        match self {
            MinMax::Max => "max",
            MinMax::Min => "min",
        }
    }

    fn lint(self) -> &'static rustc_lint::Lint {
        match self {
            MinMax::Max => MANUAL_MAX,
            MinMax::Min => MANUAL_MIN,
        }
    }
}

/// `lhs OP rhs` where `OP` is a comparison; used to reason about which operand the
/// condition selects when it holds.
struct Cmp<'tcx> {
    op: BinOpKind,
    lhs: &'tcx Expr<'tcx>,
    rhs: &'tcx Expr<'tcx>,
}

impl<'tcx> Cmp<'tcx> {
    fn new(cond: &'tcx Expr<'tcx>) -> Option<Self> {
        if let ExprKind::Binary(op, lhs, rhs) = peel_blocks(cond).kind
            && matches!(op.node, BinOpKind::Lt | BinOpKind::Le | BinOpKind::Gt | BinOpKind::Ge)
        {
            Some(Self { op: op.node, lhs, rhs })
        } else {
            None
        }
    }

    /// Returns the operator rewritten so that `var` sits on the left-hand side, i.e. the
    /// direction of the comparison as seen from `var`. Returns `None` if `var` is not one
    /// of the operands.
    fn orient(&self, cx: &LateContext<'tcx>, var: &Expr<'tcx>, ctxt: rustc_span::SyntaxContext) -> Option<BinOpKind> {
        if eq_expr_value(cx, ctxt, var, self.lhs) {
            Some(self.op)
        } else if eq_expr_value(cx, ctxt, var, self.rhs) {
            Some(flip(self.op))
        } else {
            None
        }
    }
}

fn flip(op: BinOpKind) -> BinOpKind {
    match op {
        BinOpKind::Lt => BinOpKind::Gt,
        BinOpKind::Le => BinOpKind::Ge,
        BinOpKind::Gt => BinOpKind::Lt,
        BinOpKind::Ge => BinOpKind::Le,
        other => other,
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualMax {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // `Ord::max`/`Ord::min` were stabilized in 1.21.0.
        if expr.span.from_expansion() || is_in_const_context(cx) || !self.msrv.meets(cx, msrvs::ORD_MAX_MIN) {
            return;
        }
        if let Some(If { cond, then, r#else }) = If::hir(expr)
            && let Some(cmp) = Cmp::new(cond)
            // Only `Ord` types: `f32`/`f64` implement `PartialOrd` only, and `a.max(b)`
            // differs from the branching form when an operand is `NaN`. Both operands must be
            // `Ord` (and hence the same type) for `lhs.max(rhs)` to type-check.
            && is_ord(cx, cmp.lhs)
            && is_ord(cx, cmp.rhs)
        {
            let ctxt = expr.span.ctxt();
            let found = match r#else {
                Some(r#else) => match_select(cx, &cmp, peel_blocks(then), peel_blocks(r#else), ctxt),
                None => match_guard(cx, &cmp, peel_blocks_with_stmt(then), ctxt),
            };
            if let Some((kind, recv, arg, assign_to)) = found {
                emit(cx, kind, expr, recv, arg, assign_to);
            }
        }
    }
}

/// Matches the value-returning form `if lhs OP rhs { x } else { y }`, where `{x, y}` are
/// the two operands in either order.
fn match_select<'tcx>(
    cx: &LateContext<'tcx>,
    cmp: &Cmp<'tcx>,
    then: &'tcx Expr<'tcx>,
    r#else: &'tcx Expr<'tcx>,
    ctxt: rustc_span::SyntaxContext,
) -> Option<(MinMax, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>, Option<&'tcx Expr<'tcx>>)> {
    let picks_lhs = eq_expr_value(cx, ctxt, then, cmp.lhs) && eq_expr_value(cx, ctxt, r#else, cmp.rhs);
    let picks_rhs = eq_expr_value(cx, ctxt, then, cmp.rhs) && eq_expr_value(cx, ctxt, r#else, cmp.lhs);
    // When the condition holds: `Lt`/`Le` means `lhs` is the smaller operand, `Gt`/`Ge`
    // means `lhs` is the larger one. Picking the larger operand => `max`, else `min`.
    let lhs_is_greater_when_true = matches!(cmp.op, BinOpKind::Gt | BinOpKind::Ge);
    let kind = match (picks_lhs, picks_rhs) {
        (true, false) if lhs_is_greater_when_true => MinMax::Max,
        (true, false) => MinMax::Min,
        (false, true) if lhs_is_greater_when_true => MinMax::Min,
        (false, true) => MinMax::Max,
        _ => return None,
    };
    Some((kind, cmp.lhs, cmp.rhs, None))
}

/// Matches the guarding form `if x OP bound { x = bound; }` (no `else`), equivalent to
/// `x = x.max(bound)` / `x = x.min(bound)`.
fn match_guard<'tcx>(
    cx: &LateContext<'tcx>,
    cmp: &Cmp<'tcx>,
    then: &'tcx Expr<'tcx>,
    ctxt: rustc_span::SyntaxContext,
) -> Option<(MinMax, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>, Option<&'tcx Expr<'tcx>>)> {
    if let ExprKind::Assign(target, value, _) = then.kind
        // The assigned value must be the bound that is compared against, and the assignment
        // target the clamped variable.
        && let Some(oriented) = cmp.orient(cx, target, ctxt)
        && (eq_expr_value(cx, ctxt, value, cmp.lhs) || eq_expr_value(cx, ctxt, value, cmp.rhs))
        && !eq_expr_value(cx, ctxt, value, target)
    {
        // `if x < bound { x = bound }` raises `x` to a floor => `max`;
        // `if x > bound { x = bound }` lowers `x` to a ceiling => `min`.
        let kind = match oriented {
            BinOpKind::Lt | BinOpKind::Le => MinMax::Max,
            BinOpKind::Gt | BinOpKind::Ge => MinMax::Min,
            _ => return None,
        };
        return Some((kind, target, value, Some(target)));
    }
    None
}

fn is_ord<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    cx.tcx
        .get_diagnostic_item(sym::Ord)
        .is_some_and(|ord| implements_trait(cx, ty, ord, &[]))
}

fn emit<'tcx>(
    cx: &LateContext<'tcx>,
    kind: MinMax,
    expr: &Expr<'tcx>,
    recv: &Expr<'tcx>,
    arg: &Expr<'tcx>,
    assign_to: Option<&Expr<'tcx>>,
) {
    let recv = Sugg::hir(cx, recv, "..").maybe_paren();
    let arg = Sugg::hir(cx, arg, "..");
    let call = format!("{recv}.{}({arg})", kind.method());
    let sugg = match assign_to {
        // The guard form replaces a whole `if` *statement*, so the assignment needs its
        // own terminating `;`; the value-returning form is an expression and must not.
        Some(target) => format!("{} = {call};", Sugg::hir(cx, target, "..")),
        None => call,
    };
    span_lint_and_sugg(
        cx,
        kind.lint(),
        expr.span,
        format!("this `if` expression is a manual `{}`", kind.method()),
        "replace with",
        sugg,
        // The branching form re-evaluates the selected operand, so operands with side
        // effects could behave differently.
        Applicability::MaybeIncorrect,
    );
}
