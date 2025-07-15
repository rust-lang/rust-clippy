#![feature(macro_metavar_expr_concat, rustc_private)]
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

extern crate rustc_abi;
extern crate rustc_ast;
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

mod utils;

// begin lints modules, do not remove this comment, it's used in `update_lints`
mod as_pointer_underscore;
mod as_ptr_cast_mut;
mod as_underscore;
mod borrow_as_ptr;
mod cast_abs_to_unsigned;
mod cast_enum_constructor;
mod cast_lossless;
mod cast_nan_to_int;
mod cast_possible_truncation;
mod cast_possible_wrap;
mod cast_precision_loss;
mod cast_ptr_alignment;
mod cast_sign_loss;
mod cast_slice_different_sizes;
mod cast_slice_from_raw_parts;
mod char_lit_as_u8;
mod confusing_method_to_numeric_cast;
mod fn_to_numeric_cast;
mod fn_to_numeric_cast_any;
mod fn_to_numeric_cast_with_truncation;
mod manual_dangling_ptr;
mod ptr_as_ptr;
mod ptr_cast_constness;
mod ref_as_ptr;
mod unnecessary_cast;
mod zero_ptr;
// end lints modules, do not remove this comment, it's used in `update_lints`

use clippy_config::Conf;
use clippy_utils::is_hir_ty_cfg_dependant;
use clippy_utils::msrvs::{self, Msrv};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext, LintStore};
use rustc_session::impl_lint_pass;

struct Casts {
    msrv: Msrv,
}

impl Casts {
    fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl_lint_pass!(Casts => [
    cast_precision_loss::CAST_PRECISION_LOSS,
    cast_sign_loss::CAST_SIGN_LOSS,
    cast_possible_truncation::CAST_POSSIBLE_TRUNCATION,
    cast_possible_wrap::CAST_POSSIBLE_WRAP,
    cast_lossless::CAST_LOSSLESS,
    cast_ptr_alignment::CAST_PTR_ALIGNMENT,
    cast_slice_different_sizes::CAST_SLICE_DIFFERENT_SIZES,
    unnecessary_cast::UNNECESSARY_CAST,
    fn_to_numeric_cast_any::FN_TO_NUMERIC_CAST_ANY,
    fn_to_numeric_cast::FN_TO_NUMERIC_CAST,
    fn_to_numeric_cast_with_truncation::FN_TO_NUMERIC_CAST_WITH_TRUNCATION,
    char_lit_as_u8::CHAR_LIT_AS_U8,
    ptr_as_ptr::PTR_AS_PTR,
    ptr_cast_constness::PTR_CAST_CONSTNESS,
    cast_possible_truncation::CAST_ENUM_TRUNCATION,
    cast_enum_constructor::CAST_ENUM_CONSTRUCTOR,
    cast_abs_to_unsigned::CAST_ABS_TO_UNSIGNED,
    as_underscore::AS_UNDERSCORE,
    borrow_as_ptr::BORROW_AS_PTR,
    cast_slice_from_raw_parts::CAST_SLICE_FROM_RAW_PARTS,
    as_ptr_cast_mut::AS_PTR_CAST_MUT,
    cast_nan_to_int::CAST_NAN_TO_INT,
    zero_ptr::ZERO_PTR,
    ref_as_ptr::REF_AS_PTR,
    as_pointer_underscore::AS_POINTER_UNDERSCORE,
    manual_dangling_ptr::MANUAL_DANGLING_PTR,
    confusing_method_to_numeric_cast::CONFUSING_METHOD_TO_NUMERIC_CAST,
]);

impl<'tcx> LateLintPass<'tcx> for Casts {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.in_external_macro(cx.sess().source_map()) {
            return;
        }

        if let ExprKind::Cast(cast_from_expr, cast_to_hir) = expr.kind {
            if is_hir_ty_cfg_dependant(cx, cast_to_hir) {
                return;
            }
            let (cast_from, cast_to) = (
                cx.typeck_results().expr_ty(cast_from_expr),
                cx.typeck_results().expr_ty(expr),
            );

            if !expr.span.from_expansion() && unnecessary_cast::check(cx, expr, cast_from_expr, cast_from, cast_to) {
                return;
            }
            cast_slice_from_raw_parts::check(cx, expr, cast_from_expr, cast_to, self.msrv);
            ptr_cast_constness::check(cx, expr, cast_from_expr, cast_from, cast_to, self.msrv);
            as_ptr_cast_mut::check(cx, expr, cast_from_expr, cast_to);
            fn_to_numeric_cast_any::check(cx, expr, cast_from_expr, cast_from, cast_to);
            confusing_method_to_numeric_cast::check(cx, expr, cast_from_expr, cast_from, cast_to);
            fn_to_numeric_cast::check(cx, expr, cast_from_expr, cast_from, cast_to);
            fn_to_numeric_cast_with_truncation::check(cx, expr, cast_from_expr, cast_from, cast_to);
            zero_ptr::check(cx, expr, cast_from_expr, cast_to_hir, self.msrv);

            if self.msrv.meets(cx, msrvs::MANUAL_DANGLING_PTR) {
                manual_dangling_ptr::check(cx, expr, cast_from_expr, cast_to_hir);
            }

            if cast_to.is_numeric() {
                cast_possible_truncation::check(cx, expr, cast_from_expr, cast_from, cast_to, cast_to_hir.span);
                if cast_from.is_numeric() {
                    cast_possible_wrap::check(cx, expr, cast_from, cast_to);
                    cast_precision_loss::check(cx, expr, cast_from, cast_to);
                    cast_sign_loss::check(cx, expr, cast_from_expr, cast_from, cast_to);
                    cast_abs_to_unsigned::check(cx, expr, cast_from_expr, cast_from, cast_to, self.msrv);
                    cast_nan_to_int::check(cx, expr, cast_from_expr, cast_from, cast_to);
                }
                cast_lossless::check(cx, expr, cast_from_expr, cast_from, cast_to, cast_to_hir, self.msrv);
                cast_enum_constructor::check(cx, expr, cast_from_expr, cast_from);
            }

            as_underscore::check(cx, expr, cast_to_hir);
            as_pointer_underscore::check(cx, cast_to, cast_to_hir);

            let was_borrow_as_ptr_emitted = self.msrv.meets(cx, msrvs::BORROW_AS_PTR)
                && borrow_as_ptr::check(cx, expr, cast_from_expr, cast_to_hir, self.msrv);
            if !was_borrow_as_ptr_emitted && self.msrv.meets(cx, msrvs::PTR_FROM_REF) {
                ref_as_ptr::check(cx, expr, cast_from_expr, cast_to_hir);
            }
        }

        if self.msrv.meets(cx, msrvs::RAW_REF_OP) {
            borrow_as_ptr::check_implicit_cast(cx, expr);
        }
        cast_ptr_alignment::check(cx, expr);
        char_lit_as_u8::check(cx, expr);
        ptr_as_ptr::check(cx, expr, self.msrv);
        cast_slice_different_sizes::check(cx, expr, self.msrv);
        ptr_cast_constness::check_null_ptr_cast_method(cx, expr);
    }
}

pub fn register_lint_passes(store: &mut LintStore, conf: &'static Conf) {
    store.register_late_pass(move |_| Box::new(Casts::new(conf)));
}
