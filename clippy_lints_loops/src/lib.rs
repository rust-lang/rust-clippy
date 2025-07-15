#![feature(macro_metavar_expr_concat, never_type, rustc_private, unwrap_infallible)]
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

extern crate rustc_ast;
extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_typeck;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

#[macro_use]
extern crate declare_clippy_lint;

pub mod declared_lints;

mod utils;

// begin lints modules, do not remove this comment, it's used in `update_lints`
mod char_indices_as_byte_indices;
mod empty_loop;
mod explicit_counter_loop;
mod explicit_into_iter_loop;
mod explicit_iter_loop;
mod for_kv_map;
mod infinite_loop;
mod iter_next_loop;
mod manual_find;
mod manual_flatten;
mod manual_memcpy;
mod manual_slice_fill;
mod manual_while_let_some;
mod missing_spin_loop;
mod mut_range_bound;
mod needless_range_loop;
mod never_loop;
mod same_item_push;
mod single_element_loop;
mod unused_enumerate_index;
mod while_float;
mod while_immutable_condition;
mod while_let_loop;
mod while_let_on_iterator;
// end lints modules, do not remove this comment, it's used in `update_lints`

use clippy_config::Conf;
use clippy_utils::msrvs::Msrv;
use clippy_utils::{higher, sym};
use rustc_ast::Label;
use rustc_hir::{Expr, ExprKind, LoopSource, Pat};
use rustc_lint::{LateContext, LateLintPass, LintStore};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use utils::{IncrementVisitor, InitializeVisitor, make_iterator_snippet};

struct Loops {
    msrv: Msrv,
    enforce_iter_loop_reborrow: bool,
}
impl Loops {
    fn new(conf: &'static Conf) -> Self {
        Self {
            msrv: conf.msrv,
            enforce_iter_loop_reborrow: conf.enforce_iter_loop_reborrow,
        }
    }
}

impl_lint_pass!(Loops => [
    manual_memcpy::MANUAL_MEMCPY,
    manual_flatten::MANUAL_FLATTEN,
    needless_range_loop::NEEDLESS_RANGE_LOOP,
    explicit_iter_loop::EXPLICIT_ITER_LOOP,
    explicit_into_iter_loop::EXPLICIT_INTO_ITER_LOOP,
    iter_next_loop::ITER_NEXT_LOOP,
    while_let_loop::WHILE_LET_LOOP,
    explicit_counter_loop::EXPLICIT_COUNTER_LOOP,
    empty_loop::EMPTY_LOOP,
    while_let_on_iterator::WHILE_LET_ON_ITERATOR,
    for_kv_map::FOR_KV_MAP,
    never_loop::NEVER_LOOP,
    mut_range_bound::MUT_RANGE_BOUND,
    while_immutable_condition::WHILE_IMMUTABLE_CONDITION,
    while_float::WHILE_FLOAT,
    same_item_push::SAME_ITEM_PUSH,
    single_element_loop::SINGLE_ELEMENT_LOOP,
    missing_spin_loop::MISSING_SPIN_LOOP,
    manual_find::MANUAL_FIND,
    manual_while_let_some::MANUAL_WHILE_LET_SOME,
    unused_enumerate_index::UNUSED_ENUMERATE_INDEX,
    infinite_loop::INFINITE_LOOP,
    manual_slice_fill::MANUAL_SLICE_FILL,
    char_indices_as_byte_indices::CHAR_INDICES_AS_BYTE_INDICES,
]);

impl<'tcx> LateLintPass<'tcx> for Loops {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let for_loop = higher::ForLoop::hir(expr);
        if let Some(higher::ForLoop {
            pat,
            arg,
            body,
            loop_id,
            span,
            label,
        }) = for_loop
        {
            // we don't want to check expanded macros
            // this check is not at the top of the function
            // since higher::for_loop expressions are marked as expansions
            if body.span.from_expansion() {
                return;
            }
            self.check_for_loop(cx, pat, arg, body, expr, span, label);
            if let ExprKind::Block(block, _) = body.kind {
                never_loop::check(cx, block, loop_id, span, for_loop.as_ref());
            }
        }

