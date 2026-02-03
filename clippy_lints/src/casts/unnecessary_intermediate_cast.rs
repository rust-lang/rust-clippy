use std::cmp::Ordering;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, IntTy, Ty, TyCtxt, UintTy};

use super::UNNECESSARY_INTERMEDIATE_CAST;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    cast_from_expr: &Expr<'tcx>,
    cast_from: Ty<'tcx>,
    cast_mid: Ty<'tcx>,
    cast_to: Ty<'tcx>,
) {
    // If skipping the intermediate cast isn't even allowed in Rust, don't bother checking.
    if !is_cast_allowed(cx.tcx, cast_from, cast_to) {
        return;
    }

    let mut from_class = TypeClass::from(cast_from);
    let mut mid_class = TypeClass::from(cast_mid);
    let mut to_class = TypeClass::from(cast_to);

    if can_remove_intermediate_cast(from_class, mid_class, to_class) {
        let mut applicability = Applicability::MachineApplicable;
        let from_snippet = snippet_with_applicability(cx, cast_from_expr.span, "x", &mut applicability);

        span_lint_and_sugg(
            cx,
            UNNECESSARY_INTERMEDIATE_CAST,
            expr.span,
            format!("intermediate cast is unnecessary (`{cast_from}` -> `{cast_mid}` -> `{cast_to}`)"),
            "try",
            format!("{from_snippet} as {cast_to}"),
            applicability,
        );
        return;
    }

    let pointer_size = cx.tcx.data_layout.pointer_size().bits();
    from_class.set_pointer_size(pointer_size);
    mid_class.set_pointer_size(pointer_size);
    to_class.set_pointer_size(pointer_size);

    if can_remove_intermediate_cast(from_class, mid_class, to_class) {
        // The user may want to keep casts with pointer-sized types, for cross-platform compatibility.
        let mut applicability = Applicability::MaybeIncorrect;
        let from_snippet = snippet_with_applicability(cx, cast_from_expr.span, "x", &mut applicability);

        span_lint_and_sugg(
            cx,
            UNNECESSARY_INTERMEDIATE_CAST,
            expr.span,
            format!("intermediate cast is unnecessary (`{cast_from}` -> `{cast_mid}` -> `{cast_to}`)"),
            "try",
            format!("{from_snippet} as {cast_to}"),
            applicability,
        );
    }
}

/// Returns whether Rust allows casting from `cast_from` to `cast_to`.
/// This function may be incomplete and only considers only types handled by
/// `can_remove_intermediate_cast`, returning `false` for everything else.
fn is_cast_allowed<'tcx>(tcx: TyCtxt<'tcx>, cast_from: Ty<'tcx>, cast_to: Ty<'tcx>) -> bool {
    #[allow(clippy::match_same_arms)]
    match (*cast_from.kind(), *cast_to.kind()) {
        // Integers and floats can indiscriminately cast between each other.
        (ty::Int(..) | ty::Uint(..) | ty::Float(..), ty::Int(..) | ty::Uint(..) | ty::Float(..)) => true,

        // bool and char can cast to any integer.
        (ty::Bool | ty::Char, ty::Int(..) | ty::Uint(..)) => true,

        // Only u8 can cast to char.
        (ty::Uint(UintTy::U8), ty::Char) => true,

        // Pointers can cast to each other and to integers.
        (ty::RawPtr(..), ty::RawPtr(..) | ty::Int(..) | ty::Uint(..)) => true,

        // Integers can only cast to pointers if they are thin.
        (ty::Int(..) | ty::Uint(..), ty::RawPtr(ty, ..)) => ty.has_trivial_sizedness(tcx, ty::SizedTraitKind::Sized),

        _ => false,
    }
}

