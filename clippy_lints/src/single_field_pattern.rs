#![allow(rustc::usage_of_ty_tykind)]

use clippy_utils::{diagnostics::span_lint, higher::IfLetOrMatch, higher::WhileLet};
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{symbol::Ident, MultiSpan};
use std::iter::once;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.59.0"]
    pub SINGLE_FIELD_PATTERN,
    style,
    "default lint description"
}
declare_lint_pass!(SingleFieldPattern => [SINGLE_FIELD_PATTERN]);

#[derive(PartialEq, Eq, Debug, Clone)]
enum Fields {
    Id(Ident),
    Index(usize),
    Unused,
}

impl Fields {
    // Todo - auto-fix - I'll need to add span and strings into SingleField
    fn lint(self, cx: &LateContext<'tcx>, span: impl Into<MultiSpan>) {
        span_lint(
            cx,
            SINGLE_FIELD_PATTERN,
            span,
            "pattern matching just to get a single field",
        );
    }
}

impl From<Ident> for Fields {
    fn from(id: Ident) -> Self {
        Fields::Id(id)
    }
}

impl From<usize> for Fields {
    fn from(id: usize) -> Self {
        Fields::Index(id)
    }
}

fn get_the_one<ID: Into<Fields>>(pat: impl Iterator<Item = (ID, &'hir Pat<'hir>)>) -> Option<Fields> {
    let mut iter = pat.filter(|&(_, pat)| !matches!(pat.kind, PatKind::Wild)).map(|t| t.0);
    let the_one = iter.next(); // TODO: this also eliminates blank patterns, e.g. Struct { .. } - as silly as it is
    if let None = iter.next() {
        the_one.map(|id| id.into()).or(Some(Fields::Unused))
    } else {
        None
    }
}

fn single_struct(pat: &PatKind<'hir>) -> Option<Fields> {
    if let PatKind::Struct(_, inner_pats, _) = pat {
        if let field @ Some(Fields::Id(_)) = get_the_one(inner_pats.iter().map(|field| (field.ident, field.pat))) {
            return field;
        }
    }
    None
}

fn single_tuple(pat: &PatKind<'hir>) -> Option<Fields> {
    if let PatKind::Tuple(inner_pats, leap) | PatKind::TupleStruct(_, inner_pats, leap) = pat {
        if let field @ Some(Fields::Index(index)) = get_the_one((*inner_pats).iter().enumerate()) {
            // Skip (.., x) - the meaning of let (.., x) = t and t.n are different
            if (*leap).map_or(true, |leap_index| leap_index > index) {
                return field;
            }
        }
    }
    None
}

fn single_slice(pat: &PatKind<'hir>) -> Option<Fields> {
    if let PatKind::Slice(before, dots, after) = pat {
        if let field @ Some(Fields::Index(_)) = get_the_one(before.iter().enumerate()) {
            if dots.is_none() || get_the_one(after.iter().enumerate()).is_none() {
                return field;
            }
        }
    }
    None
}

fn walk_until_single_field_leaf<'hir>(
    patterns: impl Iterator<Item = &'hir Pat<'hir>>,
    get_leaf: &impl Fn(&PatKind<'hir>) -> Option<Fields>,
) -> Option<Fields> {
    let mut fields = patterns
        .map(|pat| match &pat.kind {
            PatKind::Or(pats) => walk_until_single_field_leaf(pats.iter(), get_leaf),
            PatKind::Wild => Some(Fields::Unused),
            p => get_leaf(p),
        })
        .filter(|field| *field != Some(Fields::Unused));
    if let Some(the_one) = fields.next() {
        if fields.all(|other| other == the_one) {
            return the_one;
        }
    }
    None
}

fn find_single_pattern<'hir>(ty: &ty::TyKind<'_>, patterns: impl Iterator<Item = &'hir Pat<'hir>>) -> Option<Fields> {
    match ty {
        ty::TyKind::Adt(def @ ty::AdtDef { .. }, ..) if def.variants.raw.len() == 1 => {
            walk_until_single_field_leaf(patterns, &|kind| single_struct(kind).or_else(|| single_tuple(kind)))
        },
        ty::TyKind::Array(..) => walk_until_single_field_leaf(patterns, &|kind| single_slice(kind)),
        ty::TyKind::Tuple(..) => walk_until_single_field_leaf(patterns, &|kind| single_tuple(kind)),
        _ => None,
    }
}

fn expr_helper<'hir>(cx: &LateContext<'_>, scrutinee: &Expr<'_>, patterns: impl Iterator<Item = &'hir Pat<'hir>>) {
    find_single_pattern(cx.typeck_results().expr_ty(scrutinee).kind(), patterns)
        .map(|pattern| pattern.lint(cx, scrutinee.span));
}

impl LateLintPass<'_> for SingleFieldPattern {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        match IfLetOrMatch::parse(cx, expr) {
            Some(IfLetOrMatch::Match(scrutinee, arms, MatchSource::Normal)) => {
                expr_helper(cx, scrutinee, arms.iter().map(|arm| arm.pat))
            },
            Some(IfLetOrMatch::IfLet(scrutinee, pat, ..)) => expr_helper(cx, scrutinee, once(pat)),
            _ => {
                if let Some(WhileLet { let_pat, let_expr, .. }) = WhileLet::hir(expr) {
                    expr_helper(cx, let_expr, once(let_pat))
                }
            },
        };
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &Stmt<'tcx>) {
        if stmt.span.from_expansion() {
            return;
        }
        if let StmtKind::Local(Local { pat, ty, init, .. }) = stmt.kind {
            let scrut_type = if let Some(t) = *ty {
                cx.typeck_results().node_type(t.hir_id).kind()
            } else if let Some(e) = init {
                cx.typeck_results().expr_ty(e).kind()
            } else {
                return;
            };
            find_single_pattern(scrut_type, once(*pat)).map(|field| field.lint(cx, stmt.span));
        }
    }
}
