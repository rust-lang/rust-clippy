use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef, MaybeTypeckRes};
use clippy_utils::source::snippet_with_context;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects conversions from `Vec<T>` to `Arc<[T]>` or `Rc<[T]>` via `.into()`,
    /// `Arc::from()`, `Rc::from()`, or `From::from()`.
    ///
    /// ### Why is this bad?
    /// `Arc<[T]>` and `Rc<[T]>` store the reference count and slice data in a single
    /// contiguous allocation. Because `Vec<T>` uses a separate heap allocation with a
    /// different layout, converting `Vec<T>` into `Arc<[T]>` or `Rc<[T]>` must allocate
    /// a new block and **copy** all elements. For large vectors this copy can be expensive.
    ///
    /// Using `Arc<Box<[T]>>` (or `Rc<Box<[T]>>`) avoids the copy:
    /// `Vec::into_boxed_slice()` can reuse the existing allocation (shrinking if needed),
    /// and wrapping the resulting `Box<[T]>` in an `Arc`/`Rc` is a cheap pointer-sized
    /// allocation.
    ///
    /// The trade-off is one extra level of indirection when accessing the data.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::sync::Arc;
    /// let v: Vec<u8> = vec![1, 2, 3];
    /// let a: Arc<[u8]> = v.into();
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::sync::Arc;
    /// let v: Vec<u8> = vec![1, 2, 3];
    /// let a: Arc<Box<[u8]>> = Arc::new(v.into_boxed_slice());
    /// ```
    #[clippy::version = "1.86.0"]
    pub VEC_TO_RC_SLICE,
    perf,
    "converting `Vec<T>` to `Arc<[T]>` or `Rc<[T]>` copies all elements to a new allocation"
}

declare_lint_pass!(VecToRcSlice => [VEC_TO_RC_SLICE]);

/// Checks if `ty` is `Vec<T>` for some `T`.
fn is_vec(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    if let ty::Adt(adt, _) = ty.kind() {
        cx.tcx.is_diagnostic_item(sym::Vec, adt.did())
    } else {
        false
    }
}

/// If `ty` is `Arc<[T]>` or `Rc<[T]>`, returns the wrapper name (`"Arc"` or `"Rc"`).
fn rc_slice_wrapper(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<&'static str> {
    if let ty::Adt(adt, args) = ty.kind()
        && let Some(inner) = args.types().next()
        && inner.is_slice()
    {
        match cx.tcx.get_diagnostic_name(adt.did()) {
            Some(sym::Arc) => Some("Arc"),
            Some(sym::Rc) => Some("Rc"),
            _ => None,
        }
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for VecToRcSlice {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if e.span.from_expansion() {
            return;
        }

        match e.kind {
            // `vec_expr.into()`
            ExprKind::MethodCall(name, recv, [], _) => {
                if name.ident.name == sym::into
                    && cx.ty_based_def(e).opt_parent(cx).is_diag_item(cx, sym::Into)
                    && is_vec(cx, cx.typeck_results().expr_ty(recv))
                    && let Some(wrapper) = rc_slice_wrapper(cx, cx.typeck_results().expr_ty(e))
                {
                    emit_lint(cx, e, recv, wrapper);
                }
            },

            // `Arc::from(vec)` / `Rc::from(vec)` / `From::from(vec)`
            ExprKind::Call(path, [arg]) => {
                if let ExprKind::Path(ref qpath) = path.kind
                    && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
                    && cx.tcx.get_diagnostic_name(def_id) == Some(sym::from_fn)
                    && is_vec(cx, cx.typeck_results().expr_ty(arg))
                    && let Some(wrapper) = rc_slice_wrapper(cx, cx.typeck_results().expr_ty(e))
                {
                    emit_lint(cx, e, arg, wrapper);
                }
            },

            _ => {},
        }
    }
}

fn emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>, vec_expr: &Expr<'_>, wrapper: &str) {
    let mut app = Applicability::MaybeIncorrect;
    let vec_snippet = snippet_with_context(cx, vec_expr.span, expr.span.ctxt(), "<vec>", &mut app).0;
    span_lint_and_then(
        cx,
        VEC_TO_RC_SLICE,
        expr.span,
        format!("converting a `Vec<T>` to `{wrapper}<[T]>` copies all elements to a new allocation"),
        |diag| {
            diag.span_suggestion(
                expr.span,
                "use `into_boxed_slice()` to avoid the copy",
                format!("{wrapper}::new({vec_snippet}.into_boxed_slice())"),
                app,
            );
            diag.note(format!("this gives `{wrapper}<Box<[T]>>` and avoids the copy"));
        },
    );
}
