#![allow(rustc::usage_of_ty_tykind)]

use clippy_utils::{
    diagnostics::{multispan_sugg_with_applicability, span_lint, span_lint_and_then},
    higher::IfLetOrMatch,
    higher::WhileLet,
    source::snippet_opt,
};
use rustc_errors::Applicability;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{symbol::Ident, Span};
use std::iter::once;

declare_clippy_lint! {
    /// ### What it does
    ///  Checks for patterns that only use a single field when they could directly access the field
    ///
    /// ### Why is this bad?
    ///  It requires more information than directly accessing the field.
    ///
    /// ### Example
    /// ```rust
    /// match struct1 {
    ///     Struct { field1: Some(n), .. } if n >= 50 => {},
    ///     Struct { field1: None, .. } => {},
    ///     _ => {},
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// match struct1.field1 {
    ///     Some(n) if n >= 50 => {},
    ///     None => {},
    ///     _ => {},
    /// }
    /// ```
    #[clippy::version = "1.59.0"]
    pub SINGLE_FIELD_PATTERNS,
    style,
    "default lint description"
}
declare_lint_pass!(SingleFieldPatterns => [SINGLE_FIELD_PATTERNS]);

#[derive(Debug, Clone, Copy)]
enum SingleField {
    Id { id: Ident, pattern: Span },
    Index { index: usize, pattern: Span },
    Unused, // The name "SingleField" is a lie but idk what's better. "AtMostOneField"?
}

impl PartialEq for SingleField {
    fn eq(&self, other: &Self) -> bool {
        use SingleField::*;
        match (self, other) {
            (Id { id: id1, .. }, Id { id: id2, .. }) => id1 == id2,
            (Index { index: index1, .. }, Index { index: index2, .. }) => index1 == index2,
            (Unused, Unused) => true,
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
            if iter.all(|(_, other)| matches!(other.kind, PatKind::Wild)) {
                Some(index.into_sf(pat.span))
            } else {
                None
            }
        },
        None => Some(SingleField::Unused),
    }
}

