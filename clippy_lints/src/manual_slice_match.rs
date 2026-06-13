use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::source::{snippet_indent, snippet_opt};
use clippy_utils::sugg::Sugg;
use clippy_utils::{SpanlessEq, if_sequence, is_else_clause, is_in_const_context, sym};
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{BinOpKind, Block, Expr, ExprKind, QPath, UnOp};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::ty::adjustment::{Adjust, DerefAdjustKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::SyntaxContext;
use std::fmt::Write as _;

const MSG: &str = "`if` chain checking only the length can be rewritten with a `match` on a slice pattern";

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `if`/`else if` chains whose conditions only inspect the
    /// length of one and the same slice, `Vec` or array (via `.is_empty()` or
    /// `.len()` compared against an integer literal) and which can be rewritten
    /// as a `match` on a slice pattern.
    ///
    /// ### Why is this bad?
    /// `if` chains are not checked for exhaustiveness and the length checks tend
    /// to be repetitive. A `match` on a slice pattern is exhaustive and makes the
    /// handled shapes explicit, and the bound elements can be named instead of
    /// being indexed (which avoids potential out-of-bounds panics).
    ///
    /// ### Example
    /// ```no_run
    /// # let v: Vec<u32> = vec![];
    /// if v.is_empty() {
    ///     // ...
    /// } else if v.len() == 1 {
    ///     println!("{}", v[0]);
    /// } else {
    ///     // ...
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # let v: Vec<u32> = vec![];
    /// match v.as_slice() {
    ///     [] => { /* ... */ }
    ///     [single] => println!("{single}"),
    ///     _ => { /* ... */ }
    /// }
    /// ```
    #[clippy::version = "1.102.0"]
    pub MANUAL_SLICE_MATCH,
    pedantic,
    "`if` length-check chains that can be rewritten as a `match` on a slice pattern"
}

impl_lint_pass!(ManualSliceMatch => [MANUAL_SLICE_MATCH]);

pub struct ManualSliceMatch {
    msrv: Msrv,
    max_suggested_slice: u64,
}

impl ManualSliceMatch {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            msrv: conf.msrv.into(),
            max_suggested_slice: conf.max_suggested_slice_pattern_length,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualSliceMatch {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        // Only look at the top-most `if` in the chain.
        if is_else_clause(cx.tcx, expr) {
            return;
        }

        if is_in_const_context(cx) {
            return;
        }

        if !self.msrv.meets(cx, msrvs::SLICE_PATTERNS) {
            return;
        }

        let (conds, blocks) = if_sequence(expr);

        // Require at least two conditions and an explicit final `else`, so the
        // chain is a genuine partition over the collection's shape.
        if conds.len() < 2 || blocks.len() != conds.len() + 1 {
            return;
        }

        // Every condition must be a length/emptiness predicate over the *same*
        // receiver. The first condition fixes the receiver, the rest must match.
        let Some((recv, _, _)) = len_predicate(cx, conds[0]) else {
            return;
        };

        // Rewriting the per-branch `recv.len()` calls into a single `match recv { .. }`
        // changes how many times `recv` is evaluated. `SpanlessEq::deny_side_effects()`
        // still accepts e.g. overloaded indexing, so only accept receivers that are
        // plain places (locals, fields and built-in derefs of those).
        if !is_side_effect_free_place(cx, recv) {
            return;
        }

        let mut spanless_eq = SpanlessEq::new(cx).deny_side_effects();
        for cond in &conds[1..] {
            match len_predicate(cx, cond) {
                Some((other, _, _)) if spanless_eq.eq_expr(SyntaxContext::root(), recv, other) => {},
                _ => return,
            }
        }

        // The receiver has to be something we can match on as a slice.
        let ty = cx.typeck_results().expr_ty(recv).peel_refs();
        let Some(scrutinee) = slice_scrutinee(cx, ty, recv) else {
            return;
        };

        // Only lint when the full `match` can be built with concrete slice-pattern arms.
        // Chains using comparisons that cannot be expressed as a single set of patterns
        // (e.g. `len() < n` or `len() != n`), or needing patterns longer than
        // `max-suggested-slice-pattern-length`, are left alone.
        if let Some(sugg) = build_match(cx, &conds, &blocks, &scrutinee, expr.span, self.max_suggested_slice) {
            span_lint_and_sugg(
                cx,
                MANUAL_SLICE_MATCH,
                expr.span,
                MSG,
                "consider rewriting the `if` chain with a `match`",
                sugg,
                // The bodies are copied verbatim (still using indexing), and matching a
                // `Vec` via `as_slice()` holds a borrow across the arms, so a body that
                // moves out of the receiver would not compile. Leave it to the user.
                Applicability::MaybeIncorrect,
            );
        }
    }
}

