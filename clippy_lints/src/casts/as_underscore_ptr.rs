use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_ptr_like;
use rustc_errors::Applicability;
use rustc_hir::{Expr, MutTy, Ty, TyKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, TypeAndMut};

use super::AS_UNDERSCORE_PTR;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, mut target_ty: &Ty<'_>) {
    if let TyKind::Ptr(MutTy { mutbl, .. }) = target_ty.kind {
        // get this before stripping the pointers, so the suggestion suggests replacing the whole type
        let ty_span = target_ty.span;

        // strip all pointers from the type
        while let TyKind::Ptr(MutTy { ty: new_ty, .. }) = target_ty.kind {
            target_ty = new_ty;
        }

        if matches!(target_ty.kind, TyKind::Infer) {
            let mutbl_str = match mutbl {
                rustc_ast::Mutability::Not => "const",
                rustc_ast::Mutability::Mut => "mut",
            };
            span_lint_and_then(
                cx,
                AS_UNDERSCORE_PTR,
                expr.span,
                format!("using `as *{mutbl_str} _` conversion").as_str(),
                |diag| {
                    let ty_resolved = cx.typeck_results().expr_ty(expr);
                    if let ty::Error(_) = ty_resolved.kind() {
                        diag.help("consider giving the type explicitly");
                    } else {
                        // strip the first pointer of the resolved type of the cast, to test if the pointed to type
                        // is also a pointer-like.  This might be a logic error, so bring extra notice to it.
                        if let ty::RawPtr(TypeAndMut { ty: pointee_ty, .. }) = ty_resolved.kind() {
                            if is_ptr_like(cx.tcx, *pointee_ty).is_some() {
                                diag.note("the pointed to type is still a pointer-like type");
                            }
                        } else {
                            unreachable!("The target type of a cast for `as_underscore_ptr` is a pointer");
                        }

                        diag.span_suggestion(
                            ty_span,
                            "consider giving the type explicitly",
                            ty_resolved,
                            Applicability::MachineApplicable,
                        );
                    }
                },
            );
        }
    } else {
        // not a pointer
    }
}
