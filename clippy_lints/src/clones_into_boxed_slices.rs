use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sugg::Sugg;
use clippy_utils::sym;
use rustc_ast::{BorrowKind, UnOp};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for clones that are immediately converted into boxed slices instead of using `Box::from(...)`.
    ///
    /// ### Why is this bad?
    /// Using `Box::from(...)` is more concise and avoids creating an unnecessary temporary object.
    ///
    /// ### Example
    /// ```no_run
    /// "example".to_string().to_boxed_str()
    /// ```
    /// Use instead:
    /// ```no_run
    /// Box::from("example")
    /// ```
    #[clippy::version = "1.93.0"]
    pub CLONES_INTO_BOXED_SLICES,
    perf,
    "Cloning then converting into boxed slice instead of using Box::from"
}
declare_lint_pass!(ClonesIntoBoxedSlices => [CLONES_INTO_BOXED_SLICES]);

fn count_refs(mut expr_ty: Ty<'_>) -> i64 {
    let mut count = 0;
    while let ty::Ref(_, inner, _) = expr_ty.kind() {
        expr_ty = *inner;
        count += 1;
    }
    count
}

// Shows the lint with a suggestion using the given parts
// Assures that the inner argument is correctly ref'd/deref'd in the suggestion based on needs_ref
fn show_lint(
    cx: &LateContext<'_>,
    full_span: Span,
    mut inner: &Expr<'_>,
    needs_ref: bool,
    sugg_prefix: Option<&str>,
    placeholder: &str,
    sugg_suffix: Option<&str>,
) {
    let mut applicability = Applicability::MachineApplicable;

    while let ExprKind::AddrOf(BorrowKind::Ref, _, expr) | ExprKind::Unary(UnOp::Deref, expr) = inner.kind {
        inner = expr;
    }

    let mut sugg = Sugg::hir_with_context(cx, inner, full_span.ctxt(), placeholder, &mut applicability);

    let inner_ty = cx.typeck_results().expr_ty(inner);
    let mut ref_count = count_refs(inner_ty);
    if needs_ref {
        if ty_is_slice_like(cx, inner_ty.peel_refs()) {
            ref_count -= 1;
        } else {
            // Inner argument is in some kind of Rc-like object, so it should be addr_deref'd to get a reference
            // to the underlying slice
            sugg = sugg.addr_deref();
        }
    }
    while ref_count > 0 {
        sugg = sugg.deref();
        ref_count -= 1;
    }
    while ref_count < 0 {
        sugg = sugg.addr();
        ref_count += 1;
    }

    span_lint_and_sugg(
        cx,
        CLONES_INTO_BOXED_SLICES,
        full_span,
        "clone into boxed slice",
        "use",
        format!(
            "Box::from({}{}{})",
            sugg_prefix.unwrap_or_default(),
            sugg,
            sugg_suffix.unwrap_or_default()
        ),
        applicability,
    );
}

// Is the given type a slice, path, or one of the str types
fn ty_is_slice_like(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    ty.is_slice()
        || ty.is_str()
        || ty.is_diag_item(cx, sym::cstr_type)
        || ty.is_diag_item(cx, sym::Path)
        || ty.is_diag_item(cx, sym::OsStr)
}

// Checks if an expression is one of the into_boxed_... methods preceded by a clone-like function
// Then shows the lint with a suggestion that depends on the types of the inner argument and the
// resulting Box
impl<'tcx> LateLintPass<'tcx> for ClonesIntoBoxedSlices {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, second_method: &'tcx Expr<'_>) {
        // Is the second method into_boxed_...?
        if let ExprKind::MethodCall(second_method_path, first_method, _, _) = second_method.kind
            && second_method.span.eq_ctxt(first_method.span)
            && [
                sym::into_boxed_str,
                sym::into_boxed_slice,
                sym::into_boxed_path,
                sym::into_boxed_c_str,
                sym::into_boxed_os_str,
            ]
            .contains(&second_method_path.ident.name)
        {
            let arg = match first_method.kind {
                // Is the first method clone-like?
                ExprKind::MethodCall(first_method_path, left, _, _)
                    if [
                        sym::to_owned,
                        sym::clone,
                        sym::to_string,
                        sym::to_path_buf,
                        sym::to_os_string,
                        sym::to_vec,
                    ]
                    .contains(&first_method_path.ident.name) =>
                {
                    Some(left)
                },
                // Also check for from(...) constructor
                ExprKind::Call(
                    Expr {
                        hir_id: _,
                        kind: ExprKind::Path(QPath::TypeRelative(call_out_ty, call_path)),
                        span: _,
                    },
                    args,
                ) if call_path.ident.name == sym::from && cx.typeck_results().expr_ty(&args[0]).is_ref() => {
                    Some(&args[0])
                },
                _ => None,
            };

            if let Some(arg) = arg {
                let full_span = second_method.span.to(first_method.span);
                let arg_ty = cx.typeck_results().expr_ty(arg);
                let inner_ty = arg_ty.peel_refs();
                if ty_is_slice_like(cx, inner_ty) {
                    if second_method_path.ident.name == sym::into_boxed_path && !inner_ty.is_diag_item(cx, sym::Path) {
                        // PathBuf's from(...) can convert from other str types,
                        // so Path::new(...) must be used to assure resulting Box is the correct type
                        show_lint(cx, full_span, arg, true, Some("Path::new("), "...", Some(")"));
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
                        show_lint(cx, full_span, arg, true, None, "...", None);
                    }
                // For all the following the inner type is owned, so they have to be converted to a
                // reference first for the suggestion
                } else if inner_ty.is_lang_item(cx, LangItem::String) {
                    show_lint(cx, full_span, arg, false, None, "(...)", Some(".as_str()"));
                } else if inner_ty.is_diag_item(cx, sym::cstring_type) {
                    show_lint(cx, full_span, arg, false, None, "(...)", Some(".as_c_str()"));
                } else if inner_ty.is_diag_item(cx, sym::PathBuf) {
                    show_lint(cx, full_span, arg, false, None, "(...)", Some(".as_path()"));
                } else if inner_ty.is_diag_item(cx, sym::Vec) {
                    show_lint(cx, full_span, arg, false, Some("&"), "(...)", Some("[..]"));
                } else if inner_ty.is_diag_item(cx, sym::OsString) {
                    show_lint(cx, full_span, arg, false, None, "(...)", Some(".as_os_str()"));
                }
            }
        }
    }
}
