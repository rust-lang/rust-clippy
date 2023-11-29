//! This lint works by looking at the `min_captures` that `rustc` uses,
//! and checks that for the expression `capture_kind_expr_id`, it would
//! actually borrow normally, if it weren't for the move keyword.
//!
//! In such cases, the move keyword changes the semantics of the code (e.g.
//! without it that capture would be a normal by reference capture, but with
//! move it would get captured by value, and therefore we do not remove the `move`
//! keyword from the closure).
//!
//! A small caveat for the approach above:
//! There's both a borrow and a move of the same value into the closure, e.g.:
//!
//! ```no_run
//! let x = String::new();
//! let closure = move || {
//!     let s = x.as_str(); // L1
//!     println!("{s}");
//!     drop(x); // L2
//! };
//! ```
//!
//! In this case, the `x` `String` gets moved into the closure (because of L2), but
//! it is also borrowed prior to that at L1.
//!
//! `rustc`, in the presence of the `move` keyword automatically assumes that if
//! it borrows a value, it's going to move it into the closure (in the example above at L1,
//! so `capture_kind_expr_id` would point to the use on L1), but here, in the case
//! of this lint, we should behave a little differently, namely we should first look
//! at all the locations where a place is captured, and if any of them actually moves it,
//! the closure would consume the value.
//!
//! The logic for this is handled in `MovedVariablesCtxt::get_required_kind`, where we
//! try to infer the actual min capture kind needed.

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sugg::DiagnosticExt;
use rustc_errors::Applicability;
use rustc_hir::{CaptureBy, Closure, Expr, ExprKind, HirId};
use rustc_hir_typeck::expr_use_visitor as euv;
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::FakeReadCause;
use rustc_middle::ty;
use rustc_middle::ty::UpvarCapture;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for closures and `async` blocks where the `move` is not necessary.
    /// E.g. all the values are captured by value into the closure / `async` block.
    ///
    /// ### Why is this bad?
    /// This pattern is not necessarily bad, but sometimes the `move` keyword is unnecessary,
    /// for example when there's a closure which captures some variables by reference, so
    /// the programmer adds the `move` keyword to move the variables into the closure, but
    /// then later decides that he no longer needs the variables in question, so he removes them
    /// from the body of the closure, but forgets to also remove the `move` keyword.
    /// 
    /// This is really just a strict coding style issue.
    /// 
    /// ### Caveats
    /// There are some cases where this lint will suggest removing the `move` keyword,
    /// but it would be considered idiomatic to keep it.
    /// 
    /// For example, the closure passed to `std::thread::spawn` is usually always written
    /// with the `move` keyword, even if it's not necessary:
    /// 
    /// ```no_run
    /// let a = String::new();
    /// std::thread::spawn(move || {
    ///    // ...
    ///   function_that_does_something_with(a); // a is moved into the closure
    /// });
    /// ```
    /// 
    /// ### Example
    /// ```no_run
    /// let a = String::new();
    /// let closure = move || {
    ///     drop(a);
    /// };
    /// ```
    /// Use instead:
    /// ```no_run
    /// let a = String::new();
    /// let closure = || {
    ///     drop(a);
    /// };
    /// ```
    #[clippy::version = "1.76.0"]
    pub NEEDLESS_MOVE,
    restriction,
    "checks for needless `move`s on closures / `async` blocks"
}

declare_lint_pass!(NeedlessMove => [NEEDLESS_MOVE]);

