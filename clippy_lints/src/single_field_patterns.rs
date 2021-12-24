#![allow(rustc::usage_of_ty_tykind)]

use clippy_utils::{
    diagnostics::{multispan_sugg_with_applicability, span_lint_and_then},
    higher::IfLetOrMatch,
    higher::WhileLet,
    source::snippet_opt,
};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Local, MatchSource, Pat, PatKind, Stmt, StmtKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{symbol::Ident, Span};
use std::iter::once;

declare_clippy_lint! {
    /// ### What it does
    ///  Checks for patterns that only use a single field when they could directly access the field.
    ///
    /// ### Why is this bad?
    ///  It requires more text and more information than directly accessing the field.
    ///
    /// ### Example
    /// ```rust
    /// # struct Struct {
    /// #     field1: Option<i32>,
    /// #     field2: Option<i32>,
    /// # }
    /// # fn foo(struct1: Struct) {
    /// // bad:
    /// match struct1 {
    ///     Struct { field1: Some(n), .. } if n >= 50 => {},
    ///     Struct { field1: None, .. } => {},
    ///     _ => {},
    /// }
    /// // better:
    /// match struct1.field1 {
    ///     Some(n) if n >= 50 => {},
    ///     None => {},
    ///     _ => {},
    /// }
    /// # }
    /// ```
    #[clippy::version = "1.59.0"]
    pub SINGLE_FIELD_PATTERNS,
    style,
    "single-field patterns"
}
declare_lint_pass!(SingleFieldPatterns => [SINGLE_FIELD_PATTERNS]);

/// This represents 0 or 1 fields being used. Where more may be used, I use Option<SingleField>
///   where None represents the absence of a lint
#[derive(Debug, Clone, Copy)]
enum SingleField {
    Id { id: Ident, pattern: Span },
    Index { index: usize, pattern: Span },
    Unused, // The name "SingleField" is a lie but idk what's better. "AtMostOneField"?
}

impl PartialEq for SingleField {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SingleField::Id { id: id1, .. }, SingleField::Id { id: id2, .. }) => id1 == id2,
            (SingleField::Index { index: index1, .. }, SingleField::Index { index: index2, .. }) => index1 == index2,
            (SingleField::Unused, SingleField::Unused) => true,
            _ => false,
        }
    }
}

trait IntoSingleField {
    fn into_sf(self, span: Span) -> SingleField;
}

impl IntoSingleField for Ident {
    fn into_sf(self, pattern: Span) -> SingleField {
        SingleField::Id { id: self, pattern }
    }
}

impl IntoSingleField for usize {
    fn into_sf(self, pattern: Span) -> SingleField {
        SingleField::Index { index: self, pattern }
    }
}

fn get_sf<'a, ID: IntoSingleField>(mut iter: impl Iterator<Item = (ID, &'a Pat<'a>)>) -> Option<SingleField> {
    let one = iter.by_ref().find(|(_, pat)| !matches!(pat.kind, PatKind::Wild));
    match one {
        Some((index, pat)) => {
            if pat.span.from_expansion() {
                return None;
            }
            if iter.all(|(_, other)| matches!(other.kind, PatKind::Wild)) {
                Some(index.into_sf(pat.span))
            } else {
                None
            }
        },
        None => Some(SingleField::Unused),
    }
}

fn inner_tuple_sf(pats: &[Pat<'_>], leap: Option<usize>) -> Option<SingleField> {
    get_sf(pats.iter().enumerate()).and_then(|field| {
        if let SingleField::Index { index, .. } = field {
            // exclude (.., x) type patterns
            if let Some(leap_index) = leap {
                if leap_index <= index {
                    return None;
                }
            }
        }
        Some(field)
    })
}

fn struct_sf(pat: &PatKind<'_>) -> Option<SingleField> {
    match *pat {
        PatKind::Struct(_, pats, _) => get_sf(pats.iter().map(|field| (field.ident, field.pat))),
        PatKind::TupleStruct(_, pats, leap) => inner_tuple_sf(pats, leap),
        _ => None,
    }
}

fn tuple_sf(pat: &PatKind<'_>) -> Option<SingleField> {
    if let PatKind::Tuple(pats, leap) = *pat {
        inner_tuple_sf(pats, leap)
    } else {
        None
    }
}

fn slice_sf(pat: &PatKind<'_>) -> Option<SingleField> {
    if let PatKind::Slice(before, dots, after) = pat {
        // exclude [.., x] type patterns
        if dots.is_none() || after.is_empty() {
            return get_sf(before.iter().enumerate());
        }
    }
    None
}

/// This handles recursive patterns and flattens them out lazily
/// e.g. 1 | (2 | 9) | 3..5
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
        if self.stack.is_empty() {
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
        unreachable!("Or always has 2 patterns, so one of the prior returns must return");
    }
}

