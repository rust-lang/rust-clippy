use super::MUTABLE_ADT_ARGUMENT_TRANSMUTE;
use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, GenericArg, GenericArgKind, Ty};

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, from_ty: Ty<'tcx>, to_ty: Ty<'tcx>) -> bool {
    // assumes walk will return all types in the same order
    let mut found = vec![];

    walk_ty(&mut found, from_ty, to_ty);
    if found.is_empty() {
        false
    } else {
        span_lint_and_then(
            cx,
            MUTABLE_ADT_ARGUMENT_TRANSMUTE,
            e.span,
            "transmuting references into their mutable version is unsound",
            |diag| {
                found.dedup_by_key(|from_ty| from_ty.1);
                for (from_ty, to_ty) in found {
                    diag.note(format!("transmute of type argument `{from_ty}` to `{to_ty}`"));
                }
            },
        );
        true
    }
}

fn walk_arg<'tcx>(found: &mut Vec<(Ty<'tcx>, Ty<'tcx>)>, from_ty: GenericArg<'tcx>, to_ty: GenericArg<'tcx>) {
    let (GenericArgKind::Type(from_ty), GenericArgKind::Type(to_ty)) = (from_ty.kind(), to_ty.kind()) else {
        return;
    };
    walk_ty(found, from_ty, to_ty);
}
fn walk_ty<'tcx>(found: &mut Vec<(Ty<'tcx>, Ty<'tcx>)>, from_ty: Ty<'tcx>, to_ty: Ty<'tcx>) {
    match (from_ty.kind(), to_ty.kind()) {
        (ty::RawPtr(from_ty, _), ty::RawPtr(to_ty, _))
        | (ty::Slice(from_ty), ty::Slice(to_ty))
        | (ty::Array(from_ty, _), ty::Array(to_ty, _))
        | (ty::Pat(from_ty, _), ty::Pat(to_ty, _)) => {
            walk_ty(found, *from_ty, *to_ty);
        },

        (ty::UnsafeBinder(from_ty), ty::UnsafeBinder(to_ty)) => {
            walk_ty(found, from_ty.skip_binder(), to_ty.skip_binder());
        },
        (ty::Alias(_, from_tys), ty::Alias(_, to_tys)) => {
            from_tys
                .args
                .iter()
                .zip(to_tys.args.iter())
                .for_each(|(from_ty, to_ty)| walk_arg(found, from_ty, to_ty));
        },
        (ty::Dynamic(from_tys, _), ty::Dynamic(to_tys, _)) => {
            from_tys.iter().zip(to_tys.iter()).for_each(|(from_ty, to_ty)| {
                let (args, opt_tys) = match (from_ty.skip_binder(), to_ty.skip_binder()) {
                    (ty::ExistentialPredicate::Trait(from_trait), ty::ExistentialPredicate::Trait(to_trait)) => {
                        (from_trait.args.iter().zip(to_trait.args.iter()), None)
                    },
                    (ty::ExistentialPredicate::Projection(from_p), ty::ExistentialPredicate::Projection(to_p)) => {
                        (from_p.args.iter().zip(to_p.args.iter()), Some((from_p.term, to_p.term)))
                    },
                    _ => return,
                };

                args.chain(
                    opt_tys.and_then(|(from_term, to_term)| match (from_term.kind(), to_term.kind()) {
                        (ty::TermKind::Ty(from_ty), ty::TermKind::Ty(to_ty)) => Some((from_ty.into(), to_ty.into())),
                        _ => None,
                    }),
                )
                .for_each(|(from_ty, to_ty)| walk_arg(found, from_ty, to_ty));
            });
        },

        (ty::Tuple(from_tys), ty::Tuple(to_tys)) => {
            from_tys
                .iter()
                .zip(to_tys.iter())
                .for_each(|(from_ty, to_ty)| walk_ty(found, from_ty, to_ty));
        },
        // it's safe to transmute the parameters of functions/closure
        (ty::Closure(_, from_r), ty::Closure(_, to_r)) => {
            let from_ty = from_r.as_closure().sig().output();
            let to_ty = to_r.as_closure().sig().output();
            walk_ty(found, from_ty.skip_binder(), to_ty.skip_binder());
        },
        // it's safe to transmute the parameters of functions/closure
        (ty::FnPtr(from_r, _), ty::FnPtr(to_r, _)) => {
            walk_ty(found, from_r.output().skip_binder(), to_r.output().skip_binder());
        },
        (ty::Ref(_, from_ty_ref, from_mut), ty::Ref(_, to_ty_ref, to_mut)) => {
            if from_mut < to_mut {
                found.push((from_ty, to_ty));
            }
            walk_ty(found, *from_ty_ref, *to_ty_ref);
        },
        (ty::CoroutineWitness(_, from_tys), ty::CoroutineWitness(_, to_tys)) => {
            from_tys
                .iter()
                .zip(to_tys.iter())
                .for_each(|(from_ty, to_ty)| walk_arg(found, from_ty, to_ty));
        },
        (ty::Adt(adt1, from_tys), ty::Adt(adt2, to_tys)) if adt1 == adt2 => {
            from_tys
                .iter()
                .zip(to_tys.iter())
                .for_each(|(from_ty, to_ty)| walk_arg(found, from_ty, to_ty));
        },
        // for coroutines we they don't return anything so we just skip them, since it's safe to transmute the
        // parameters of functions/closure, and coroutines do not have return types.
        _ => {},
    }
}
