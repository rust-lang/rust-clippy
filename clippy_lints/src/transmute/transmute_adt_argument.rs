use super::MUTABLE_ADT_ARGUMENT_TRANSMUTE;
use clippy_utils::diagnostics::span_lint;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, GenericArgKind, Ty};

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, from_ty: Ty<'tcx>, to_ty: Ty<'tcx>) -> bool {
    // assumes walk will return all types in the same order
    let mut from_ty_walker = from_ty.walk();
    let mut to_ty_walker = to_ty.walk();
    let mut found = false;
    while let Some((from_ty, to_ty)) = from_ty_walker.next().zip(to_ty_walker.next()) {
        if let (GenericArgKind::Type(from_ty), GenericArgKind::Type(to_ty)) = (from_ty.kind(), to_ty.kind()) {
            match (from_ty.kind(), to_ty.kind()) {
                (ty::Bool, ty::Bool)
                | (ty::Char, ty::Char)
                | (ty::Int(_), ty::Int(_))
                | (ty::Uint(_), ty::Uint(_))
                | (ty::Float(_), ty::Float(_))
                | (ty::Foreign(_), ty::Foreign(_))
                | (ty::Str, ty::Str)
                | (ty::Array(_, _), ty::Array(_, _))
                | (ty::Pat(_, _), ty::Pat(_, _))
                | (ty::Slice(_), ty::Slice(_))
                | (ty::RawPtr(_, _), ty::RawPtr(_, _))
                | (ty::FnDef(_, _), ty::FnDef(_, _))
                | (ty::FnPtr(_, _), ty::FnPtr(_, _))
                | (ty::UnsafeBinder(_), ty::UnsafeBinder(_))
                | (ty::Dynamic(_, _), ty::Dynamic(_, _))
                | (ty::Closure(_, _), ty::Closure(_, _))
                | (ty::CoroutineClosure(_, _), ty::CoroutineClosure(_, _))
                | (ty::Coroutine(_, _), ty::Coroutine(_, _))
                | (ty::CoroutineWitness(_, _), ty::CoroutineWitness(_, _))
                | (ty::Never, ty::Never)
                | (ty::Tuple(_), ty::Tuple(_))
                | (ty::Alias(_, _), ty::Alias(_, _))
                | (ty::Param(_), ty::Param(_))
                | (ty::Bound(_, _), ty::Bound(_, _))
                | (ty::Placeholder(_), ty::Placeholder(_))
                | (ty::Infer(_), ty::Infer(_))
                | (ty::Error(_), ty::Error(_)) => {},
                (ty::Ref(_, _, from_mut), ty::Ref(_, _, to_mut)) => {
                    if from_mut < to_mut {
                        span_lint(
                            cx,
                            MUTABLE_ADT_ARGUMENT_TRANSMUTE,
                            e.span,
                            format!("transmute of type argument {from_ty} to {to_ty}"),
                        );
                        found = true;
                    }
                },
                (ty::Adt(adt1, _), ty::Adt(adt2, _)) if adt1 == adt2 => {},
                _ => {
                    from_ty_walker.skip_current_subtree();
                    to_ty_walker.skip_current_subtree();
                },
            }
        }
    }
    found
}
