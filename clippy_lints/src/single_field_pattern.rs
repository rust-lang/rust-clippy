#![allow(rustc::usage_of_ty_tykind)]

use clippy_utils::{diagnostics::span_lint, higher::IfLetOrMatch, higher::WhileLet};
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{symbol::Ident, MultiSpan, Span};
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

#[derive(Debug, Clone, Copy)]
enum Fields {
    Id(Ident, Span), // these are the pattern span - for possible conversion to string
    Index(usize, Span),
    Unused,
}
use Fields::*;

impl PartialEq for Fields {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Id(id1, ..), Id(id2, ..)) => id1 == id2,
            (Index(index1, ..), Index(index2, ..)) => index1 == index2,
            (Unused, Unused) => true,
            _ => false,
        }
    }
}

impl Fields {
    // Todo - auto-fix - I'll need to add span and strings into SingleField
    fn lint(self, cx: &LateContext<'_>, span: impl Into<MultiSpan>) {
        span_lint(
            cx,
            SINGLE_FIELD_PATTERN,
            span,
            "pattern matching just to get a single field",
        );
    }
}

trait IntoFields {
    fn into_fields(self, span: Span) -> Fields;
}

impl IntoFields for Ident {
    fn into_fields(self, span: Span) -> Fields {
        Fields::Id(self, span)
    }
}

impl IntoFields for usize {
    fn into_fields(self, span: Span) -> Fields {
        Fields::Index(self, span)
    }
}

fn get_the_one<'a, ID: IntoFields>(mut iter: impl Iterator<Item = (ID, &'a Pat<'a>)>) -> Option<Fields> {
    let one = iter.by_ref().find(|(_, pat)| !matches!(pat.kind, PatKind::Wild));
    match one {
        Some((index, pat)) => {
            for (_, other_pat) in iter {
                if !matches!(other_pat.kind, PatKind::Wild) {
                    return None;
                }
            }
            Some(index.into_fields(pat.span))
        },
        None => Some(Fields::Unused),
    }
}

fn single_struct(pat: &PatKind<'_>) -> Option<Fields> {
    match pat {
        PatKind::Struct(_, pats, _) => get_the_one(pats.iter().map(|field| (field.ident, field.pat))),
        PatKind::TupleStruct(_, pats, leap) => single_tuple_inner(pats, leap),
        _ => None,
    }
}

fn single_tuple_inner(pats: &&[Pat<'_>], leap: &Option<usize>) -> Option<Fields> {
    get_the_one(pats.iter().enumerate()).and_then(|field| {
        if let Fields::Index(index, ..) = field {
            if let Some(leap_index) = *leap {
                if leap_index <= index {
                    return None;
                }
            }
        }
        Some(field)
    })
}

fn single_tuple(pat: &PatKind<'_>) -> Option<Fields> {
    if let PatKind::Tuple(pats, leap) = pat {
        single_tuple_inner(pats, leap)
    } else {
        None
    }
}

fn single_slice(pat: &PatKind<'_>) -> Option<Fields> {
    if let PatKind::Slice(before, dots, after) = pat {
        if dots.is_none() || after.len() == 0 {
            return get_the_one(before.iter().enumerate());
        }
    }
    None
}

/// This handles recursive patterns and flattens them out lazily
/// That basically just means handling stuff like 1 | (2 | 9) | 3..5
struct FlatPatterns<'hir, I>
where
    I: Iterator<Item = &'hir Pat<'hir>>,
{
    patterns: I,
    stack: Vec<&'hir Pat<'hir>>,
}

impl<I: Iterator<Item = &'hir Pat<'hir>>> FlatPatterns<'hir, I> {
    fn new(patterns: I) -> Self {
        Self {
            patterns,
            stack: Vec::new(),
        }
    }
}

impl<I: Iterator<Item = &'hir Pat<'hir>>> Iterator for FlatPatterns<'hir, I> {
    type Item = &'hir Pat<'hir>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stack.len() == 0 {
            if let Some(pat) = self.patterns.next() {
                self.stack.push(pat);
            } else {
                return None;
            }
        }
        while let Some(pat) = self.stack.pop() {
            match pat.kind {
                PatKind::Or(pats) => self.stack.extend(pats),
                _ => return Some(pat),
            }
        }
        None
    }
}

fn walk_until_single_field_leaf<'hir>(
    patterns: impl Iterator<Item = &'hir Pat<'hir>>,
    get_leaf: &impl Fn(&PatKind<'hir>) -> Option<Fields>,
) -> Option<Fields> {
    let mut fields = FlatPatterns::new(patterns).map(|p| {
        if matches!(p.kind, PatKind::Wild) {
            Some(Fields::Unused) // todo: add pat span so we can replace it
        } else {
            get_leaf(&p.kind)
        }
    });
    if let Some(the_one) = fields.next() {
        if fields.all(|other| other == the_one || matches!(other, Some(Fields::Unused))) {
            the_one
        } else {
            None
        }
    } else {
        // This should only happen if the first one was None
        // In which case we have one with 2+ and don't have a lint
        None
    }
}

fn find_single_pattern<'hir>(ty: &ty::TyKind<'_>, patterns: impl Iterator<Item = &'hir Pat<'hir>>) -> Option<Fields> {
    match ty {
        ty::TyKind::Adt(def @ ty::AdtDef { .. }, ..) if def.variants.raw.len() == 1 => {
            walk_until_single_field_leaf(patterns, &single_struct)
        },
        ty::TyKind::Array(..) => walk_until_single_field_leaf(patterns, &single_slice),
        ty::TyKind::Tuple(..) => walk_until_single_field_leaf(patterns, &single_tuple),
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
