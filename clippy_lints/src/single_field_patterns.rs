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
use std::iter;

declare_clippy_lint! {
    /// ### What it does
    ///  Checks for patterns that only match a single field of a struct when they could directly access the field.
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

/// This represents 0 or 1 fields being used. Where more may be used, `Option<SingleField>` is used
///   where `None` represents the absence of a lint
#[derive(Debug, Clone, Copy)]
enum SingleField {
    Id { id: Ident, pattern: Span },
    Unused, // The name "SingleField" is a lie but idk what's better. "AtMostOneField"?
}

impl PartialEq for SingleField {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SingleField::Id { id: id1, .. }, SingleField::Id { id: id2, .. }) => id1 == id2,
            (SingleField::Unused, SingleField::Unused) => true,
            _ => false,
        }
    }
}

impl SingleField {
    fn new<'a>(mut iter: impl Iterator<Item = (Ident, &'a Pat<'a>)>) -> Option<SingleField> {
        let one = iter.by_ref().find(|(_, pat)| !matches!(pat.kind, PatKind::Wild));
        match one {
            Some((id, pat)) => {
                if pat.span.from_expansion() {
                    return None;
                }
                if iter.all(|(_, other)| matches!(other.kind, PatKind::Wild)) {
                    Some(SingleField::Id { id, pattern: pat.span })
                } else {
                    None
                }
            },
            None => Some(SingleField::Unused),
        }
    }
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
) -> Option<(SingleField, Vec<(Span, String)>)> {
    let fields = FlatPatterns::new(patterns).map(|p| {
        (
            p.span,
            match p.kind {
                PatKind::Wild => Some(SingleField::Unused),
                PatKind::Struct(_, pats, _) => SingleField::new(pats.iter().map(|field| (field.ident, field.pat))),
                _ => None,
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
                SingleField::Id { pattern, .. } => {
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
            multispan_sugg_with_applicability(diag, "try accessing this field directly", Applicability::MaybeIncorrect, sugg);
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
        ty::Adt(def, ..) if def.is_struct() => {
            if let Some((field, mut spans)) = find_sf_lint(cx, patterns) {
                spans.push((
                    scrutinee.span,
                    match field {
                        SingleField::Id { id, .. } => format!("{}.{}", scrutinee_name, id.as_str()),
                        SingleField::Unused => return,
                    },
                ));
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
            Some(IfLetOrMatch::IfLet(scrutinee, pat, ..)) => lint_sf(cx, expr.span, scrutinee, iter::once(pat)),
            _ => {
                if let Some(WhileLet { let_pat, let_expr, .. }) = WhileLet::hir(expr) {
                    lint_sf(cx, expr.span, let_expr, iter::once(let_pat));
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
            lint_sf(cx, stmt.span, scrutinee, iter::once(*pat));
        }
    }
}
