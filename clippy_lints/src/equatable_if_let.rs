use clippy_utils::source::{snippet_with_applicability, snippet_with_context};
use clippy_utils::ty::implements_trait;
use clippy_utils::{diagnostics::span_lint_and_sugg, higher::MatchesExpn};
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{
    def::{DefKind, Res},
    Arm, Expr, ExprKind, Pat, PatKind, QPath,
};
use rustc_lint::{LateContext, LateLintPass, Lint};
use rustc_middle::ty::{Adt, Ty};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{Span, SyntaxContext};

use crate::utils::conf::EquatablePatternLevel;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `if let <pat> = <expr>` (and `while let` and similars) that can be expressed
    /// using `if <expr> == <pat>`.
    ///
    /// ### Why is this bad?
    ///
    /// * It reads better and has less cognitive load because equality won't cause binding.
    /// * It is a [Yoda condition](https://en.wikipedia.org/wiki/Yoda_conditions). Yoda conditions are widely
    /// criticized for increasing the cognitive load of reading the code.
    /// * Equality is a simple bool expression and can be merged with `&&` and `||` and
    /// reuse if blocks
    ///
    /// ### Example
    /// ```rust,ignore
    /// if let Some(2) = x {
    ///     do_thing();
    /// }
    /// ```
    /// Should be written
    /// ```rust,ignore
    /// if x == Some(2) {
    ///     do_thing();
    /// }
    /// ```
    pub EQUATABLE_IF_LET,
    pedantic,
    "using if let instead of if with a equality condition"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `matches!(<expr>, <pat>)` that can be expressed
    /// using `<expr> == <pat>`.
    ///
    /// ### Why is this bad?
    ///
    /// It is less concise and less clear.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let condition = matches!(x, Some(2));
    /// ```
    /// Should be written
    /// ```rust,ignore
    /// let condition = x == Some(2);
    /// ```
    pub EQUATABLE_MATCHES,
    pedantic,
    "using `matches!` instead of equality"
}

pub struct PatternEquality {
    level: EquatablePatternLevel,
}

impl PatternEquality {
    pub fn new(level: EquatablePatternLevel) -> PatternEquality {
        PatternEquality { level }
    }
}

impl_lint_pass!(PatternEquality => [EQUATABLE_IF_LET, EQUATABLE_MATCHES]);

fn equatable_pattern(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    fn array_rec(cx: &LateContext<'_>, pats: &[Pat<'_>]) -> bool {
        pats.iter().all(|x| equatable_pattern(cx, x))
    }
    fn is_derived(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
        let ty = cx.typeck_results().pat_ty(pat);
        if let Some(def_id) = cx.tcx.lang_items().structural_peq_trait() {
            implements_trait(cx, ty, def_id, &[ty.into()])
        } else {
            false
        }
    }
    match &pat.kind {
        PatKind::Slice(a, None, []) => array_rec(cx, a),
        PatKind::Struct(_, a, etc) => !etc && is_derived(cx, pat) && a.iter().all(|x| equatable_pattern(cx, x.pat)),
        PatKind::Tuple(a, etc) => !etc.is_some() && array_rec(cx, a),
        PatKind::TupleStruct(_, a, etc) => !etc.is_some() && is_derived(cx, pat) && array_rec(cx, a),
        PatKind::Ref(x, _) | PatKind::Box(x) => equatable_pattern(cx, x),
        PatKind::Path(QPath::Resolved(_, b)) => match b.res {
            Res::Def(DefKind::Const, _) => true,
            _ => is_derived(cx, pat),
        },
        PatKind::Path(_) => is_derived(cx, pat),
        PatKind::Lit(_) => true,
        PatKind::Slice(..) | PatKind::Range(..) | PatKind::Binding(..) | PatKind::Wild | PatKind::Or(_) => false,
    }
}

fn is_partial_eq(cx: &LateContext<'tcx>, t1: Ty<'tcx>, t2: Ty<'tcx>) -> bool {
    if let Some(def_id) = cx.tcx.lang_items().eq_trait() {
        implements_trait(cx, t1, def_id, &[t2.into()])
    } else {
        false
    }
}

