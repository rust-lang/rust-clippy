#![feature(if_let_guard, macro_metavar_expr_concat, rustc_private)]
#![allow(
    clippy::missing_docs_in_private_items,
    clippy::must_use_candidate,
    rustc::diagnostic_outside_of_impl,
    rustc::untranslatable_diagnostic,
    clippy::literal_string_with_formatting_args
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications,
    rustc::internal
)]

extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_pretty;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

#[macro_use]
extern crate declare_clippy_lint;

pub mod declared_lints;

mod manual_utils;

// begin lints modules, do not remove this comment, it's used in `update_lints`
mod collapsible_match;
mod infallible_destructuring_match;
mod manual_filter;
mod manual_map;
mod manual_ok_err;
mod manual_unwrap_or;
mod map_unit_fn;
mod match_as_ref;
mod match_bool;
mod match_like_matches;
mod match_ref_pats;
mod match_same_arms;
mod match_single_binding;
mod match_str_case_mismatch;
mod match_wild_enum;
mod match_wild_err_arm;
mod needless_match;
mod overlapping_arms;
mod redundant_guards;
mod redundant_pattern_match;
mod rest_pat_in_fully_bound_struct;
mod significant_drop_in_scrutinee;
mod single_match;
mod try_err;
mod wild_in_or_pats;
// end lints modules, do not remove this comment, it's used in `update_lints`

use clippy_config::Conf;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::walk_span_to_context;
use clippy_utils::{
    higher, is_direct_expn_of, is_in_const_context, is_span_match, span_contains_cfg, span_extract_comments, sym,
};
use rustc_hir::{Arm, Expr, ExprKind, LetStmt, MatchSource, Pat, PatKind};
use rustc_lint::{LateContext, LateLintPass, LintContext, LintStore};
use rustc_session::impl_lint_pass;
use rustc_span::{SpanData, SyntaxContext};

struct Matches {
    msrv: Msrv,
    infallible_destructuring_match_linted: bool,
}

impl Matches {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            msrv: conf.msrv,
            infallible_destructuring_match_linted: false,
        }
    }
}

impl_lint_pass!(Matches => [
    single_match::SINGLE_MATCH,
    match_ref_pats::MATCH_REF_PATS,
    match_bool::MATCH_BOOL,
    single_match::SINGLE_MATCH_ELSE,
    overlapping_arms::MATCH_OVERLAPPING_ARM,
    match_wild_err_arm::MATCH_WILD_ERR_ARM,
    match_as_ref::MATCH_AS_REF,
    match_wild_enum::WILDCARD_ENUM_MATCH_ARM,
    match_wild_enum::MATCH_WILDCARD_FOR_SINGLE_VARIANTS,
    wild_in_or_pats::WILDCARD_IN_OR_PATTERNS,
    match_single_binding::MATCH_SINGLE_BINDING,
    infallible_destructuring_match::INFALLIBLE_DESTRUCTURING_MATCH,
    rest_pat_in_fully_bound_struct::REST_PAT_IN_FULLY_BOUND_STRUCTS,
    redundant_pattern_match::REDUNDANT_PATTERN_MATCHING,
    match_like_matches::MATCH_LIKE_MATCHES_MACRO,
    match_same_arms::MATCH_SAME_ARMS,
    needless_match::NEEDLESS_MATCH,
    collapsible_match::COLLAPSIBLE_MATCH,
    manual_unwrap_or::MANUAL_UNWRAP_OR,
    manual_unwrap_or::MANUAL_UNWRAP_OR_DEFAULT,
    match_str_case_mismatch::MATCH_STR_CASE_MISMATCH,
    significant_drop_in_scrutinee::SIGNIFICANT_DROP_IN_SCRUTINEE,
    try_err::TRY_ERR,
    manual_map::MANUAL_MAP,
    manual_filter::MANUAL_FILTER,
    redundant_guards::REDUNDANT_GUARDS,
    manual_ok_err::MANUAL_OK_ERR,
]);

impl<'tcx> LateLintPass<'tcx> for Matches {
    #[expect(clippy::too_many_lines)]
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if is_direct_expn_of(expr.span, sym::matches).is_none() && expr.span.in_external_macro(cx.sess().source_map()) {
            return;
        }
        let from_expansion = expr.span.from_expansion();

