use super::implicit_clone::is_clone_like;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::{is_copy, is_type_diagnostic_item, should_call_clone_as_function};
use clippy_utils::{is_diag_trait_item, peel_blocks};
use rustc_errors::Applicability;
use rustc_hir::def_id::DefId;
use rustc_hir::{self as hir, LangItem};
use rustc_lint::LateContext;
use rustc_middle::mir::Mutability;
use rustc_middle::ty::adjustment::Adjust;
use rustc_middle::ty::{self, Ty};
use rustc_span::symbol::Ident;
use rustc_span::{Span, sym};

use super::MAP_CLONE;

// If this `map` is called on an `Option` or a `Result` and the previous call is `as_ref`, we don't
// run this lint because it would overlap with `useless_asref` which provides a better suggestion
// in this case.
fn should_run_lint(cx: &LateContext<'_>, e: &hir::Expr<'_>, method_id: DefId) -> bool {
    if is_diag_trait_item(cx, method_id, sym::Iterator) {
        return true;
    }
    // We check if it's an `Option` or a `Result`.
    if let Some(id) = cx.tcx.impl_of_method(method_id) {
        let identity = cx.tcx.type_of(id).instantiate_identity();
        if !is_type_diagnostic_item(cx, identity, sym::Option) && !is_type_diagnostic_item(cx, identity, sym::Result) {
            return false;
        }
    } else {
        return false;
    }
    // We check if the previous method call is `as_ref`.
    if let hir::ExprKind::MethodCall(path1, receiver, _, _) = &e.kind
        && let hir::ExprKind::MethodCall(path2, _, _, _) = &receiver.kind
    {
        return path2.ident.name != sym::as_ref || path1.ident.name != sym::map;
    }

    true
}

pub(super) fn check(cx: &LateContext<'_>, e: &hir::Expr<'_>, recv: &hir::Expr<'_>, arg: &hir::Expr<'_>, msrv: Msrv) {
    if let Some(method_id) = cx.typeck_results().type_dependent_def_id(e.hir_id)
        && should_run_lint(cx, e, method_id)
    {
        match arg.kind {
            hir::ExprKind::Closure(&hir::Closure { body, .. }) => {
                let closure_body = cx.tcx.hir_body(body);
                let closure_expr = peel_blocks(closure_body.value);
                match closure_body.params[0].pat.kind {
                    hir::PatKind::Ref(inner, Mutability::Not) => {
                        if let hir::PatKind::Binding(hir::BindingMode::NONE, .., name, None) = inner.kind
                            && ident_eq(name, closure_expr)
                        {
                            lint_explicit_closure(cx, e.span, recv.span, true, msrv);
                        }
                    },
                    hir::PatKind::Binding(hir::BindingMode::NONE, .., name, None) => {
                        match closure_expr.kind {
                            hir::ExprKind::Unary(hir::UnOp::Deref, inner) => {
                                if ident_eq(name, inner)
                                    && let ty::Ref(.., Mutability::Not) = cx.typeck_results().expr_ty(inner).kind()
                                {
                                    lint_explicit_closure(cx, e.span, recv.span, true, msrv);
                                }
                            },
                            hir::ExprKind::MethodCall(method, obj, [], _) => {
                                if ident_eq(name, obj)
                                && let Some(fn_id) = cx.typeck_results().type_dependent_def_id(closure_expr.hir_id)
                                && (is_clone(cx, method.ident.name, fn_id)
                                    || is_clone_like(cx, method.ident.as_str(), fn_id))
                                // no autoderefs
                                && !cx.typeck_results().expr_adjustments(obj).iter()
                                    .any(|a| matches!(a.kind, Adjust::Deref(Some(..))))
                                && let obj_ty = cx.typeck_results().expr_ty_adjusted(obj)
                                && let ty::Ref(_, ty, Mutability::Not) = obj_ty.kind()
                                // Verify that the method call's output type is the same as its input type. This is to
                                // avoid cases like `to_string` being called on a `&str`, or `to_vec` being called on a
                                // slice.
                                && *ty == cx.typeck_results().expr_ty_adjusted(closure_expr)
                                {
                                    let obj_ty_unadjusted = cx.typeck_results().expr_ty(obj);
                                    if obj_ty == obj_ty_unadjusted {
                                        let copy = is_copy(cx, *ty);
                                        lint_explicit_closure(cx, e.span, recv.span, copy, msrv);
                                    } else {
                                        // To avoid issue #6299, ensure `obj` is not a mutable reference.
                                        if !matches!(obj_ty_unadjusted.kind(), ty::Ref(_, _, Mutability::Mut)) {
                                            lint_needless_cloning(cx, e.span, recv.span);
                                        }
                                    }
                                }
                            },
                            hir::ExprKind::Call(call, [arg]) => {
                                if let hir::ExprKind::Path(qpath) = call.kind
                                    && ident_eq(name, arg)
                                {
                                    handle_path(cx, call, &qpath, e, recv);
                                }
                            },
                            _ => {},
                        }
                    },
                    _ => {},
                }
            },
            hir::ExprKind::Path(qpath) => handle_path(cx, arg, &qpath, e, recv),
            _ => {},
        }
    }
}

