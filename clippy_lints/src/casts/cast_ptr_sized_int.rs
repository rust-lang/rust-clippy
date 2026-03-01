use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_isize_or_usize;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

use super::CAST_PTR_SIZED_INT;

/// Checks for casts between pointer-sized integer types (`usize`/`isize`) and
/// fixed-size integer types where the behavior depends on the target architecture.
///
/// Some casts are always safe and are NOT linted:
/// - `u8`/`u16` → `usize`: always fits (usize is at least 16-bit)
/// - `i8`/`i16` → `isize`: always fits (isize is at least 16-bit)
/// - `usize` → `u64`/`u128`: always fits (usize is at most 64-bit)
/// - `isize` → `i64`/`i128`: always fits (isize is at most 64-bit)
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>, cast_from: Ty<'tcx>, cast_to: Ty<'tcx>) {
    // Only consider integer-to-integer casts.
    if !cast_from.is_integral() || !cast_to.is_integral() {
        return;
    }

    let from_is_ptr_sized = is_isize_or_usize(cast_from);
    let to_is_ptr_sized = is_isize_or_usize(cast_to);

    // We only care about casts where exactly one side is pointer-sized.
    if from_is_ptr_sized == to_is_ptr_sized {
        return;
    }

    // Identify which side is the pointer-sized type and which is the fixed-size type.
    let (ptr_sized_ty, fixed_ty, fixed_bits_opt, direction) = if from_is_ptr_sized {
        (cast_from, cast_to, fixed_type_bits(cast_to), "to")
    } else {
        (cast_to, cast_from, fixed_type_bits(cast_from), "from")
    };

    let Some(fixed_bits) = fixed_bits_opt else {
        // If the non-pointer side is not a fixed-size integer, bail out.
        return;
    };

    // If this cast is always safe regardless of target pointer width, don't lint.
    if is_always_safe_cast(
        from_is_ptr_sized,
        fixed_bits,
        cast_from.is_signed(),
        cast_to.is_signed(),
    ) {
        return;
    }

    let msg = format!("casting `{cast_from}` to `{cast_to}`: will always truncate");

    span_lint_and_then(cx, CAST_PTR_SIZED_INT, expr.span, msg, |diag| {
        let help_msg = format!(
            "`{ptr_sized_ty}` varies in size depending on the target, \
            so casting {direction} `{fixed_ty}` may produce different results across platforms"
        );
        diag.help(help_msg);
        diag.help("consider using `TryFrom` or `TryInto` for explicit fallible conversions");
    });
}

/// Returns the bit width of a fixed-size integer type, or None if not a fixed-size int.
fn fixed_type_bits(ty: Ty<'_>) -> Option<u64> {
    match ty.kind() {
        ty::Int(int_ty) => int_ty.bit_width(),
        ty::Uint(uint_ty) => uint_ty.bit_width(),
        _ => None,
    }
}

/// Determines if a cast between pointer-sized and fixed-size integers is always safe.
///
/// Always safe casts (no architecture dependency):
/// - Small fixed → ptr-sized: u8/i8/u16/i16 → usize/isize (ptr-sized is at least 16-bit)
/// - Ptr-sized → large fixed: usize/isize → u64/i64/u128/i128 (ptr-sized is at most 64-bit)
///
/// NOT safe (depends on architecture):
/// - Large fixed → ptr-sized: u32/u64/etc → usize (may truncate on smaller ptr widths)
/// - Ptr-sized → small fixed: usize → u8/u16/u32 (may truncate on larger ptr widths)
fn is_always_safe_cast(from_is_ptr_sized: bool, fixed_bits: u64, from_signed: bool, to_signed: bool) -> bool {
    // Note: sign-change issues are handled by a separate lint (cast_sign_loss). Here we
    // only reason about whether the numeric magnitude will always fit regardless of
    // the target pointer width.

    if from_is_ptr_sized {
        // Casting from pointer-sized -> fixed-size:
        // - Pointer-sized integers (usize/isize) are at most 64 bits.
        // - A fixed-size target with >= 64 bits can always hold the magnitude of a pointer-sized value, but
        //   we must respect signedness:
        //   * isize -> i64/i128 is safe (from_signed == true and to_signed && fixed_bits >= 64)
        //   * usize -> u64/u128 is safe (from_signed == false and !to_signed && fixed_bits >= 64)
        if fixed_bits < 64 {
            return false;
        }
        if to_signed {
            // Target is signed: safe only if source is signed (isize -> i64)
            from_signed && fixed_bits >= 64
        } else {
            // Target is unsigned: safe only if source is unsigned (usize -> u64)
            !from_signed && fixed_bits >= 64
        }
    } else if from_signed == to_signed {
        // Casting from fixed-size -> pointer-sized:
        // - Pointer-sized integers are at least 16 bits.
        // - Small fixed-size types (<= 16 bits) always fit in the smallest pointer width.
        // Same signedness: small fixed types (<=16 bits) are always safe.
        fixed_bits <= 16
    } else {
        // Sign change: only the case unsigned small -> signed ptr is considered safe here.
        fixed_bits <= 16 && !from_signed
    }
}