fn inner_tuple_sf(pats: &&[Pat<'_>], leap: &Option<usize>) -> Option<SingleField> {
    get_sf(pats.iter().enumerate()).and_then(|field| {
        if let SingleField::Index { index, .. } = field {
            if let Some(leap_index) = *leap {
                if leap_index <= index {
                    return None;
                }
            }
        }
        Some(field)
    })
}

fn struct_sf(pat: &PatKind<'_>) -> Option<SingleField> {
    match pat {
        PatKind::Struct(_, pats, _) => get_sf(pats.iter().map(|field| (field.ident, field.pat))),
        PatKind::TupleStruct(_, pats, leap) => inner_tuple_sf(pats, leap),
        _ => None,
    }
}

fn tuple_sf(pat: &PatKind<'_>) -> Option<SingleField> {
    if let PatKind::Tuple(pats, leap) = pat {
        inner_tuple_sf(pats, leap)
    } else {
        None
    }
}

fn slice_sf(pat: &PatKind<'_>) -> Option<SingleField> {
    if let PatKind::Slice(before, dots, after) = pat {
        if dots.is_none() || after.len() == 0 {
            return get_sf(before.iter().enumerate());
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

fn find_sf_lint<'hir, T: LintContext>(
    cx: &T,
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
    // if we can somehow replace this operation and this vec with an iterator, that'd be nice
    // this is slightly difficult because it's a two-part operation
    // prior = iter.as_ref().find_partition(...first id...)
    // latter = iter.
    let mut spans = Vec::<(Span, String)>::new();
    let mut the_one: Option<SingleField> = None;
    for (target, sf) in fields {
        if let Some(sf) = sf {
            match sf {
                SingleField::Unused => {
                    // this doesn't work if all fields are unused
                    // Maybe out of scope, but not handled by another lint?
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
    if spans.len() > 0 {
        Some((the_one.unwrap_or(SingleField::Unused), spans))
    } else {
        None
    }
}

fn apply_lint_sf<T: LintContext>(cx: &T, span: Span, sugg: impl IntoIterator<Item = (Span, String)>) {
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

fn apply_lint_zero_fields<T: LintContext>(cx: &T, span: Span) {
    span_lint(
        cx,
        SINGLE_FIELD_PATTERNS,
        span,
        "this single-variant pattern matches no fields",
    );
}

fn typed_sf_lint<'hir, T: LintContext>(
    cx: &T,
    overall_span: Span,
    scrutinee_span: Span,
    ty: &ty::TyKind<'_>,
    patterns: impl Iterator<Item = &'hir Pat<'hir>>,
) {
    let scrutinee_name = if let Some(name) = snippet_opt(cx, scrutinee_span) {
        name
    } else {
        return;
    };
    match ty {
        ty::TyKind::Adt(def @ ty::AdtDef { .. }, ..) if def.variants.raw.len() == 1 => {
            if let Some((field, mut spans)) = find_sf_lint(cx, patterns, &struct_sf) {
                spans.push((
                    scrutinee_span,
                    match field {
                        SingleField::Id { id, .. } => format!("{}.{}", scrutinee_name, id.as_str()),
                        SingleField::Index { index, .. } => format!("{}.{}", scrutinee_name, index),
                        SingleField::Unused => {
                            apply_lint_zero_fields(cx, overall_span);
                            return;
                        },
                    },
                ));
                apply_lint_sf(cx, overall_span, spans);
            }
        },
        ty::TyKind::Array(..) => {
            if let Some((field, mut spans)) = find_sf_lint(cx, patterns, &slice_sf) {
                match field {
                    SingleField::Index { index, .. } => {
                        spans.push((scrutinee_span, format!("{}[{}]", scrutinee_name, index)));
                        apply_lint_sf(cx, overall_span, spans);
                    },
                    SingleField::Unused => {
                        apply_lint_zero_fields(cx, overall_span);
                    },
                    _ => { /* shouldn't happen */ },
                }
            }
        },
        ty::TyKind::Tuple(..) => {
            if let Some((field, mut spans)) = find_sf_lint(cx, patterns, &tuple_sf) {
                match field {
                    SingleField::Index { index, .. } => {
                        spans.push((scrutinee_span, format!("{}.{}", scrutinee_name, index)));
                        apply_lint_sf(cx, overall_span, spans);
                    },
                    SingleField::Unused => {
                        apply_lint_zero_fields(cx, overall_span);
                    },
                    _ => { /* shouldn't happen */ },
                }
            }
        },
        _ => (),
    };
}

fn expr_sf_lint<'hir>(
    cx: &LateContext<'_>,
    overall_span: Span,
    scrutinee: &Expr<'_>,
    patterns: impl Iterator<Item = &'hir Pat<'hir>>,
) {
    typed_sf_lint(
        cx,
        overall_span,
        scrutinee.span,
        cx.typeck_results().expr_ty(scrutinee).kind(),
        patterns,
    );
}

impl LateLintPass<'_> for SingleFieldPatterns {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        match IfLetOrMatch::parse(cx, expr) {
            Some(IfLetOrMatch::Match(scrutinee, arms, MatchSource::Normal)) => {
                expr_sf_lint(cx, expr.span, scrutinee, arms.iter().map(|arm| arm.pat));
            },
            Some(IfLetOrMatch::IfLet(scrutinee, pat, ..)) => expr_sf_lint(cx, expr.span, scrutinee, once(pat)),
            _ => {
                // todo for maybe, other missing patterns
                if let Some(WhileLet { let_pat, let_expr, .. }) = WhileLet::hir(expr) {
                    expr_sf_lint(cx, expr.span, let_expr, once(let_pat));
                }
            },
        };
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &Stmt<'tcx>) {
        if stmt.span.from_expansion() {
            return;
        }
        if let StmtKind::Local(Local { pat, init, .. }) = stmt.kind {
            if let Some(scrutinee) = init {
                typed_sf_lint(
                    cx,
                    stmt.span,
                    scrutinee.span,
                    cx.typeck_results().expr_ty(scrutinee).kind(),
                    once(*pat),
                );
            }
        }
    }
}