fn is_clone(cx: &LateContext<'_>, method: rustc_span::Symbol, fn_id: DefId) -> bool {
    if method == sym::clone
        && let Some(trait_id) = cx.tcx.trait_of_item(fn_id)
        && cx.tcx.lang_items().clone_trait() == Some(trait_id)
    {
        true
    } else {
        false
    }
}

fn handle_path(
    cx: &LateContext<'_>,
    arg: &hir::Expr<'_>,
    qpath: &hir::QPath<'_>,
    e: &hir::Expr<'_>,
    recv: &hir::Expr<'_>,
) {
    let method = || match qpath {
        hir::QPath::TypeRelative(_, method) => Some(method),
        _ => None,
    };
    if let Some(path_def_id) = cx.qpath_res(qpath, arg.hir_id).opt_def_id()
        && (cx.tcx.lang_items().get(LangItem::CloneFn) == Some(path_def_id)
            || method().is_some_and(|method| is_clone_like(cx, method.ident.name.as_str(), path_def_id)))
        // The `copied` and `cloned` methods are only available on `&T` and `&mut T` in `Option`
        // and `Result`.
        // Previously, `handle_path` would make this determination by checking whether `recv`'s type
        // was instantiated with a reference. However, that approach can produce false negatives.
        // Consider `std::slice::Iter`, for example. Even if it is instantiated with a non-reference
        // type, its `map` method will expect a function that operates on references.
        && let Some(ty) = clone_like_referent(cx, arg)
        && !should_call_clone_as_function(cx, ty)
    {
        lint_path(cx, e.span, recv.span, is_copy(cx, ty.peel_refs()));
    }
}

/// Determine whether `expr` is function whose input type is `&T` and whose output type is `T`. If
/// so, return `T`.
fn clone_like_referent<'tcx>(cx: &LateContext<'tcx>, expr: &hir::Expr<'_>) -> Option<Ty<'tcx>> {
    let ty = cx.typeck_results().expr_ty_adjusted(expr);
    if let ty::FnDef(def_id, generic_args) = ty.kind()
        && let tys = cx
            .tcx
            .fn_sig(def_id)
            .instantiate(cx.tcx, generic_args)
            .skip_binder()
            .inputs_and_output
        && let [input_ty, output_ty] = tys.as_slice()
        && let ty::Ref(_, referent_ty, Mutability::Not) = input_ty.kind()
        && referent_ty == output_ty
    {
        Some(*referent_ty)
    } else {
        None
    }
}

fn ident_eq(name: Ident, path: &hir::Expr<'_>) -> bool {
    if let hir::ExprKind::Path(hir::QPath::Resolved(None, path)) = path.kind {
        path.segments.len() == 1 && path.segments[0].ident == name
    } else {
        false
    }
}

fn lint_needless_cloning(cx: &LateContext<'_>, root: Span, receiver: Span) {
    span_lint_and_sugg(
        cx,
        MAP_CLONE,
        root.trim_start(receiver).unwrap(),
        "you are needlessly cloning iterator elements",
        "remove the `map` call",
        String::new(),
        Applicability::MachineApplicable,
    );
}

fn lint_path(cx: &LateContext<'_>, replace: Span, root: Span, is_copy: bool) {
    let mut applicability = Applicability::MachineApplicable;

    let replacement = if is_copy { "copied" } else { "cloned" };

    span_lint_and_sugg(
        cx,
        MAP_CLONE,
        replace,
        "you are explicitly cloning with `.map()`",
        format!("consider calling the dedicated `{replacement}` method"),
        format!(
            "{}.{replacement}()",
            snippet_with_applicability(cx, root, "..", &mut applicability),
        ),
        applicability,
    );
}

fn lint_explicit_closure(cx: &LateContext<'_>, replace: Span, root: Span, is_copy: bool, msrv: Msrv) {
    let mut applicability = Applicability::MachineApplicable;

    let (message, sugg_method) = if is_copy && msrv.meets(cx, msrvs::ITERATOR_COPIED) {
        ("you are using an explicit closure for copying elements", "copied")
    } else {
        ("you are using an explicit closure for cloning elements", "cloned")
    };

    span_lint_and_sugg(
        cx,
        MAP_CLONE,
        replace,
        message,
        format!("consider calling the dedicated `{sugg_method}` method"),
        format!(
            "{}.{sugg_method}()",
            snippet_with_applicability(cx, root, "..", &mut applicability),
        ),
        applicability,
    );
}
