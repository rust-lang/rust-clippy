use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::{implements_trait, is_type_diagnostic_item};
use clippy_utils::{is_from_proc_macro, last_path_segment};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_middle::ty::print::with_forced_trimmed_paths;
use rustc_middle::ty::GenericArgKind;
use rustc_session::declare_lint_pass;
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does.
    /// This lint warns when you use `Arc` with a type that does not implement `Send` or `Sync`.
    ///
    /// ### Why is this bad?
    /// `Arc<T>` is a thread-safe `Rc<T>` and guarantees that updates to the reference counter
    /// use atomic operations. To send an `Arc<T>` across thread boundaries and
    /// share ownership between multiple threads, `T` must be [both `Send` and `Sync`](https://doc.rust-lang.org/std/sync/struct.Arc.html#thread-safety),
    /// so either `T` should be made `Send + Sync` or an `Rc` should be used instead of an `Arc`
    ///
    /// ### Example
    /// ```no_run
    /// # use std::cell::RefCell;
    /// # use std::sync::Arc;
    ///
    /// fn main() {
    ///     // This is fine, as `i32` implements `Send` and `Sync`.
    ///     let a = Arc::new(42);
    ///
    ///     // `RefCell` is `!Sync`, so either the `Arc` should be replaced with an `Rc`
    ///     // or the `RefCell` replaced with something like a `RwLock`
    ///     let b = Arc::new(RefCell::new(42));
    /// }
    /// ```
    #[clippy::version = "1.72.0"]
    pub ARC_WITH_NON_SEND_SYNC,
    suspicious,
    "using `Arc` with a type that does not implement `Send` and `Sync`"
}
declare_lint_pass!(ArcWithNonSendSync => [ARC_WITH_NON_SEND_SYNC]);

impl<'tcx> LateLintPass<'tcx> for ArcWithNonSendSync {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !expr.span.from_expansion()
            && let ty = cx.typeck_results().expr_ty(expr)
            && is_type_diagnostic_item(cx, ty, sym::Arc)
            && let ExprKind::Call(func, [arg]) = expr.kind
            && let ExprKind::Path(func_path) = func.kind
            && last_path_segment(&func_path).ident.name == sym::new
            && let arg_ty = cx.typeck_results().expr_ty(arg)
            // make sure that the type is not and does not contain any type parameters
            && arg_ty.walk().all(|arg| {
                !matches!(arg.unpack(), GenericArgKind::Type(ty) if matches!(ty.kind(), ty::Param(_)))
            })
            && let Some(send) = cx.tcx.get_diagnostic_item(sym::Send)
            && let Some(sync) = cx.tcx.lang_items().sync_trait()
            && let [is_send, is_sync] = [send, sync].map(|id| implements_trait(cx, arg_ty, id, &[]))
            && !(is_send && is_sync)
            && !is_from_proc_macro(cx, expr)
        {
            span_lint_and_then(
                cx,
                ARC_WITH_NON_SEND_SYNC,
                expr.span,
                "usage of an `Arc` that is not `Send` and `Sync`",
                |diag| {
                    with_forced_trimmed_paths!({
                        diag.note(format!("`Arc<{arg_ty}>` is not `Send` and `Sync` as:"));

                        if !is_send {
                            diag.note(format!("- the trait `Send` is not implemented for `{arg_ty}`"));
                        }
                        if !is_sync {
                            diag.note(format!("- the trait `Sync` is not implemented for `{arg_ty}`"));
                        }

                        diag.help("consider using an `Rc` instead. `Arc` does not provide benefits for non `Send` and `Sync` types");

                        diag.note("if you intend to use `Arc` with `Send` and `Sync` traits");

                        diag.note(format!(
                            "wrap the inner type with a `Mutex` or implement `Send` and `Sync` for `{arg_ty}`"
                        ));
                    });
                },
            );
        }
    }
}