impl NeedlessMove {
    fn check_closure<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, closure: &'tcx Closure<'tcx>) {
        let CaptureBy::Value { move_kw } = closure.capture_clause else {
            return;
        };

        if move_kw.is_dummy() {
            // async fn ...() {} convert the body to an `async move {}` block,
            // with a DUMMY_SP for the move_kw
            return;
        }

        // Collect moved & borrowed variables from the closure, which the closure *actually* needs.
        let ctx = {
            let mut ctx = MovedVariablesCtxt::default();
            let body = cx.tcx.hir().body(closure.body);
            let infcx = cx.tcx.infer_ctxt().build();
            euv::ExprUseVisitor::new(&mut ctx, &infcx, closure.def_id, cx.param_env, cx.typeck_results())
                .consume_body(body);
            ctx
        };

        let mut lint_result = LintResult::NothingCaptured;

        for captured_place in cx.typeck_results().closure_min_captures_flattened(closure.def_id) {
            let place = &captured_place.place;
            if let Some(ck_expr_id) = captured_place.info.capture_kind_expr_id {
                let required_ck = ctx.get_required_kind(place, ck_expr_id);
                match required_ck {
                    UpvarCapture::ByRef(_) => {
                        // no matter what the old `lint_result` is, we keep the move.
                        lint_result = LintResult::NeedMove;
                    },
                    UpvarCapture::ByValue => {
                        lint_result = match lint_result {
                            LintResult::NothingCaptured | LintResult::Consumed => LintResult::Consumed,
                            LintResult::NeedMove => LintResult::NeedMove,
                        }
                    },
                }
            }
        }

        let note_msg = match lint_result {
            LintResult::NothingCaptured => {
                "there were no captured variables, so the `move` is unnecessary"
            },
            LintResult::Consumed => {
                "there were consumed variables, but no borrowed variables, so the `move` is unnecessary"
            },
            LintResult::NeedMove => {
                // there was a value which would be borrowed if it weren't for the move keyword,
                // so we should keep it, as removing it would change semantics.
                return;
            },
        };

        span_lint_and_then(
            cx,
            NEEDLESS_MOVE,
            expr.span,
            "you seem to use `move`, but the `move` is unnecessary",
            |diag| {
                diag.suggest_remove_item(cx, move_kw, "remove the `move`", Applicability::MachineApplicable);
                diag.note(note_msg);
            },
        );
    }
}

impl<'tcx> LateLintPass<'tcx> for NeedlessMove {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::Closure(closure) = &expr.kind {
            Self::check_closure(cx, expr, closure);
        }
    }
}

enum LintResult {
    /// do not remove the `move` keyword.
    NeedMove,
    Consumed,
    NothingCaptured,
}

#[derive(Debug, Default)]
struct MovedVariablesCtxt<'tcx> {
    // for each base variable, we remember:
    /// The places where it was captured (and consumed, e.g. moved into the closure).
    moved: Vec<(euv::Place<'tcx>, HirId)>,
    /// The places where it was captured by reference (and not consumed).
    captured: Vec<(euv::Place<'tcx>, HirId, ty::BorrowKind)>,
}

impl<'tcx> MovedVariablesCtxt<'tcx> {
    fn get_required_kind(&self, place: &euv::Place<'tcx>, ref_hir_id: HirId) -> UpvarCapture {
        if self
            .moved
            .iter()
            .any(|upvar_ref| upvar_ref.0 == *place || upvar_ref.1 == ref_hir_id)
        {
            UpvarCapture::ByValue
        } else {
            self.captured
                .iter()
                .find(|upvar_ref| upvar_ref.1 == ref_hir_id)
                .map_or(UpvarCapture::ByRef(ty::BorrowKind::ImmBorrow), |it| {
                    UpvarCapture::ByRef(it.2)
                })
        }
    }
}

impl<'tcx> euv::Delegate<'tcx> for MovedVariablesCtxt<'tcx> {
    fn consume(&mut self, cmt: &euv::PlaceWithHirId<'tcx>, hir_id: HirId) {
        if let euv::PlaceBase::Upvar(_) = cmt.place.base {
            self.moved.push((cmt.place.clone(), hir_id));
        }
    }

    fn borrow(&mut self, cmt: &euv::PlaceWithHirId<'tcx>, hir_id: HirId, bk: ty::BorrowKind) {
        if let euv::PlaceBase::Upvar(_) = cmt.place.base {
            self.captured.push((cmt.place.clone(), hir_id, bk));
        }
    }

    fn mutate(&mut self, cmt: &euv::PlaceWithHirId<'tcx>, hir_id: HirId) {
        self.borrow(cmt, hir_id, ty::BorrowKind::MutBorrow);
    }

    fn fake_read(&mut self, _: &euv::PlaceWithHirId<'tcx>, _: FakeReadCause, _: HirId) {}
}