        // we don't want to check expanded macros
        if expr.span.from_expansion() {
            return;
        }

        // check for never_loop
        if let ExprKind::Loop(block, ..) = expr.kind {
            never_loop::check(cx, block, expr.hir_id, expr.span, None);
        }

        // check for `loop { if let {} else break }` that could be `while let`
        // (also matches an explicit "match" instead of "if let")
        // (even if the "match" or "if let" is used for declaration)
        if let ExprKind::Loop(block, label, LoopSource::Loop, _) = expr.kind {
            // also check for empty `loop {}` statements, skipping those in #[panic_handler]
            empty_loop::check(cx, expr, block);
            while_let_loop::check(cx, expr, block);
            infinite_loop::check(cx, expr, block, label);
        }

        while_let_on_iterator::check(cx, expr);

        if let Some(higher::While { condition, body, span }) = higher::While::hir(expr) {
            while_immutable_condition::check(cx, condition, body);
            while_float::check(cx, condition);
            missing_spin_loop::check(cx, condition, body);
            manual_while_let_some::check(cx, condition, body, span);
        }

        if let ExprKind::MethodCall(path, recv, [arg], _) = expr.kind
            && matches!(
                path.ident.name,
                sym::all | sym::any | sym::filter_map | sym::find_map | sym::flat_map | sym::for_each | sym::map
            )
        {
            unused_enumerate_index::check_method(cx, expr, recv, arg);
        }
    }
}

impl Loops {
    #[allow(clippy::too_many_arguments)]
    fn check_for_loop<'tcx>(
        &self,
        cx: &LateContext<'tcx>,
        pat: &'tcx Pat<'_>,
        arg: &'tcx Expr<'_>,
        body: &'tcx Expr<'_>,
        expr: &'tcx Expr<'_>,
        span: Span,
        label: Option<Label>,
    ) {
        let is_manual_memcpy_triggered = manual_memcpy::check(cx, pat, arg, body, expr);
        if !is_manual_memcpy_triggered {
            manual_slice_fill::check(cx, pat, arg, body, expr, self.msrv);
            needless_range_loop::check(cx, pat, arg, body, expr);
            explicit_counter_loop::check(cx, pat, arg, body, expr, label);
        }
        self.check_for_loop_arg(cx, pat, arg);
        for_kv_map::check(cx, pat, arg, body);
        mut_range_bound::check(cx, arg, body);
        single_element_loop::check(cx, pat, arg, body, expr);
        same_item_push::check(cx, pat, arg, body, expr, self.msrv);
        manual_flatten::check(cx, pat, arg, body, span, self.msrv);
        manual_find::check(cx, pat, arg, body, span, expr);
        unused_enumerate_index::check_loop(cx, arg, pat, None, body);
        char_indices_as_byte_indices::check(cx, pat, arg, body);
    }

    fn check_for_loop_arg(&self, cx: &LateContext<'_>, _: &Pat<'_>, arg: &Expr<'_>) {
        if !arg.span.from_expansion()
            && let ExprKind::MethodCall(method, self_arg, [], _) = arg.kind
        {
            match method.ident.name {
                sym::iter | sym::iter_mut => {
                    explicit_iter_loop::check(cx, self_arg, arg, self.msrv, self.enforce_iter_loop_reborrow);
                },
                sym::into_iter => {
                    explicit_into_iter_loop::check(cx, self_arg, arg);
                },
                sym::next => {
                    iter_next_loop::check(cx, arg);
                },
                _ => {},
            }
        }
    }
}

pub fn register_lint_passes(store: &mut LintStore, conf: &'static Conf) {
    store.register_late_pass(move |_| Box::new(Loops::new(conf)));
}