fn find_sf_lint<'hir>(
    cx: &LateContext<'_>,
    patterns: impl Iterator<Item = &'hir Pat<'hir>>,
    leaf_sf: &impl Fn(&PatKind<'hir>) -> Option<SingleField>,
) -> Option<(SingleField, Vec<(Span, String)>)> {
    let fields = FlatPatterns::new(patterns).map(|p| {
        (
            p.span,
            if matches!(p.kind, PatKind::Wild) {
                Some(SingleField::Unused)
            } else {
                leaf_sf(&p.kind)
            },
        )
    });
    let mut spans = Vec::<(Span, String)>::new();
    let mut the_one: Option<SingleField> = None;
    for (target, sf) in fields {
        if target.from_expansion() {
            return None;
        }
        if let Some(sf) = sf {
            match sf {
                SingleField::Unused => {
                    spans.push((target, String::from("_")));
                },
                SingleField::Id { pattern, .. } | SingleField::Index { pattern, .. } => {
                    if let Some(str) = snippet_opt(cx, pattern) {
                        spans.push((target, str));
                    } else {
                        return None;
                    }
                    if let Some(one) = the_one {
                        if sf != one {
                            return None;
                        }
                    } else {
                        the_one = Some(sf);
                    }
                },
            }
        } else {
            return None;
        }
    }
    the_one.map(|one| (one, spans))
}

fn apply_lint_sf(cx: &LateContext<'_>, span: Span, sugg: impl IntoIterator<Item = (Span, String)>) {
    span_lint_and_then(
        cx,
        SINGLE_FIELD_PATTERNS,
        span,
        "this single-variant pattern only matches one field",
        |diag| {
            multispan_sugg_with_applicability(diag, "try this", Applicability::MachineApplicable, sugg);
        },
    );
}

fn remove_deref<'a>(mut scrutinee: &'a Expr<'a>) -> &'a Expr<'a> {
    // it would be wrong to convert something like `if let (x, _) = *a` into `if let x = *a.0`
    while let ExprKind::Unary(UnOp::Deref, expr) = scrutinee.kind {
        scrutinee = expr;
    }
    scrutinee
}

fn lint_sf<'hir>(
    cx: &LateContext<'_>,
    overall_span: Span,
    scrutinee: &Expr<'_>,
    patterns: impl Iterator<Item = &'hir Pat<'hir>>,
) {
    if scrutinee.span.from_expansion() {
        return;
    }
    let scrutinee_name = if let Some(name) = snippet_opt(cx, remove_deref(scrutinee).span) {
        name
    } else {
        return;
    };
    match cx.typeck_results().expr_ty(scrutinee).kind() {
        ty::TyKind::Adt(def @ ty::AdtDef { .. }, ..) if def.is_struct() => {
            if let Some((field, mut spans)) = find_sf_lint(cx, patterns, &struct_sf) {
                spans.push((
                    scrutinee.span,
                    match field {
                        SingleField::Id { id, .. } => format!("{}.{}", scrutinee_name, id.as_str()),
                        SingleField::Index { index, .. } => format!("{}.{}", scrutinee_name, index),
                        SingleField::Unused => return,
                    },
                ));
                apply_lint_sf(cx, overall_span, spans);
            }
        },
        ty::TyKind::Array(..) => {
            if let Some((SingleField::Index { index, .. }, mut spans)) = find_sf_lint(cx, patterns, &slice_sf) {
                spans.push((scrutinee.span, format!("{}[{}]", scrutinee_name, index)));
                apply_lint_sf(cx, overall_span, spans);
            }
        },
        ty::TyKind::Tuple(..) => {
            if let Some((SingleField::Index { index, .. }, mut spans)) = find_sf_lint(cx, patterns, &tuple_sf) {
                spans.push((scrutinee.span, format!("{}.{}", scrutinee_name, index)));
                apply_lint_sf(cx, overall_span, spans);
            }
        },
        _ => (),
    };
}

impl LateLintPass<'_> for SingleFieldPatterns {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        match IfLetOrMatch::parse(cx, expr) {
            Some(IfLetOrMatch::Match(scrutinee, arms, MatchSource::Normal)) => {
                lint_sf(cx, expr.span, scrutinee, arms.iter().map(|arm| arm.pat));
            },
            Some(IfLetOrMatch::IfLet(scrutinee, pat, ..)) => lint_sf(cx, expr.span, scrutinee, once(pat)),
            _ => {
                if let Some(WhileLet { let_pat, let_expr, .. }) = WhileLet::hir(expr) {
                    lint_sf(cx, expr.span, let_expr, once(let_pat));
                }
            },
        };
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &Stmt<'tcx>) {
        if stmt.span.from_expansion() {
            return;
        }
        if let StmtKind::Local(Local {
            pat,
            init: Some(scrutinee),
            ..
        }) = stmt.kind
        {
            lint_sf(cx, stmt.span, scrutinee, once(*pat));
        }
    }
}
