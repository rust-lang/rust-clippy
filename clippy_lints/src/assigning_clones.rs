use clippy_utils::{diagnostics::span_lint_and_then, sugg::Sugg, ty::implements_trait};
use rustc_errors::Applicability;
use rustc_hir::{self as hir, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for code like `foo = bar.clone();`
    ///
    /// ### Why is this bad?
    /// If cloning `bar` allocates memory (or other resources), then this code will allocate
    /// new memory for the clone of `bar`, then drop `foo`, then overwrite `foo` with the clone.
    /// Instead, `Clone::clone_from()`, or `ToOwned::clone_into()`, may be able to update
    /// `foo` in-place, reusing existing memory.
    ///
    /// Note that this only provides any actual improvement if the type has explicitly implemented
    /// the `clone_from()` trait method, since the trait-provided implementation will just call
    /// `clone()`.
    ///
    /// ### Example
    /// ```rust
    /// #[derive(Clone)]
    /// struct Thing;
    ///
    /// pub fn assign_to_ref(a: &mut Thing, b: Thing) {
    ///     *a = b.clone();
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// #[derive(Clone)]
    /// struct Thing;
    ///
    /// pub fn assign_to_ref(a: &mut Thing, b: Thing) {
    ///     a.clone_from(&b);
    /// }
    /// ```
    #[clippy::version = "1.70.0"]
    pub ASSIGNING_CLONES,
    perf,
    "assigning the result of cloning may be inefficient"
}
declare_lint_pass!(AssigningClones => [ASSIGNING_CLONES]);

impl<'tcx> LateLintPass<'tcx> for AssigningClones {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, assign_expr: &'tcx hir::Expr<'_>) {
        let ExprKind::Assign(assign_target, clone_call, _span) = assign_expr.kind else { return };
        // TODO: Also look for `Clone::clone` function calls, not just method calls
        let ExprKind::MethodCall(method_name, clone_receiver, args, _span) = clone_call.kind else { return };

        // Fast syntactic check: if it has args it can't be the call we are looking for,
        // so we don't even need to consult the types.
        if !args.is_empty() {
            return;
        }

        let op = if method_name.ident.name == sym::clone {
            Op::Clone
        } else if method_name.ident.name == sym!(to_owned) {
            Op::ToOwned
        } else {
            return;
        };

        if ok_to_suggest(cx, op, assign_target, clone_call).is_some() {
            suggest(cx, op, assign_expr, assign_target, clone_receiver);
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Op {
    Clone,
    ToOwned,
}

// Return `Some(())` iff we have confirmed that the call is in fact one we want to replace.
fn ok_to_suggest<'tcx>(
    cx: &LateContext<'tcx>,
    op: Op,
    assign_target: &hir::Expr<'tcx>,
    clone_call: &hir::Expr<'tcx>,
) -> Option<()> {
    // Check that the call is actually a call to the trait.
    // TODO: Actually we are currently just checking that the result of the call is
    // a type that implements the trait, which is a bad proxy for it.
    let clone_result_type = cx.typeck_results().expr_ty_adjusted(clone_call);
    if !(implements_trait(cx, clone_result_type, op.expected_trait(cx)?, &[])) {
        return None;
    }

    // If we're assigning to a dereferenced reference, then we know the place is already valid.
    // On the other hand, if the place is a variable or a Box, it might be uninitialized,
    // in which case the suggestion might be wrong.
    // TODO: Actually ask whether the place is uninitialized at this point, instead of
    // guessing based on the syntax and type.
    let ExprKind::Unary(hir::UnOp::Deref, derefed_target_expr) = assign_target.kind
        else { return None };
    if !cx.typeck_results().expr_ty(derefed_target_expr).is_ref() {
        return None;
    }

    Some(())
}

fn suggest<'tcx>(
    cx: &LateContext<'tcx>,
    op: Op,
    assign_expr: &hir::Expr<'tcx>,
    assign_target: &hir::Expr<'tcx>,
    clone_receiver: &hir::Expr<'tcx>,
) {
    span_lint_and_then(cx, ASSIGNING_CLONES, assign_expr.span, op.message(), |diag| {
        // TODO: Make this MachineApplicable once we are more certain that the method being called
        // is what we think it is.
        let mut applicability = Applicability::MaybeIncorrect;

        diag.span_suggestion(
            assign_expr.span,
            op.suggestion_msg(),
            op.suggested_replacement(cx, assign_target, clone_receiver, &mut applicability),
            applicability,
        );
    });
}

