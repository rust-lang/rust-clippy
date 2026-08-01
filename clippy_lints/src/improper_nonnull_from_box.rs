use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _};
use clippy_utils::source::snippet_with_context;
use clippy_utils::sym;
use clippy_utils::visitors::is_expr_unsafe;
use rustc_errors::Applicability;
use rustc_hir::{Block, BlockCheckMode, Expr, ExprKind, LangItem, Node, UnsafeSource};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unsafe usage of `NonNull::new_unchecked(Box::into_raw(x))` or dangerous usage of `NonNull::from_mut(Box::leak(x))` and suggests calling `Box::into_non_null(x)` instead.
    ///
    /// ### Why is this bad?
    /// First, `NonNull::new_unchecked` is an unsafe function, which we don't need to call at all. Second, at the time of writing, whether or not you are allowed to reconstruct the `Box` from the mutable reference returned by `Box::leak` is an [open question](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.leak). Thus, this lint helps prevent future dangerous calls to `Box::from_non_null`.
    ///
    /// ### Example
    /// ```no_run
    /// use std::ptr::NonNull;
    /// let one = Box::new(1);
    /// let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(one)) };
    /// let one = Box::new(1);
    /// let ptr = NonNull::from_mut(Box::leak(one));
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::ptr::NonNull;
    /// let one = Box::new(1);
    /// let ptr = Box::into_non_null(one);
    /// let one = Box::new(1);
    /// let ptr = Box::into_non_null(one);
    /// ```
    #[clippy::version = "1.98.0"]
    pub IMPROPER_NONNULL_FROM_BOX,
    complexity,
    "using `NonNull::new_unchecked` with `Box::into_raw` or `NonNull::from_mut` with `Box::leak`, while `Box::into_non_null` can be used instead"
}

impl_lint_pass!(ImproperNonnullFromBox => [IMPROPER_NONNULL_FROM_BOX]);

pub struct ImproperNonnullFromBox {
    msrv: Msrv,
}

impl ImproperNonnullFromBox {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ImproperNonnullFromBox {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if !expr.span.from_expansion()
            && let ExprKind::Call(expr1, [arg]) = expr.kind
            && let ExprKind::Call(expr2, [arg]) = arg.kind
        {
            if expr1
                .ty_rel_def_if_named(cx, sym::new_unchecked)
                .opt_parent(cx)
                .opt_impl_ty(cx)
                .is_diag_item(cx, sym::NonNull)
                && expr2
                    .ty_rel_def_if_named(cx, sym::into_raw)
                    .opt_parent(cx)
                    .opt_impl_ty(cx)
                    .is_lang_item(cx, LangItem::OwnedBox)
                && self.msrv.meets(cx, msrvs::BOX_INTO_NON_NULL)
            {
                let ctxt = expr.span.ctxt();
                let span = match cx.tcx.parent_hir_node(expr.hir_id) {
                    Node::Block(&Block {
                        rules: BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided),
                        span: unsafe_span,
                        stmts,
                        ..
                    }) if unsafe_span.ctxt() == ctxt && !is_expr_unsafe(cx, arg) && stmts.is_empty() => unsafe_span,
                    _ => expr.span,
                };

                span_lint_and_then(
                    cx,
                    IMPROPER_NONNULL_FROM_BOX,
                    span,
                    "use of `NonNull::new_unchecked` with `Box::into_raw`",
                    |diag| {
                        let mut app = Applicability::MachineApplicable;
                        let arg_name = snippet_with_context(cx, arg.span, ctxt, "_", &mut app).0;

                        diag.span_suggestion(span, "try", format!("Box::into_non_null({arg_name})"), app);
                    },
                );
            } else if expr1
                .ty_rel_def_if_named(cx, sym::from_mut)
                .opt_parent(cx)
                .opt_impl_ty(cx)
                .is_diag_item(cx, sym::NonNull)
                && expr2
                    .ty_rel_def_if_named(cx, sym::leak)
                    .opt_parent(cx)
                    .opt_impl_ty(cx)
                    .is_lang_item(cx, LangItem::OwnedBox)
            {
                span_lint_and_then(
                    cx,
                    IMPROPER_NONNULL_FROM_BOX,
                    expr.span,
                    "use of `NonNull::from_mut` with `Box::leak`",
                    |diag| {
                        let ctxt = expr.span.ctxt();
                        let mut app = Applicability::MachineApplicable;
                        let arg_name = snippet_with_context(cx, arg.span, ctxt, "_", &mut app).0;

                        let sugg = if self.msrv.meets(cx, msrvs::BOX_INTO_NON_NULL) {
                            format!("Box::into_non_null({arg_name})")
                        } else {
                            // No MSRV check here, since the MSRV for `NonNull::from_mut` satisfies
                            // `NonNull::new_unchecked` and `Box::into_raw`.
                            format!("unsafe {{ NonNull::new_unchecked(Box::into_raw({arg_name})) }}")
                        };

                        diag.span_suggestion(expr.span, "try", sugg, app);
                    },
                );
            }
        }
    }
}
