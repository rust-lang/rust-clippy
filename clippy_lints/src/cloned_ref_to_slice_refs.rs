use std::ops::ControlFlow;

use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef, MaybeTypeckRes};
use clippy_utils::sugg::Sugg;
use clippy_utils::visitors::is_const_evaluatable;
use clippy_utils::{is_clone_like, is_in_const_context, is_mutable};
use rustc_ast::Mutability;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, PathSegment};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_middle::ty::adjustment::{Adjust, OverloadedDeref};
use rustc_session::impl_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for slice references with cloned references such as `&[f.clone()]`.
    ///
    /// ### Why is this bad
    ///
    /// A reference does not need to be owned in order to be used as a slice.
    ///
    /// ### Known problems
    ///
    /// This lint does not know whether or not a clone implementation has side effects.
    ///
    /// ### Example
    ///
    /// ```ignore
    /// let data = 10;
    /// let data_ref = &data;
    /// take_slice(&[data_ref.clone()]);
    /// ```
    /// Use instead:
    /// ```ignore
    /// use std::slice;
    /// let data = 10;
    /// let data_ref = &data;
    /// take_slice(slice::from_ref(data_ref));
    /// ```
    #[clippy::version = "1.89.0"]
    pub CLONED_REF_TO_SLICE_REFS,
    perf,
    "cloning a reference for slice references"
}

pub struct ClonedRefToSliceRefs<'a> {
    msrv: &'a Msrv,
}
impl<'a> ClonedRefToSliceRefs<'a> {
    pub fn new(conf: &'a Conf) -> Self {
        Self { msrv: &conf.msrv }
    }
}

impl_lint_pass!(ClonedRefToSliceRefs<'_> => [CLONED_REF_TO_SLICE_REFS]);

impl<'tcx> LateLintPass<'tcx> for ClonedRefToSliceRefs<'_> {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if self.msrv.meets(cx, {
            if is_in_const_context(cx) {
                msrvs::CONST_SLICE_FROM_REF
            } else {
                msrvs::SLICE_FROM_REF
            }
        })
            // `&[foo.clone()]` expressions
            && let ExprKind::AddrOf(_, mutability, arr) = &expr.kind
            // mutable references would have a different meaning
            && mutability.is_not()

            // check for single item arrays
            && let ExprKind::Array([item]) = &arr.kind

            // check for clones
            && let ExprKind::MethodCall(path, val, _, _) = item.kind
            && let Some(adjustment) = adjust_to_clone_like(cx, item, path, val)

            // check for immutability or purity
            && (!is_mutable(cx, val) || is_const_evaluatable(cx, val))

            // get appropriate crate for `slice::from_ref`
            && let Some(builtin_crate) = clippy_utils::std_or_core(cx)
        {
            let mut applicability = Applicability::MaybeIncorrect;
            let sugg = Sugg::hir_with_context(cx, val, expr.span.ctxt(), "_", &mut applicability);

            span_lint_and_sugg(
                cx,
                CLONED_REF_TO_SLICE_REFS,
                expr.span,
                format!(
                    "unnecessary use of `{}` to create a slice from a reference",
                    path.ident.name
                ),
                "try",
                format!("{builtin_crate}::slice::from_ref({adjustment}{sugg})"),
                applicability,
            );
        }
    }
}

/// If the `expr` is clone-like with proper type, return the adjustment string
fn adjust_to_clone_like(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    path: &PathSegment<'_>,
    recv: &Expr<'_>,
) -> Option<String> {
    let parent_def = cx.ty_based_def(expr).opt_parent(cx)?;
    if !parent_def.is_diag_item(cx, sym::Clone) && !is_clone_like(cx, path.ident.name, parent_def) {
        return None;
    }

    let ret_ty = cx.typeck_results().expr_ty(expr);
    let recv_ty = cx.typeck_results().expr_ty_adjusted(recv);
    let ty::Ref(_, recv_ty_inner, Mutability::Not) = recv_ty.kind() else {
        return None;
    };

    let recv_adjustments = cx.typeck_results().expr_adjustments(recv);

    // Special cases:
    // 1. OsString -> OsStr -> &OsStr
    // 2. PathBuf -> Path -> &Path
    let is_special_case = || {
        if let Some(name) = recv_ty_inner.opt_diag_name(cx)
            && matches!(name, sym::OsStr | sym::Path)
            && let [rest @ .., adjust, _] = recv_adjustments
            && matches!(
                adjust.kind,
                Adjust::Deref(Some(OverloadedDeref {
                    mutbl: Mutability::Not,
                    ..
                }))
            )
            && adjust.target.is_diag_item(cx, name)
            && let source = if let [.., adjust] = rest {
                adjust.target
            } else {
                cx.typeck_results().expr_ty(recv)
            }
            && source.is_diag_item(
                cx,
                match name {
                    sym::OsStr => sym::OsString,
                    sym::Path => sym::PathBuf,
                    _ => unreachable!(),
                },
            )
        {
            return Some(source);
        }

        None
    };

    let target_ty = if ret_ty == *recv_ty_inner {
        ret_ty
    } else {
        is_special_case()?
    };

    let source_ty = cx.typeck_results().expr_ty(recv);
    let adjust_count = recv_adjustments
        .iter()
        .enumerate()
        .try_fold(source_ty, |ty, (i, a)| {
            if ty == target_ty {
                ControlFlow::Break(i)
            } else {
                ControlFlow::Continue(a.target)
            }
        })
        .break_value()?;

    let (needs_borrow, deref_count) = if adjust_count == 0
        || !source_ty.is_ref()
        || recv_adjustments
            .iter()
            .any(|a| matches!(a.kind, Adjust::Deref(Some(_))))
    {
        (true, adjust_count)
    } else {
        (false, adjust_count - 1)
    };

    Some(if needs_borrow {
        format!("&{}", "*".repeat(deref_count))
    } else {
        "*".repeat(deref_count)
    })
}