fn pat_to_string(
    cx: &LateContext<'tcx>,
    app: &mut Applicability,
    pat: &Pat<'_>,
    goal: Ty<'_>,
    ctxt: SyntaxContext,
) -> Option<String> {
    fn inner(
        cx: &LateContext<'tcx>,
        app: &mut Applicability,
        pat: &Pat<'_>,
        goal: Ty<'_>,
        r: &mut String,
        ctxt: SyntaxContext,
    ) -> bool {
        let ty = cx.typeck_results().pat_ty(pat);
        if ty == goal {
            match &pat.kind {
                PatKind::TupleStruct(q, ..) | PatKind::Struct(q, ..) => {
                    let (adt_def, generic_args) = if let Adt(x, y) = ty.kind() {
                        (x, y)
                    } else {
                        return false; // shouldn't happen
                    };
                    let path = if let QPath::Resolved(.., p) = q {
                        p
                    } else {
                        return false; // give up
                    };
                    let var = adt_def.variant_of_res(path.res);
                    match &pat.kind {
                        PatKind::TupleStruct(_, params, _) => {
                            *r += &*snippet_with_applicability(cx, path.span, "..", app);
                            *r += "(";
                            for (i, (p, f)) in params.iter().zip(var.fields.iter()).enumerate() {
                                if i != 0 {
                                    *r += ", ";
                                }
                                inner(cx, app, p, f.ty(cx.tcx, generic_args), r, ctxt);
                            }
                            *r += ")";
                        },
                        PatKind::Struct(_, fields, _) => {
                            *r += &*snippet_with_applicability(cx, path.span, "..", app);
                            *r += " { ";
                            for (i, p) in fields.iter().enumerate() {
                                if i != 0 {
                                    *r += ", ";
                                }
                                *r += &*snippet_with_applicability(cx, p.ident.span, "..", app);
                                *r += ": ";
                                if let Some(x) = var.fields.iter().find(|f| f.ident == p.ident) {
                                    inner(cx, app, p.pat, x.ty(cx.tcx, generic_args), r, ctxt);
                                } else {
                                    return false; // won't happen
                                }
                            }
                            *r += " }";
                        },
                        _ => return false, // won't happen
                    }
                },
                _ => {
                    *r += &*snippet_with_context(cx, pat.span, ctxt, "..", app).0;
                },
            }
            return true;
        }
        if goal.is_ref() {
            if let Some(tam) = goal.builtin_deref(true) {
                *r += "&";
                return inner(cx, app, pat, tam.ty, r, ctxt);
            }
        }
        false
    }
    let mut r = "".to_string();
    if let PatKind::Struct(..) = pat.kind {
        r += "(";
    }
    let success = inner(cx, app, pat, goal, &mut r, ctxt);
    if let PatKind::Struct(..) = pat.kind {
        r += ")";
    }
    if !success {
        return None;
    }
    Some(r)
}

fn level_contains(level: EquatablePatternLevel, pat: &Pat<'_>) -> bool {
    match level {
        EquatablePatternLevel::Primitive => matches!(pat.kind, PatKind::Lit(_)),
        EquatablePatternLevel::Simple => matches!(pat.kind, PatKind::Lit(_) | PatKind::Path(_)),
        EquatablePatternLevel::All => true,
    }
}

fn emit_lint(
    cx: &LateContext<'tcx>,
    pat: &Pat<'_>,
    exp: &Expr<'_>,
    ctxt: SyntaxContext,
    span: Span,
    lint: &'static Lint,
    level: EquatablePatternLevel,
) {
    if_chain! {
        if equatable_pattern(cx, pat);
        if level_contains(level, pat);
        let exp_ty = cx.typeck_results().expr_ty(exp);
        if is_partial_eq(cx, exp_ty, exp_ty);
        let mut app = Applicability::MachineApplicable;
        if let Some(pat_str) = pat_to_string(cx, &mut app, pat, exp_ty, ctxt);
        then {
            let exp_str = snippet_with_context(cx, exp.span, ctxt, "..", &mut app).0;
            span_lint_and_sugg(
                cx,
                lint,
                span,
                "this pattern matching can be expressed using equality",
                "try",
                format!(
                    "{} == {}",
                    exp_str,
                    pat_str,
                ),
                app,
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for PatternEquality {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::Let(pat, exp, _) = expr.kind {
            emit_lint(cx, pat, exp, expr.span.ctxt(), expr.span, EQUATABLE_IF_LET, self.level);
        }
        if let Some(MatchesExpn {
            call_site,
            arm: Arm { pat, guard: None, .. },
            exp,
        }) = MatchesExpn::parse(expr)
        {
            emit_lint(cx, pat, exp, expr.span.ctxt(), call_site, EQUATABLE_MATCHES, self.level);
        }
    }
}
