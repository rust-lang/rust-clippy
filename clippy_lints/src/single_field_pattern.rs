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
enum SingleField {
    Id { id: Ident, pattern: Span },
    Index { index: usize, pattern: Span },
    Unused,
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

impl SingleField {
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
    fn into_fields(self, span: Span) -> SingleField;
}

impl IntoFields for Ident {
    fn into_fields(self, pattern: Span) -> SingleField {
        SingleField::Id { id: self, pattern }
    }
}

impl IntoFields for usize {
    fn into_fields(self, pattern: Span) -> SingleField {
        SingleField::Index { index: self, pattern }
    }
}

fn get_sf<'a, ID: IntoFields>(mut iter: impl Iterator<Item = (ID, &'a Pat<'a>)>) -> Option<SingleField> {
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

fn find_sf_lint<'hir>(
    patterns: impl Iterator<Item = &'hir Pat<'hir>>,
    leaf_sf: &impl Fn(&PatKind<'hir>) -> Option<SingleField>,
) -> Option<SingleField> {
    // todo - return an Option<Fields, Vec<(Span, Span)>> - previous for the scrutinee, latter to
    // replace patterns appropriately - 2 spans to map a pattern span to a struct match span
    let mut fields = FlatPatterns::new(patterns).map(|p| {
        if matches!(p.kind, PatKind::Wild) {
            Some(SingleField::Unused) // todo: add pat span so we can replace it
        } else {
            leaf_sf(&p.kind)
        }
    });
    // todo: handle initial unused case - this should be the first one with an actual field
    if let Some(the_one) = fields.next() {
        if fields.all(|other| other == the_one || matches!(other, Some(SingleField::Unused))) {
            the_one
        } else {
            None
        }
    } else {
        // This should only happen if patterns is empty
        None
    }
}

fn typed_sf_lint<'hir>(ty: &ty::TyKind<'_>, patterns: impl Iterator<Item = &'hir Pat<'hir>>) -> Option<SingleField> {
    match ty {
        ty::TyKind::Adt(def @ ty::AdtDef { .. }, ..) if def.variants.raw.len() == 1 => {
            find_sf_lint(patterns, &struct_sf)
        },
        ty::TyKind::Array(..) => find_sf_lint(patterns, &slice_sf),
        ty::TyKind::Tuple(..) => find_sf_lint(patterns, &tuple_sf),
        _ => None,
    }
}

fn expr_sf_lint<'hir>(cx: &LateContext<'_>, scrutinee: &Expr<'_>, patterns: impl Iterator<Item = &'hir Pat<'hir>>) {
    typed_sf_lint(cx.typeck_results().expr_ty(scrutinee).kind(), patterns)
        .map(|pattern| pattern.lint(cx, scrutinee.span));
}

impl LateLintPass<'_> for SingleFieldPattern {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        match IfLetOrMatch::parse(cx, expr) {
            Some(IfLetOrMatch::Match(scrutinee, arms, MatchSource::Normal)) => {
                expr_sf_lint(cx, scrutinee, arms.iter().map(|arm| arm.pat))
            },
            Some(IfLetOrMatch::IfLet(scrutinee, pat, ..)) => expr_sf_lint(cx, scrutinee, once(pat)),
            _ => {
                if let Some(WhileLet { let_pat, let_expr, .. }) = WhileLet::hir(expr) {
                    expr_sf_lint(cx, let_expr, once(let_pat))
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
            typed_sf_lint(scrut_type, once(*pat)).map(|field| field.lint(cx, stmt.span));
        }
    }
}
