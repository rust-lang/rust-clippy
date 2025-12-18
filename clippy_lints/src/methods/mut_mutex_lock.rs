use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::expr_custom_deref_adjustment;
use clippy_utils::res::MaybeDef;
use clippy_utils::ty::peel_and_count_ty_refs;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Mutability};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::MUT_MUTEX_LOCK;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, name_span: Span) {
    // given a `recv` like `a.b.mutex`, this returns `[a.b.mutex, a.b, a]`
    let mut projection_chain = std::iter::successors(Some(recv), |recv| {
        if let ExprKind::Index(r, ..) | ExprKind::Field(r, _) = recv.kind {
            Some(r)
        } else {
            None
        }
    });

    if (cx.typeck_results().expr_ty_adjusted(recv))
        .peel_refs()
        .is_diag_item(cx, sym::Mutex)
        // If, somewhere along the projection chain, we stumble upon a field of type `&T`, or dereference a
        // type like `Arc<T>` to `&T`, we no longer have mutable access to the undelying `Mutex`
        && projection_chain.all(|recv| {
            let expr_ty = cx.typeck_results().expr_ty(recv);
            // The reason we don't use `expr_ty_adjusted` here is twofold:
            //
            // Consider code like this:
            // ```rs
            // struct Foo(Mutex<i32>);
            //
            // fn fun(f: &Foo) {
            //     f.0.lock()
            // }
            // ```
            // - In the outermost receiver (`f.0`), the adjusted type would be `&Mutex`, due to an adjustment
            //   performed by `Mutex::lock`.
            // - In the intermediary receivers (here, only `f`), the adjusted type would be fully dereferenced
            //   (`Foo`), which would make us miss the fact that `f` is actually behind a `&` -- this
            //   information is preserved in the pre-adjustment type (`&Foo`)
            peel_and_count_ty_refs(expr_ty).2 != Some(Mutability::Not)
                && expr_custom_deref_adjustment(cx, recv) != Some(Mutability::Not)
        })
    {
        span_lint_and_sugg(
            cx,
            MUT_MUTEX_LOCK,
            name_span,
            "calling `&mut Mutex::lock` unnecessarily locks an exclusive (mutable) reference",
            "change this to",
            "get_mut".to_owned(),
            Applicability::MaybeIncorrect,
        );
    }
}
