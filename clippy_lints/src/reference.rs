use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::peel_hir_expr_while;
use clippy_utils::res::MaybeResPath as _;
use clippy_utils::source::snippet;
use clippy_utils::sugg::{Sugg, has_enclosing_paren};
use clippy_utils::ty::adjust_derefs_manually_drop;
use rustc_errors::Applicability;
use rustc_hir::{BindingMode, BorrowKind, ByRef, Expr, ExprKind, HirId, Node, PatKind, UnOp};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `*&` and `*&mut` in expressions.
    ///
    /// ### Why is this bad?
    /// Immediately dereferencing a reference is no-op and
    /// makes the code less clear.
    ///
    /// ### Known problems
    /// Multiple dereference/addrof pairs are not handled so
    /// the suggested fix for `x = **&&y` is `x = *&y`, which is still incorrect.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let a = f(*&mut b);
    /// let c = *&d;
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// let a = f(b);
    /// let c = d;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub DEREF_ADDROF,
    complexity,
    "use of `*&` or `*&mut` in an expression"
}

declare_lint_pass!(DerefAddrOf => [DEREF_ADDROF]);

impl<'tcx> LateLintPass<'tcx> for DerefAddrOf {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        if !e.span.from_expansion()
            && let ExprKind::Unary(UnOp::Deref, deref_target) = e.kind
            && !deref_target.span.from_expansion()
            && let ExprKind::AddrOf(borrow_kind, _, addrof_target) = deref_target.kind
            // NOTE(tesuji): `*&` forces rustc to const-promote the array to `.rodata` section.
            // See #12854 for details.
            && !matches!(addrof_target.kind, ExprKind::Array(_))
            && deref_target.span.eq_ctxt(e.span)
            && !addrof_target.span.from_expansion()
        {
            // Removing `*&raw` from a borrowed mutable static can trigger rustc's `static_mut_refs`.
            if borrow_kind == BorrowKind::Raw
                && is_mutable_static_place(cx, addrof_target)
                && is_in_static_mut_refs_borrow_context(cx, e)
            {
                return;
            }

            let mut applicability = Applicability::MachineApplicable;
            let mut sugg = || Sugg::hir_with_applicability(cx, addrof_target, "_", &mut applicability);

            // If this expression is an explicit `DerefMut` of a `ManuallyDrop` reached through a
            // union, we may remove the reference if we are at the point where the implicit
            // dereference would take place. Otherwise, we should not lint.
            let sugg = match is_manually_drop_through_union(cx, e.hir_id, addrof_target) {
                ManuallyDropThroughUnion::Directly => sugg().deref(),
                ManuallyDropThroughUnion::Indirect => return,
                ManuallyDropThroughUnion::No => sugg(),
            };

            let sugg = if has_enclosing_paren(snippet(cx, e.span, "")) {
                sugg.maybe_paren()
            } else {
                sugg
            };

            span_lint_and_sugg(
                cx,
                DEREF_ADDROF,
                e.span,
                "immediately dereferencing a reference",
                "try",
                sugg.to_string(),
                applicability,
            );
        }
    }
}

/// Is this a mutable static, or a field of one?
fn is_mutable_static_place<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    peel_hir_expr_while(expr, |e| {
        if let ExprKind::Field(base, _) = e.kind {
            Some(base)
        } else {
            None
        }
    })
    .basic_res()
    .opt_def_id()
    .is_some_and(|id| cx.tcx.is_mutable_static(id))
}

/// Checks whether `expr` is borrowed in a way `static_mut_refs` lints: by `&`/`&mut`, as the
/// receiver of a `&self`/`&mut self` method or by a `ref` binding, through fields but not indexing.
fn is_in_static_mut_refs_borrow_context<'tcx>(cx: &LateContext<'tcx>, mut expr: &'tcx Expr<'tcx>) -> bool {
    loop {
        match cx.tcx.parent_hir_node(expr.hir_id) {
            Node::Expr(parent) => match parent.kind {
                ExprKind::Field(..) => expr = parent,
                ExprKind::AddrOf(BorrowKind::Ref, ..) => return true,
                ExprKind::MethodCall(_, receiver, ..) if receiver.hir_id == expr.hir_id => {
                    return cx
                        .typeck_results()
                        .type_dependent_def_id(parent.hir_id)
                        .is_some_and(|id| cx.tcx.fn_sig(id).skip_binder().inputs().skip_binder()[0].is_ref());
                },
                _ => return false,
            },
            Node::LetStmt(local) => {
                return matches!(local.pat.kind, PatKind::Binding(BindingMode(ByRef::Yes(..), _), ..));
            },
            _ => return false,
        }
    }
}

/// Is this a `ManuallyDrop` reached through a union, and when is `DerefMut` called on it?
enum ManuallyDropThroughUnion {
    /// `ManuallyDrop` reached through a union and immediately explicitely dereferenced
    Directly,
    /// `ManuallyDrop` reached through a union, and dereferenced later on
    Indirect,
    /// Any other situation
    No,
}

/// Check if `addrof_target` is part of an access to a `ManuallyDrop` entity reached through a
/// union, and when it is dereferenced using `DerefMut` starting from `expr_id` and going up.
fn is_manually_drop_through_union(
    cx: &LateContext<'_>,
    expr_id: HirId,
    addrof_target: &Expr<'_>,
) -> ManuallyDropThroughUnion {
    if is_reached_through_union(cx, addrof_target) {
        let typeck = cx.typeck_results();
        for (idx, id) in std::iter::once(expr_id)
            .chain(cx.tcx.hir_parent_id_iter(expr_id))
            .enumerate()
        {
            if let Node::Expr(expr) = cx.tcx.hir_node(id) {
                if adjust_derefs_manually_drop(typeck.expr_adjustments(expr), typeck.expr_ty(expr)) {
                    return if idx == 0 {
                        ManuallyDropThroughUnion::Directly
                    } else {
                        ManuallyDropThroughUnion::Indirect
                    };
                }
            } else {
                break;
            }
        }
    }
    ManuallyDropThroughUnion::No
}

/// Checks whether `expr` denotes an object reached through a union
fn is_reached_through_union(cx: &LateContext<'_>, mut expr: &Expr<'_>) -> bool {
    while let ExprKind::Field(parent, _) | ExprKind::Index(parent, _, _) = expr.kind {
        if cx.typeck_results().expr_ty_adjusted(parent).is_union() {
            return true;
        }
        expr = parent;
    }
    false
}