/// Returns whether it's safe to remove the cast to `cast_mid` without affecting the result.
/// This errs on the side of caution, and should not cause false positives.
fn can_remove_intermediate_cast(from_class: TypeClass, mid_class: TypeClass, to_class: TypeClass) -> bool {
    #[allow(clippy::match_same_arms)]
    match (from_class, mid_class, to_class) {
        // Ignore any type classed as "Other"
        (TypeClass::Other, _, _) | (_, TypeClass::Other, _) | (_, _, TypeClass::Other) => false,

        // Every other type can represent a bool, and sign never matters.
        (TypeClass::Bool, _, _) => true,

        (TypeClass::Int(from_bits, from_signed), TypeClass::Int(mid_bits, mid_signed), TypeClass::Int(to_bits, _)) => {
            match (from_signed, mid_signed) {
                (false, false) | (true, true) => mid_bits >= to_bits || mid_bits >= from_bits,
                (false, true) => mid_bits >= to_bits || mid_bits > from_bits,
                (true, false) => mid_bits >= to_bits,
            }
        },

        (TypeClass::Float(from_bits), TypeClass::Float(mid_bits), TypeClass::Float(to_bits)) => {
            mid_bits >= to_bits || mid_bits >= from_bits
        },

        (TypeClass::Int(from_bits, from_signed), TypeClass::Int(mid_bits, mid_signed), TypeClass::Float(..)) => {
            match (from_signed, mid_signed) {
                (false, false) | (true, true) => mid_bits >= from_bits,
                (false, true) => mid_bits > from_bits,
                (true, false) => false,
            }
        },

        (TypeClass::Int(from_bits, from_signed), TypeClass::Float(mid_bits), TypeClass::Int(to_bits, to_signed)) => {
            match (from_signed, to_signed) {
                (false, false) | (true, true) => mid_bits > from_bits && to_bits >= from_bits,
                (false, true) => mid_bits > from_bits && to_bits > from_bits,
                (true, false) => false,
            }
        },

        (TypeClass::Int(from_bits, _), TypeClass::Float(mid_bits), TypeClass::Float(to_bits)) => {
            mid_bits >= to_bits || mid_bits > from_bits
        },

        (TypeClass::Float(..), TypeClass::Int(..), TypeClass::Int(..)) => false,

        (TypeClass::Float(..), TypeClass::Int(..), TypeClass::Float(..)) => false,

        (TypeClass::Float(from_bits), TypeClass::Float(mid_bits), TypeClass::Int(to_bits, _)) => {
            mid_bits > to_bits || mid_bits >= from_bits
        },

        _ => false,
    }
}

#[derive(Clone, Copy)]
enum TypeClass {
    Other,
    Int(IntSize, bool),
    Bool,
    Float(u64),
}

impl TypeClass {
    fn set_pointer_size(&mut self, pointer_size: u64) {
        if let Self::Int(size @ IntSize::Pointer, _) = self {
            *size = IntSize::Fixed(pointer_size);
        }
    }
}

impl From<Ty<'_>> for TypeClass {
    fn from(ty: Ty<'_>) -> Self {
        #[allow(clippy::match_same_arms)]
        match *ty.kind() {
            ty::Bool => Self::Bool,
            ty::Char => Self::Int(IntSize::Fixed(32), false),
            ty::Int(IntTy::Isize) => Self::Int(IntSize::Pointer, true),
            ty::Int(ty) => {
                let size = ty.bit_width().expect("all other integer types should have fixed sizes");
                Self::Int(IntSize::Fixed(size), true)
            },
            ty::Uint(UintTy::Usize) => Self::Int(IntSize::Pointer, false),
            ty::Uint(ty) => {
                let size = ty.bit_width().expect("all other integer types should have fixed sizes");
                Self::Int(IntSize::Fixed(size), false)
            },
            ty::Float(ty) => Self::Float(ty.bit_width()),
            ty::RawPtr(..) => Self::Int(IntSize::Pointer, false),
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IntSize {
    Fixed(u64),
    Pointer,
}

impl PartialOrd for IntSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        #[allow(clippy::match_same_arms)]
        match (self, other) {
            (IntSize::Fixed(from), IntSize::Fixed(to)) => Some(from.cmp(to)),
            (IntSize::Fixed(_), IntSize::Pointer) => None,
            (IntSize::Pointer, IntSize::Fixed(_)) => None,
            (IntSize::Pointer, IntSize::Pointer) => Some(Ordering::Equal),
        }
    }
}

impl PartialOrd<u64> for IntSize {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        match self {
            IntSize::Fixed(size) => Some(size.cmp(other)),
            IntSize::Pointer => None,
        }
    }
}

impl PartialEq<u64> for IntSize {
    fn eq(&self, other: &u64) -> bool {
        self.partial_cmp(other).is_some_and(Ordering::is_eq)
    }
}

impl PartialOrd<IntSize> for u64 {
    fn partial_cmp(&self, other: &IntSize) -> Option<Ordering> {
        other.partial_cmp(self).map(Ordering::reverse)
    }
}

impl PartialEq<IntSize> for u64 {
    fn eq(&self, other: &IntSize) -> bool {
        other.eq(self)
    }
}
