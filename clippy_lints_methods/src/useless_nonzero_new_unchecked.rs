use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_and_then};
use clippy_utils::is_inside_always_const_context;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::{Block, BlockCheckMode, Expr, ExprKind, Node, QPath, UnsafeSource};
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for `NonZero*::new_unchecked()` being used in a `const` context.
    ///
    /// ### Why is this bad?
    ///
    /// Using `NonZero*::new_unchecked()` is an `unsafe` function and requires an `unsafe` context. When used in a
    /// context evaluated at compilation time, `NonZero*::new().unwrap()` will provide the same result with identical
    /// runtime performances while not requiring `unsafe`.
    ///
    /// ### Example
    /// ```no_run
    /// use std::num::NonZeroUsize;
    /// const PLAYERS: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(3) };
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::num::NonZeroUsize;
    /// const PLAYERS: NonZeroUsize = NonZeroUsize::new(3).unwrap();
    /// ```
    #[clippy::version = "1.86.0"]
    pub USELESS_NONZERO_NEW_UNCHECKED,
    complexity,
    "using `NonZero::new_unchecked()` in a `const` context"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>, func: &Expr<'tcx>, args: &[Expr<'_>], msrv: Msrv) {
    if let ExprKind::Path(QPath::TypeRelative(ty, segment)) = func.kind
        && segment.ident.name == sym::new_unchecked
        && let [init_arg] = args
        && is_inside_always_const_context(cx.tcx, expr.hir_id)
        && is_type_diagnostic_item(cx, cx.typeck_results().node_type(ty.hir_id), sym::NonZero)
        && msrv.meets(cx, msrvs::CONST_UNWRAP)
    {
        let mut app = Applicability::MachineApplicable;
        let ty_str = snippet_with_applicability(cx, ty.span, "_", &mut app);
        let msg = format!("`{ty_str}::new()` and `Option::unwrap()` can be safely used in a `const` context");
        let sugg = format!(
            "{ty_str}::new({}).unwrap()",
            snippet_with_applicability(cx, init_arg.span, "_", &mut app)
        );

        if let Node::Block(Block {
            stmts: [],
            span: block_span,
            rules: BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided),
            ..
        }) = cx.tcx.parent_hir_node(expr.hir_id)
        {
            if !block_span.from_expansion() {
                // The expression is the only component of an `unsafe` block. Propose
                // to replace the block altogether.
                span_lint_and_sugg(
                    cx,
                    USELESS_NONZERO_NEW_UNCHECKED,
                    *block_span,
                    msg,
                    "use instead",
                    sugg,
                    app,
                );
            }
        } else {
            // The expression is enclosed in a larger `unsafe` context. Indicate that
            // this may no longer be needed for the fixed expression.
            span_lint_and_then(cx, USELESS_NONZERO_NEW_UNCHECKED, expr.span, msg, |diagnostic| {
                diagnostic
                    .span_suggestion(expr.span, "use instead", sugg, app)
                    .note("the fixed expression does not require an `unsafe` context");
            });
        }
    }
}