        if let ExprKind::Match(ex, arms, source) = expr.kind {
            if is_direct_expn_of(expr.span, sym::matches).is_some()
                && let [arm, _] = arms
            {
                redundant_pattern_match::check_match(cx, expr, ex, arms);
                redundant_pattern_match::check_matches_true(cx, expr, arm, ex);
            }

            if source == MatchSource::Normal && !is_span_match(cx, expr.span) {
                return;
            }
            if matches!(source, MatchSource::Normal | MatchSource::ForLoopDesugar) {
                significant_drop_in_scrutinee::check_match(cx, expr, ex, arms, source);
            }

            collapsible_match::check_match(cx, arms, self.msrv);
            if !from_expansion {
                // These don't depend on a relationship between multiple arms
                match_wild_err_arm::check(cx, ex, arms);
                wild_in_or_pats::check(cx, ex, arms);
            }

            if let MatchSource::TryDesugar(_) = source {
                try_err::check(cx, expr, ex);
            }

            if !from_expansion && !contains_cfg_arm(cx, expr, ex, arms) {
                if source == MatchSource::Normal {
                    if !(self.msrv.meets(cx, msrvs::MATCHES_MACRO)
                        && match_like_matches::check_match(cx, expr, ex, arms))
                    {
                        match_same_arms::check(cx, arms);
                    }

                    redundant_pattern_match::check_match(cx, expr, ex, arms);
                    let source_map = cx.tcx.sess.source_map();
                    let mut match_comments = span_extract_comments(source_map, expr.span);
                    // We remove comments from inside arms block.
                    if !match_comments.is_empty() {
                        for arm in arms {
                            for comment in span_extract_comments(source_map, arm.body.span) {
                                if let Some(index) = match_comments
                                    .iter()
                                    .enumerate()
                                    .find(|(_, cm)| **cm == comment)
                                    .map(|(index, _)| index)
                                {
                                    match_comments.remove(index);
                                }
                            }
                        }
                    }
                    // If there are still comments, it means they are outside of the arms. Tell the lint
                    // code about it.
                    single_match::check(cx, ex, arms, expr, !match_comments.is_empty());
                    match_bool::check(cx, ex, arms, expr);
                    overlapping_arms::check(cx, ex, arms);
                    match_wild_enum::check(cx, ex, arms);
                    match_as_ref::check(cx, ex, arms, expr);
                    needless_match::check_match(cx, ex, arms, expr);
                    match_str_case_mismatch::check(cx, ex, arms);
                    redundant_guards::check(cx, arms, self.msrv);

                    if !is_in_const_context(cx) {
                        manual_unwrap_or::check_match(cx, expr, ex, arms);
                        manual_map::check_match(cx, expr, ex, arms);
                        manual_filter::check_match(cx, ex, arms, expr);
                        manual_ok_err::check_match(cx, expr, ex, arms);
                    }

                    if self.infallible_destructuring_match_linted {
                        self.infallible_destructuring_match_linted = false;
                    } else {
                        match_single_binding::check(cx, ex, arms, expr);
                    }
                }
                match_ref_pats::check(cx, ex, arms.iter().map(|el| el.pat), expr);
            }
        } else if let Some(if_let) = higher::IfLet::hir(cx, expr) {
            collapsible_match::check_if_let(cx, if_let.let_pat, if_let.if_then, if_let.if_else, self.msrv);
            significant_drop_in_scrutinee::check_if_let(cx, expr, if_let.let_expr, if_let.if_then, if_let.if_else);
            if !from_expansion {
                if let Some(else_expr) = if_let.if_else {
                    if self.msrv.meets(cx, msrvs::MATCHES_MACRO) {
                        match_like_matches::check_if_let(
                            cx,
                            expr,
                            if_let.let_pat,
                            if_let.let_expr,
                            if_let.if_then,
                            else_expr,
                        );
                    }
                    if !is_in_const_context(cx) {
                        manual_unwrap_or::check_if_let(
                            cx,
                            expr,
                            if_let.let_pat,
                            if_let.let_expr,
                            if_let.if_then,
                            else_expr,
                        );
                        manual_map::check_if_let(cx, expr, if_let.let_pat, if_let.let_expr, if_let.if_then, else_expr);
                        manual_filter::check_if_let(
                            cx,
                            expr,
                            if_let.let_pat,
                            if_let.let_expr,
                            if_let.if_then,
                            else_expr,
                        );
                        manual_ok_err::check_if_let(
                            cx,
                            expr,
                            if_let.let_pat,
                            if_let.let_expr,
                            if_let.if_then,
                            else_expr,
                        );
                    }
                }
                redundant_pattern_match::check_if_let(
                    cx,
                    expr,
                    if_let.let_pat,
                    if_let.let_expr,
                    if_let.if_else.is_some(),
                    if_let.let_span,
                );
                needless_match::check_if_let(cx, expr, &if_let);
            }
        } else {
            if let Some(while_let) = higher::WhileLet::hir(expr) {
                significant_drop_in_scrutinee::check_while_let(cx, expr, while_let.let_expr, while_let.if_then);
            }
            if !from_expansion {
                redundant_pattern_match::check(cx, expr);
            }
        }
    }

    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx LetStmt<'_>) {
        self.infallible_destructuring_match_linted |=
            local.els.is_none() && infallible_destructuring_match::check(cx, local);
    }

    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx Pat<'_>) {
        rest_pat_in_fully_bound_struct::check(cx, pat);
    }
}

