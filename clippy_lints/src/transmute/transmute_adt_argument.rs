use super::MUTABLE_ADT_ARGUMENT_TRANSMUTE;
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::walk::TypeWalker;
use rustc_middle::ty::{self, GenericArgKind, Ty, TyCtxt};

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, from_ty: Ty<'tcx>, to_ty: Ty<'tcx>) -> bool {
    // assumes walk will return all types in the same order
    let mut from_ty_walker = from_ty.walk();
    let mut to_ty_walker = to_ty.walk();
    let mut found = vec![];

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
                | (ty::UnsafeBinder(_), ty::UnsafeBinder(_))
                | (ty::Alias(_, _), ty::Alias(_, _))
                | (ty::Dynamic(_, _), ty::Dynamic(_, _))
                | (ty::CoroutineWitness(_, _), ty::CoroutineWitness(_, _))
                | (ty::Never, ty::Never)
                | (ty::Tuple(_), ty::Tuple(_))
                | (ty::Param(_), ty::Param(_))
                | (ty::Bound(_, _), ty::Bound(_, _))
                | (ty::Placeholder(_), ty::Placeholder(_))
                | (ty::Infer(_), ty::Infer(_))
                | (ty::Error(_), ty::Error(_)) => {},
                (ty::Closure(_, from_r), ty::Closure(_, to_r)) => {
                    let from_r = from_r.as_closure().sig();
                    let to_r = to_r.as_closure().sig();
                    skip_fn_parameters(
                        &mut from_ty_walker,
                        &mut to_ty_walker,
                        from_r.inputs().iter(),
                        to_r.inputs().iter(),
                    );
                },
                (ty::FnPtr(from_r, _), ty::FnPtr(to_r, _)) => {
                    skip_fn_parameters(
                        &mut from_ty_walker,
                        &mut to_ty_walker,
                        from_r.inputs().iter(),
                        to_r.inputs().iter(),
                    );
                },
                (ty::Ref(_, _, from_mut), ty::Ref(_, _, to_mut)) => {
                    if from_mut < to_mut {
                        found.push((from_ty, to_ty));
                    }
                },
                (ty::Adt(adt1, _), ty::Adt(adt2, _)) if adt1 == adt2 => {},
                // for coroutines we they don't return anything so we just skip them (ty::CoroutineClosure(_, _),
                // ty::CoroutineClosure(_, _)) | (ty::Coroutine(_, _), ty::Coroutine(_, _))
                _ => {
                    from_ty_walker.skip_current_subtree();
                    to_ty_walker.skip_current_subtree();
                },
            }
        }
    }
    if found.is_empty() {
        false
    } else {
        span_lint_and_then(
            cx,
            MUTABLE_ADT_ARGUMENT_TRANSMUTE,
            e.span,
            "transmuting references into their mutable version is unsound",
            |diag| {
                found.dedup_by(|ty1, ty2| ty1.1 == ty2.1);
                for (from_ty, to_ty) in found {
                    diag.note(format!("transmute of type argument {from_ty} to {to_ty}"));
                }
            },
        );
        true
    }
}

fn skip_fn_parameters(
    from_ty_walker: &mut TypeWalker<TyCtxt<'_>>,
    to_ty_walker: &mut TypeWalker<TyCtxt<'_>>,
    from_r: impl Iterator,
    to_r: impl Iterator,
) {
    if from_r
        .map(|_| {
            from_ty_walker.next();
            from_ty_walker.skip_current_subtree();
        })
        .count()
        != to_r
            .map(|_| {
                to_ty_walker.next();
                to_ty_walker.skip_current_subtree();
            })
            .count()
    {
        from_ty_walker.next();
        from_ty_walker.skip_current_subtree();
        to_ty_walker.next();
        to_ty_walker.skip_current_subtree();
    }
}
