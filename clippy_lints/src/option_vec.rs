use clippy_utils::{diagnostics::span_lint, is_from_proc_macro, qpath_generic_tys};
use rustc_hir::{Node, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `Option<Vec<T>>`.
    ///
    /// ### Why is this bad?
    /// It's not, but some codebases may wish to create a wrapper that ensures the `Vec` is never
    /// empty. That way, `None` is the empty case. For others, it could be so that an empty `Vec` is
    /// effectively `None`.
    ///
    /// ### Example
    /// ```rust,ignore
    /// struct A {
    ///     numbers: Option<Vec<i32>>),
    /// }
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// /// A wrapper around `Vec` that ensures it's never empty.
    /// struct OccupiedVec<T> { /* snip */ }
    ///
    /// struct A {
    ///     numbers: Option<OccupiedVec<i32>>,
    /// }
    /// ```
    #[clippy::version = "1.72.0"]
    pub OPTION_VEC,
    restriction,
    "disallows usage of `Option<Vec<T>>`"
}
impl_lint_pass!(OptionVec => [OPTION_VEC]);

pub struct OptionVec {
    pub avoid_breaking_exported_api: bool,
}

impl LateLintPass<'_> for OptionVec {
    fn check_ty<'tcx>(&mut self, cx: &LateContext<'tcx>, ty: &Ty<'tcx>) {
        if in_external_macro(cx.sess(), ty.span)
            || (self.avoid_breaking_exported_api && cx.effective_visibilities.is_exported(ty.hir_id.owner.def_id))
        {
            return;
        }

        // Allow something like `Vec<Option<Vec<T>>>`, as silly as it may be
        if let Some(node) = cx.tcx.hir().find_parent(ty.hir_id) && let Node::Ty(..) = node {
            return;
        }

        if let TyKind::Path(option_qpath) = ty.kind
            && let Some(option_def_id) = cx.qpath_res(&option_qpath, ty.hir_id).opt_def_id()
            && cx.tcx.is_diagnostic_item(sym::Option, option_def_id)
            && let [vec_ty] = &*qpath_generic_tys(&option_qpath).collect::<Vec<_>>()
            && let TyKind::Path(vec_qpath) = vec_ty.kind
            && let Some(vec_def_id) = cx.qpath_res(&vec_qpath, vec_ty.hir_id).opt_def_id()
            && cx.tcx.is_diagnostic_item(sym::Vec, vec_def_id)
            && !is_from_proc_macro(cx, ty)
        {
            span_lint(cx, OPTION_VEC, ty.span, "usage of `Option<Vec<T>>`");
        }
    }
}