/// Checks if there are any arms with a `#[cfg(..)]` attribute.
fn contains_cfg_arm(cx: &LateContext<'_>, e: &Expr<'_>, scrutinee: &Expr<'_>, arms: &[Arm<'_>]) -> bool {
    let Some(scrutinee_span) = walk_span_to_context(scrutinee.span, SyntaxContext::root()) else {
        // Shouldn't happen, but treat this as though a `cfg` attribute were found
        return true;
    };

    let start = scrutinee_span.hi();
    let mut arm_spans = arms.iter().map(|arm| {
        let data = arm.span.data();
        (data.ctxt == SyntaxContext::root()).then_some((data.lo, data.hi))
    });
    let end = e.span.hi();

    // Walk through all the non-code space before each match arm. The space trailing the final arm is
    // handled after the `try_fold` e.g.
    //
    // match foo {
    // _________^-                      everything between the scrutinee and arm1
    //|    arm1 => (),
    //|---^___________^                 everything before arm2
    //|    #[cfg(feature = "enabled")]
    //|    arm2 => some_code(),
    //|---^____________________^        everything before arm3
    //|    // some comment about arm3
    //|    arm3 => some_code(),
    //|---^____________________^        everything after arm3
    //|    #[cfg(feature = "disabled")]
    //|    arm4 = some_code(),
    //|};
    //|^
    let found = arm_spans.try_fold(start, |start, range| {
        let Some((end, next_start)) = range else {
            // Shouldn't happen as macros can't expand to match arms, but treat this as though a `cfg` attribute
            // were found.
            return Err(());
        };
        let span = SpanData {
            lo: start,
            hi: end,
            ctxt: SyntaxContext::root(),
            parent: None,
        }
        .span();
        (!span_contains_cfg(cx, span)).then_some(next_start).ok_or(())
    });
    match found {
        Ok(start) => {
            let span = SpanData {
                lo: start,
                hi: end,
                ctxt: SyntaxContext::root(),
                parent: None,
            }
            .span();
            span_contains_cfg(cx, span)
        },
        Err(()) => true,
    }
}

/// Checks if `pat` contains OR patterns that cannot be nested due to a too low MSRV.
fn pat_contains_disallowed_or(cx: &LateContext<'_>, pat: &Pat<'_>, msrv: Msrv) -> bool {
    let mut contains_or = false;
    pat.walk(|p| {
        let is_or = matches!(p.kind, PatKind::Or(_));
        contains_or |= is_or;
        !is_or
    });
    contains_or && !msrv.meets(cx, msrvs::OR_PATTERNS)
}

pub fn register_lint_passes(store: &mut LintStore, conf: &'static Conf) {
    store.register_late_pass(move |_| Box::new(Matches::new(conf)));
    store.register_late_pass(|_| Box::new(map_unit_fn::MapUnit));
}
