use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sugg::Sugg;
use clippy_utils::sym;
use clippy_utils::ty::peel_and_count_ty_refs;
use rustc_ast::{BorrowKind, UnOp};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;
use rustc_span::{Span, Symbol};

use super::CLONES_INTO_BOXED_SLICES;

// Shows the lint with a suggestion using the given parts
// Assures that the inner argument is correctly ref'd/deref'd in the suggestion based on needs_ref
fn show_lint(
    cx: &LateContext<'_>,
    full_span: Span,
    mut inner: &Expr<'_>,
    needs_ref: bool,
    suggestion: (Option<&str>, &str, Option<&str>),
    degrade_app_to: Option<Applicability>,
) {
    let mut applicability = degrade_app_to.unwrap_or(Applicability::MachineApplicable);

    while let ExprKind::AddrOf(BorrowKind::Ref, _, expr) | ExprKind::Unary(UnOp::Deref, expr) = inner.kind {
        inner = expr;
    }

    let mut sugg = Sugg::hir_with_context(cx, inner, full_span.ctxt(), suggestion.1, &mut applicability);

    let inner_ty = cx.typeck_results().expr_ty(inner);
    let (inner_ty_peeled, ref_count, _) = peel_and_count_ty_refs(inner_ty);
    let mut ref_count = ref_count as i128;
    if needs_ref {
        if ty_is_slice_like(cx, inner_ty_peeled) {
            ref_count -= 1;
        } else {
            // Inner argument is in some kind of Rc-like object, so it should be addr_deref'd to get a reference
            // to the underlying slice
            sugg = sugg.addr_deref();
        }
    }
    for _ in 0..ref_count {
        sugg = sugg.deref();
    }
    for _ in 0..-ref_count {
        sugg = sugg.addr();
    }

    span_lint_and_sugg(
        cx,
        CLONES_INTO_BOXED_SLICES,
        full_span,
        "clone into boxed slice",
        "use",
        format!(
            "Box::from({}{}{})",
            suggestion.0.unwrap_or_default(),
            sugg,
            suggestion.2.unwrap_or_default()
        ),
        applicability,
    );
}

// Is the given type a slice, path, or one of the str types
fn ty_is_slice_like(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    ty.is_slice() || ty.is_str() || matches!(ty.opt_diag_name(&cx.tcx), Some(sym::cstr_type | sym::Path | sym::OsStr))
}

// Shows the lint with a suggestion that depends on the types of the inner argument and the
// resulting Box
pub(super) fn check(
    cx: &LateContext<'_>,
    first_method_ty: Ty<'_>,
    second_method_name: Symbol,
    full_span: Span,
    arg: &Expr<'_>,
) {
    let first_ty_diag_name = first_method_ty.opt_diag_name(cx);
    if (second_method_name == sym::into_boxed_c_str && first_ty_diag_name != Some(sym::cstring_type))
        || (second_method_name == sym::into_boxed_os_str && first_ty_diag_name != Some(sym::OsString))
        || (second_method_name == sym::into_boxed_path && first_ty_diag_name != Some(sym::PathBuf))
        || (second_method_name == sym::into_boxed_str && !first_method_ty.is_lang_item(cx, LangItem::String))
        || (second_method_name == sym::into_boxed_slice && first_ty_diag_name != Some(sym::Vec))
    {
        return;
    }

    let arg_ty = cx.typeck_results().expr_ty(arg);
    let inner_ty = arg_ty.peel_refs();
    if ty_is_slice_like(cx, inner_ty) {
        if second_method_name == sym::into_boxed_path && !inner_ty.is_diag_item(cx, sym::Path) {
            // PathBuf's from(...) can convert from other str types,
            // so Path::new(...) must be used to assure resulting Box is the correct type
            show_lint(
                cx,
                full_span,
                arg,
                true,
                (Some("Path::new("), "...", Some(")")),
                Some(Applicability::MaybeIncorrect),
            );
        } else if let ExprKind::Unary(UnOp::Deref, deref_inner) = arg.kind
            && cx
                .typeck_results()
                .expr_ty(deref_inner)
                .is_lang_item(cx, LangItem::OwnedBox)
        {
            // Special case when inner argument is already in a Box: just use Box::clone
            let mut applicability = Applicability::MachineApplicable;
            span_lint_and_sugg(
                cx,
                CLONES_INTO_BOXED_SLICES,
                full_span,
                "clone into boxed slice",
                "use",
                format!(
                    "{}.clone()",
                    snippet_with_applicability(cx, deref_inner.span, "...", &mut applicability)
                ),
                applicability,
            );
        } else {
            // Inner type is a ref to a slice, so it can be directly used in the suggestion
            show_lint(cx, full_span, arg, true, (None, "...", None), None);
        }
    // For all the following the inner type is owned, so they have to be converted to a
    // reference first for the suggestion
    } else if inner_ty.is_lang_item(cx, LangItem::String) {
        show_lint(cx, full_span, arg, false, (None, "(...)", Some(".as_str()")), None);
    } else if let Some(diag) = inner_ty.opt_diag_name(cx) {
        match diag {
            sym::cstring_type => show_lint(cx, full_span, arg, false, (None, "(...)", Some(".as_c_str()")), None),
            sym::PathBuf => show_lint(cx, full_span, arg, false, (None, "(...)", Some(".as_path()")), None),
            sym::Vec => show_lint(cx, full_span, arg, false, (Some("&"), "(...)", Some("[..]")), None),
            sym::OsString => show_lint(cx, full_span, arg, false, (None, "(...)", Some(".as_os_str()")), None),
            _ => (),
        }
    }
}
