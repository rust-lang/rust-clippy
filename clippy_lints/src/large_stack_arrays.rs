use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::snippet;
use rustc_hir::{Expr, ExprKind, Item, ItemKind, Node};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{self, ConstKind};
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for local arrays that may be too large.
    ///
    /// ### Why is this bad?
    /// Large local arrays may cause stack overflow.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let a = [0u32; 1_000_000];
    /// ```
    #[clippy::version = "1.41.0"]
    pub LARGE_STACK_ARRAYS,
    pedantic,
    "allocating large arrays on stack may cause stack overflow"
}

pub struct LargeStackArrays {
    maximum_allowed_size: u128,
}

impl LargeStackArrays {
    #[must_use]
    pub fn new(maximum_allowed_size: u128) -> Self {
        Self { maximum_allowed_size }
    }
}

impl_lint_pass!(LargeStackArrays => [LARGE_STACK_ARRAYS]);

impl<'tcx> LateLintPass<'tcx> for LargeStackArrays {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let ExprKind::Repeat(_, _) = expr.kind
          && let ty::Array(element_type, cst) = cx.typeck_results().expr_ty(expr).kind()
          && let ConstKind::Value(ty::ValTree::Leaf(element_count)) = cst.kind()
          && let Ok(element_count) = element_count.try_to_target_usize(cx.tcx)
          && let Ok(element_size) = cx.layout_of(*element_type).map(|l| l.size.bytes())
          && !cx.tcx.hir().parent_iter(expr.hir_id)
              .any(|(_, node)| matches!(node, Node::Item(Item { kind: ItemKind::Static(..), .. })))
          && self.maximum_allowed_size < u128::from(element_count) * u128::from(element_size) {
              span_lint_and_help(
                  cx,
                  LARGE_STACK_ARRAYS,
                  expr.span,
                  &format!(
                      "allocating a local array larger than {} bytes",
                      self.maximum_allowed_size
                  ),
                  None,
                  &format!(
                      "consider allocating on the heap with `vec!{}.into_boxed_slice()`",
                      snippet(cx, expr.span, "[...]")
                  ),
              );
          }
    }
}
