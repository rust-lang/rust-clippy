use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::sugg::Sugg;
use clippy_utils::{std_or_core, sym};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Mutability, QPath};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `as` casts between raw pointers that change their constness, namely `*const T` to
    /// `*mut T` and `*mut T` to `*const T`.
    ///
    /// ### Why is this bad?
    /// Though `as` casts between raw pointers are not terrible, `pointer::cast_mut` and
    /// `pointer::cast_const` are safer because they cannot accidentally cast the pointer to another
    /// type. Or, when null pointers are involved, `null()` and `null_mut()` can be used directly.
    ///
    /// ### Example
    /// ```no_run
    /// let ptr: *const u32 = &42_u32;
    /// let mut_ptr = ptr as *mut u32;
    /// let ptr = mut_ptr as *const u32;
    /// let ptr1 = std::ptr::null::<u32>() as *mut u32;
    /// let ptr2 = std::ptr::null_mut::<u32>() as *const u32;
    /// let ptr3 = std::ptr::null::<u32>().cast_mut();
    /// let ptr4 = std::ptr::null_mut::<u32>().cast_const();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let ptr: *const u32 = &42_u32;
    /// let mut_ptr = ptr.cast_mut();
    /// let ptr = mut_ptr.cast_const();
    /// let ptr1 = std::ptr::null_mut::<u32>();
    /// let ptr2 = std::ptr::null::<u32>();
    /// let ptr3 = std::ptr::null_mut::<u32>();
    /// let ptr4 = std::ptr::null::<u32>();
    /// ```
    #[clippy::version = "1.72.0"]
    pub PTR_CAST_CONSTNESS,
    pedantic,
    "casting using `as` on raw pointers to change constness when specialized methods apply"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    cast_expr: &Expr<'_>,
    cast_from: Ty<'tcx>,
    cast_to: Ty<'tcx>,
    msrv: Msrv,
) {
    if let ty::RawPtr(from_ty, from_mutbl) = cast_from.kind()
        && let ty::RawPtr(to_ty, to_mutbl) = cast_to.kind()
        && matches!(
            (from_mutbl, to_mutbl),
            (Mutability::Not, Mutability::Mut) | (Mutability::Mut, Mutability::Not)
        )
        && from_ty == to_ty
        && !from_ty.has_erased_regions()
    {
        if let ExprKind::Call(func, []) = cast_expr.kind
            && let ExprKind::Path(QPath::Resolved(None, path)) = func.kind
            && let Some(defid) = path.res.opt_def_id()
            && let Some(prefix) = std_or_core(cx)
            && let mut app = Applicability::MachineApplicable
            && let sugg = format!("{}", Sugg::hir_with_applicability(cx, cast_expr, "_", &mut app))
            && let Some((_, after_lt)) = sugg.split_once("::<")
            && let Some((source, target, target_func)) = match cx.tcx.get_diagnostic_name(defid) {
                Some(sym::ptr_null) => Some(("const", "mutable", "null_mut")),
                Some(sym::ptr_null_mut) => Some(("mutable", "const", "null")),
                _ => None,
            }
        {
            span_lint_and_sugg(
                cx,
                PTR_CAST_CONSTNESS,
                expr.span,
                format!("`as` casting to make a {source} null pointer into a {target} null pointer"),
                format!("use `{target_func}()` directly instead"),
                format!("{prefix}::ptr::{target_func}::<{after_lt}"),
                app,
            );
            return;
        }

        if msrv.meets(cx, msrvs::POINTER_CAST_CONSTNESS) {
            let mut app = Applicability::MachineApplicable;
            let sugg = Sugg::hir_with_context(cx, cast_expr, expr.span.ctxt(), "_", &mut app);
            let constness = match *to_mutbl {
                Mutability::Not => "const",
                Mutability::Mut => "mut",
            };

            span_lint_and_sugg(
                cx,
                PTR_CAST_CONSTNESS,
                expr.span,
                "`as` casting between raw pointers while changing only its constness",
                format!("try `pointer::cast_{constness}`, a safer alternative"),
                format!("{}.cast_{constness}()", sugg.maybe_paren()),
                app,
            );
        }
    }
}

pub(super) fn check_null_ptr_cast_method(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::MethodCall(method, cast_expr, [], _) = expr.kind
        && let ExprKind::Call(func, []) = cast_expr.kind
        && let ExprKind::Path(QPath::Resolved(None, path)) = func.kind
        && let Some(defid) = path.res.opt_def_id()
        && let method = match (cx.tcx.get_diagnostic_name(defid), method.ident.name) {
            (Some(sym::ptr_null), sym::cast_mut) => "null_mut",
            (Some(sym::ptr_null_mut), sym::cast_const) => "null",
            _ => return,
        }
        && let Some(prefix) = std_or_core(cx)
        && let mut app = Applicability::MachineApplicable
        && let sugg = format!("{}", Sugg::hir_with_applicability(cx, cast_expr, "_", &mut app))
        && let Some((_, after_lt)) = sugg.split_once("::<")
    {
        span_lint_and_sugg(
            cx,
            PTR_CAST_CONSTNESS,
            expr.span,
            "changing constness of a null pointer",
            format!("use `{method}()` directly instead"),
            format!("{prefix}::ptr::{method}::<{after_lt}"),
            app,
        );
    }
}
