use crate::utils;
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for casts from an unsigned type to a signed type of
    /// the same size, or possibly smaller due to target-dependent integers.
    /// Performing such a cast is a no-op for the compiler (that is, nothing is
    /// changed at the bit level), and the binary representation of the value is
    /// reinterpreted. This can cause wrapping if the value is too big
    /// for the target signed type. However, the cast works as defined, so this lint
    /// is `Allow` by default.
    ///
    /// ### Why is this bad?
    /// While such a cast is not bad in itself, the results can
    /// be surprising when this is not the intended behavior:
    ///
    /// ### Example
    /// ```no_run
    /// let _ = u32::MAX as i32; // will yield a value of `-1`
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let _ = i32::try_from(u32::MAX).ok();
    /// ```
    ///
    #[clippy::version = "pre 1.29.0"]
    pub CAST_POSSIBLE_WRAP,
    pedantic,
    "casts that may cause wrapping around the value, e.g., `x as i32` where `x: u32` and `x > i32::MAX`"
}

// this should be kept in sync with the allowed bit widths of `usize` and `isize`
const ALLOWED_POINTER_SIZES: [u64; 3] = [16, 32, 64];

// whether the lint should be emitted, and the required pointer size, if it matters
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum EmitState {
    NoLint,
    LintAlways,
    LintOnPtrSize(u64),
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, cast_from: Ty<'_>, cast_to: Ty<'_>) {
    let (Some(from_nbits), Some(to_nbits)) = (
        utils::int_ty_to_nbits(cx.tcx, cast_from),
        utils::int_ty_to_nbits(cx.tcx, cast_to),
    ) else {
        return;
    };

    // emit a lint if a cast is:
    // 1. unsigned to signed
    // and
    // 2. either:
    //
    //    2a. between two types of constant size that are always the same size
    //    2b. between one target-dependent size and one constant size integer,
    //        and the constant integer is in the allowed set of target dependent sizes
    //        (the ptr size could be chosen to be the same as the constant size)

    if cast_from.is_signed() || !cast_to.is_signed() {
        return;
    }

    let should_lint = match (cast_from.is_ptr_sized_integral(), cast_to.is_ptr_sized_integral()) {
        (true, true) => {
            // casts between two ptr sized integers are trivially always the same size
            // so do not depend on any specific pointer size to be the same
            EmitState::LintAlways
        },
        (true, false) => {
            // the first type is `usize` and the second is a constant sized signed integer
            if ALLOWED_POINTER_SIZES.contains(&to_nbits) {
                EmitState::LintOnPtrSize(to_nbits)
            } else {
                EmitState::NoLint
            }
        },
        (false, true) => {
            // the first type is a constant sized unsigned integer, and the second is `isize`
            if ALLOWED_POINTER_SIZES.contains(&from_nbits) {
                EmitState::LintOnPtrSize(from_nbits)
            } else {
                EmitState::NoLint
            }
        },
        (false, false) => {
            // the types are both a constant known size
            // and do not depend on any specific pointer size to be the same
            if from_nbits == to_nbits {
                EmitState::LintAlways
            } else {
                EmitState::NoLint
            }
        },
    };

    let message = match should_lint {
        EmitState::NoLint => return,
        EmitState::LintAlways => format!("casting `{cast_from}` to `{cast_to}` may wrap around the value"),
        EmitState::LintOnPtrSize(ptr_size) => format!(
            "casting `{cast_from}` to `{cast_to}` may wrap around the value on targets with {ptr_size}-bit wide pointers",
        ),
    };

    span_lint_and_then(cx, CAST_POSSIBLE_WRAP, expr.span, message, |diag| {
        if let EmitState::LintOnPtrSize(16) = should_lint {
            diag
                .note("`usize` and `isize` may be as small as 16 bits on some platforms")
                .note("for more information see https://doc.rust-lang.org/reference/types/numeric.html#machine-dependent-integer-types");
        }
    });
}
