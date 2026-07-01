use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeDef;
use clippy_utils::{SpanlessEq, hash_expr};
use rustc_data_structures::fx::FxIndexMap;
use rustc_hir as hir;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unsafe code calling `.as_ref()` multiple times
    /// on the same generic receiver.
    ///
    /// ### Why is this bad?
    /// Usually `AsRef::as_ref()` is pure and multiple calls will return a
    /// reference to the exact same value. However, unsafe code cannot always
    /// soundly rely on this.
    ///
    /// ### Example
    /// ```no_run
    /// # unsafe fn ffi_call(_ptr: *const u8, _len: usize) {}
    /// fn safe_ffi_call(data: impl AsRef<[u8]>) {
    ///     unsafe {
    ///         ffi_call(data.as_ref().as_ptr(), data.as_ref().len());
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # unsafe fn ffi_call(_ptr: *const u8, _len: usize) {}
    /// fn safe_ffi_call(data: impl AsRef<[u8]>) {
    ///     let data = data.as_ref();
    ///     unsafe {
    ///         ffi_call(data.as_ptr(), data.len());
    ///     }
    /// }
    /// ```
    ///
    /// ### Known problems
    /// False negatives: Such code can also be unsound when the receiver
    /// is a concrete type from a foreign crate.
    ///
    /// False positives: Not all generic implementations are provided by
    /// foreign code. Not all unsafe code actually relies on the purity of
    /// `as_ref()`.
    #[clippy::version = "1.98.0"]
    pub TRUSTED_ASREF,
    suspicious,
    "unsafe block calls `.as_ref()` multiple times on the same generic receiver"
}

declare_lint_pass!(TrustedAsref => [TRUSTED_ASREF]);

impl<'tcx> LateLintPass<'tcx> for TrustedAsref {
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx hir::Block<'tcx>) {
        if block.rules == hir::BlockCheckMode::UnsafeBlock(hir::UnsafeSource::UserProvided)
            && !block.span.in_external_macro(cx.tcx.sess.source_map())
        {
            let mut visitor = ForeignAsRefCallVisitor {
                cx,
                receiver_buckets: FxIndexMap::default(),
            };
            visitor.visit_block(block);

            for bucket in visitor.receiver_buckets.into_values() {
                for calls in bucket {
                    check_calls(cx, calls);
                }
            }
        }
    }
}

fn check_calls(cx: &LateContext<'_>, calls: Calls<'_>) {
    if calls.spans.len() < 2 {
        return;
    }

    span_lint_and_help(
        cx,
        TRUSTED_ASREF,
        calls.spans,
        "unsafe block calls `as_ref` multiple times on the same receiver",
        None,
        "consider calling `as_ref()` once and reusing the result to avoid potential unsoundness",
    );
}

struct Calls<'tcx> {
    receiver: &'tcx hir::Expr<'tcx>,
    spans: Vec<Span>,
}

struct ForeignAsRefCallVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    receiver_buckets: FxIndexMap<u64, Vec<Calls<'tcx>>>,
}

impl<'tcx> Visitor<'tcx> for ForeignAsRefCallVisitor<'_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        if matches!(
            expr.kind,
            hir::ExprKind::Block(
                hir::Block {
                    rules: hir::BlockCheckMode::UnsafeBlock(_),
                    ..
                },
                _
            )
        ) {
            return; // Do not descend into nested unsafe block
        }

        if let hir::ExprKind::MethodCall(_, receiver, _, _) = expr.kind
            && let Some(def_id) = self.cx.typeck_results().type_dependent_def_id(expr.hir_id)
            && def_id.opt_parent(self.cx).is_diag_item(self.cx, sym::AsRef)
            && let ty::Param(_) = self.cx.typeck_results().expr_ty(receiver).peel_refs().kind()
        {
            let receiver_bucket = self.receiver_buckets.entry(hash_expr(self.cx, receiver)).or_default();

            let calls = match receiver_bucket
                .iter_mut()
                .find(|calls| SpanlessEq::new(self.cx).eq_expr(receiver.span.ctxt(), calls.receiver, receiver))
            {
                Some(calls) => calls,
                None => receiver_bucket.push_mut(Calls {
                    receiver,
                    spans: Vec::new(),
                }),
            };

            calls.spans.push(expr.span);
        }

        walk_expr(self, expr);
    }
}