impl Op {
    fn expected_trait(self, cx: &LateContext<'_>) -> Option<hir::def_id::DefId> {
        match self {
            Op::Clone => cx.tcx.lang_items().clone_trait(),
            Op::ToOwned => cx.tcx.get_diagnostic_item(sym::ToOwned),
        }
    }

    fn message(self) -> &'static str {
        // TODO: Use the receiver type to say "is" instead of "may be" for types which
        // are known to have optimizations (e.g. `String`).
        match self {
            Op::Clone => "assigning the result of `Clone::clone()` may be inefficient",
            Op::ToOwned => "assigning the result of `ToOwned::to_owned()` may be inefficient",
        }
    }

    fn suggestion_msg(self) -> &'static str {
        match self {
            Op::Clone => "use `clone_from()`",
            Op::ToOwned => "use `clone_into()`",
        }
    }

    fn suggested_replacement<'tcx>(
        self,
        cx: &LateContext<'tcx>,
        assign_target: &hir::Expr<'tcx>,
        clone_receiver: &hir::Expr<'tcx>,
        applicability: &mut Applicability,
    ) -> String {
        match self {
            Op::Clone => {
                // The assignment LHS, which will become the receiver of the `.clone_from()` call,
                // should lose one level of dereference operator since autoref takes care of that.
                let target_sugg = if let ExprKind::Unary(hir::UnOp::Deref, ref_expr) = assign_target.kind {
                    Sugg::hir_with_applicability(cx, ref_expr, "_", applicability)
                } else {
                    Sugg::hir_with_applicability(cx, assign_target, "_", applicability)
                }
                .maybe_par();

                // Determine whether we need to reference the argument to clone_from().
                let clone_receiver_type = cx.typeck_results().expr_ty(clone_receiver);
                let clone_receiver_adj_type = cx.typeck_results().expr_ty_adjusted(clone_receiver);
                let mut clone_source_sugg = Sugg::hir_with_applicability(cx, clone_receiver, "_", applicability);
                if clone_receiver_type != clone_receiver_adj_type {
                    // The receiver may have been a value type, so we need to add an `&` to
                    // be sure the argument to clone_from will be a reference.
                    clone_source_sugg = clone_source_sugg.addr();
                };

                format!("{target_sugg}.clone_from({clone_source_sugg})")
            },
            Op::ToOwned => {
                // If the assignment dereferences, we want the `&mut` that's getting dereferenced.
                // If it doesn't, then we need to *take* a `&mut`.
                // TODO: This doesn't yet handle `DerefMut` types (but it can't meet them)
                let target_sugg = if let ExprKind::Unary(hir::UnOp::Deref, ref_expr) = assign_target.kind {
                    Sugg::hir_with_applicability(cx, ref_expr, "_", applicability)
                } else {
                    // TODO: there is not yet a test for this branch, and there cannot be
                    // until we remove the assigning-to-a-variable restriction.
                    Sugg::hir_with_applicability(cx, assign_target, "_", applicability).mut_addr()
                }
                .maybe_par();

                // We are replacing `foo.to_owned()` with `foo.clone_into(...)`, so the receiver
                // can stay unchanged.
                let receiver_sugg = Sugg::hir_with_applicability(cx, clone_receiver, "_", applicability);

                format!("{receiver_sugg}.clone_into({target_sugg})")
            },
        }
    }
}