/// Builds the full `match` replacement text with one arm per `if`/`else if`
/// branch plus a wildcard arm for the final `else`. Returns `None` if any
/// condition cannot be turned into a slice pattern, or if the earlier arms
/// already cover every length (the wildcard arm would then be unreachable).
fn build_match<'tcx>(
    cx: &LateContext<'tcx>,
    conds: &[&'tcx Expr<'tcx>],
    blocks: &[&'tcx Block<'tcx>],
    scrutinee: &str,
    span: rustc_span::Span,
    max_suggested_slice: u64,
) -> Option<String> {
    let indent = snippet_indent(cx, span).unwrap_or_default();
    let arm_indent = format!("{indent}    ");
    let last = conds.len() - 1;

    let mut arms = String::new();
    let mut exact_lengths = Vec::new();
    let mut open_min = None;
    for (i, cond) in conds.iter().enumerate() {
        let (_, op, n) = len_predicate(cx, cond)?;
        let pat = arm_pattern(op, n, i == last, max_suggested_slice)?;
        // Two arms matching the same fixed length would make the second one
        // unreachable; bail rather than emit a suggestion that warns.
        if let Some(len) = pat.exact_len {
            if exact_lengths.contains(&len) {
                return None;
            }
            exact_lengths.push(len);
        }
        if pat.min_len.is_some() {
            open_min = pat.min_len;
        }
        // Bodies are kept verbatim: reindenting them would also change the contents
        // of multi-line string literals.
        let body = snippet_opt(cx, blocks[i].span)?;
        let _ = writeln!(arms, "{arm_indent}{} => {body}", pat.text);
    }

    // `[] | [_] | .. | [_; k - 1]` together with `[_; k, ..]` match every length, so the
    // final wildcard arm would be unreachable (and trigger `unreachable_patterns`).
    if let Some(min) = open_min
        && (0..min).all(|len| exact_lengths.contains(&len))
    {
        return None;
    }

    let else_body = snippet_opt(cx, blocks[last + 1].span)?;
    let _ = writeln!(arms, "{arm_indent}_ => {else_body}");

    Some(format!("match {scrutinee} {{\n{arms}{indent}}}"))
}

struct ArmPat {
    text: String,
    /// `Some(n)` when the pattern matches exactly `n` elements.
    exact_len: Option<u128>,
    /// `Some(n)` when the pattern matches `n` or more elements.
    min_len: Option<u128>,
}

/// Maps a normalized `len <op> n` predicate to a slice pattern. Open-ended
/// patterns (`> n`, `>= n`) are only allowed in the final condition, where the
/// following wildcard arm keeps the `match` exhaustive without overlap.
///
/// Arms whose number of `_` placeholders would exceed `max_suggested_slice` are
/// rejected (returning `None`), so a predicate like `len() > 100` does not generate a
/// pattern with a hundred placeholders; the caller then falls back to a help-only
/// diagnostic instead of a concrete suggestion. The trailing `..` is not counted, to
/// match the `max-suggested-slice-pattern-length` configuration semantics.
fn arm_pattern(op: BinOpKind, n: u128, is_last: bool, max_suggested_slice: u64) -> Option<ArmPat> {
    let underscores = |k: u128| -> Option<String> {
        if k > u128::from(max_suggested_slice) {
            return None;
        }
        Some(vec!["_"; usize::try_from(k).ok()?].join(", "))
    };
    match op {
        // `len() == n` -> exactly `n` elements.
        BinOpKind::Eq => Some(ArmPat {
            text: format!("[{}]", underscores(n)?),
            exact_len: Some(n),
            min_len: None,
        }),
        // `len() > n` -> at least `n + 1` elements.
        BinOpKind::Gt if is_last => Some(ArmPat {
            text: format!("[{}, ..]", underscores(n + 1)?),
            exact_len: None,
            min_len: Some(n + 1),
        }),
        // `len() >= n` -> at least `n` elements (`n == 0` would match everything).
        BinOpKind::Ge if is_last && n >= 1 => Some(ArmPat {
            text: format!("[{}, ..]", underscores(n)?),
            exact_len: None,
            min_len: Some(n),
        }),
        _ => None,
    }
}

/// If `cond` is a length/emptiness predicate, returns its receiver together with
/// the comparison normalized to `len <op> n` form (the literal on the right) and
/// the literal value. `recv.is_empty()` is treated as `len == 0`.
fn len_predicate<'tcx>(cx: &LateContext<'tcx>, cond: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, BinOpKind, u128)> {
    match cond.kind {
        // `recv.is_empty()`
        ExprKind::MethodCall(path, recv, [], _)
            if path.ident.name == sym::is_empty && is_inherent_slice_or_vec_method(cx, cond) =>
        {
            Some((recv, BinOpKind::Eq, 0))
        },
        // `recv.len() <cmp> <int>` or `<int> <cmp> recv.len()`
        ExprKind::Binary(op, lhs, rhs) if is_len_cmp(op.node) => {
            if let Some(recv) = len_receiver(cx, lhs)
                && let Some(n) = int_lit_val(rhs)
            {
                Some((recv, op.node, n))
            } else if let Some(recv) = len_receiver(cx, rhs)
                && let Some(n) = int_lit_val(lhs)
            {
                // Flip the operator so the literal ends up on the right.
                Some((recv, flip_cmp(op.node), n))
            } else {
                None
            }
        },
        _ => None,
    }
}

/// Returns the receiver of a `recv.len()` call.
fn len_receiver<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(path, recv, [], _) = expr.kind
        && path.ident.name == sym::len
        && is_inherent_slice_or_vec_method(cx, expr)
    {
        Some(recv)
    } else {
        None
    }
}

/// Checks that the method call `expr` resolved to an inherent method of `[T]` or
/// `Vec<T>`, and not e.g. to a user trait method with the same name.
fn is_inherent_slice_or_vec_method(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let Some(impl_id) = cx.ty_based_def(expr).opt_parent(cx)
        && let DefKind::Impl { of_trait: false } = cx.tcx.def_kind(impl_id)
    {
        let impl_ty = cx.tcx.type_of(impl_id).instantiate_identity().skip_norm_wip();
        impl_ty.is_slice() || impl_ty.is_diag_item(cx, sym::Vec)
    } else {
        false
    }
}

/// Checks that evaluating `expr` cannot have side effects: it must be a local
/// variable, possibly followed by field accesses and built-in dereferences, with no
/// overloaded `Deref` involved.
fn is_side_effect_free_place(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let typeck = cx.typeck_results();
    if typeck
        .expr_adjustments(expr)
        .iter()
        .any(|adj| matches!(adj.kind, Adjust::Deref(DerefAdjustKind::Overloaded(_))))
    {
        return false;
    }
    match expr.kind {
        ExprKind::Path(QPath::Resolved(None, path)) => matches!(path.res, Res::Local(_)),
        ExprKind::Field(base, _) => is_side_effect_free_place(cx, base),
        ExprKind::Unary(UnOp::Deref, base) => typeck.expr_ty(base).is_ref() && is_side_effect_free_place(cx, base),
        _ => false,
    }
}

fn int_lit_val(expr: &Expr<'_>) -> Option<u128> {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Int(Pu128(n), _) = lit.node
    {
        Some(n)
    } else {
        None
    }
}

/// Swaps the operands of a comparison: `a <op> b` is equivalent to `b <flipped> a`.
fn flip_cmp(kind: BinOpKind) -> BinOpKind {
    match kind {
        BinOpKind::Lt => BinOpKind::Gt,
        BinOpKind::Le => BinOpKind::Ge,
        BinOpKind::Gt => BinOpKind::Lt,
        BinOpKind::Ge => BinOpKind::Le,
        other => other,
    }
}

fn is_len_cmp(kind: BinOpKind) -> bool {
    matches!(
        kind,
        BinOpKind::Eq | BinOpKind::Ne | BinOpKind::Lt | BinOpKind::Le | BinOpKind::Gt | BinOpKind::Ge
    )
}

/// Builds the `match` scrutinee text for a sliceable receiver, or `None` if the
/// receiver type cannot be matched on as a slice pattern.
fn slice_scrutinee<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, recv: &'tcx Expr<'tcx>) -> Option<String> {
    let sugg = Sugg::hir(cx, recv, "..");
    match ty.kind() {
        // Slices can be matched directly. Arrays are deliberately excluded: their
        // length is a compile-time constant, so a length-check chain over an array
        // is degenerate and a slice pattern of a different length would not even
        // type-check.
        ty::Slice(_) => Some(sugg.to_string()),
        // `Vec` needs an explicit conversion to a slice.
        _ if ty.is_diag_item(cx, sym::Vec) => Some(format!("{}.as_slice()", sugg.maybe_paren())),
        _ => None,
    }
}
