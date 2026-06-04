use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::path_to_local_with_projections;
use clippy_utils::visitors::for_each_expr_without_closures;
use rustc_hir::{self as hir, PatExprKind, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Symbol, sym};
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Warns when calling `.build()` on a `derive_builder`-generated builder without
    /// setting all required fields (those without `#[builder(default)]`).
    ///
    /// ### Why is this bad?
    /// Missing required fields cause a runtime panic or error from `.build()`,
    /// which the compiler cannot catch.
    ///
    /// ### Example
    /// ```no_run
    /// // Bad: `timeout` is required but never set
    /// let _ = FooBuilder::default().build();
    ///
    /// // Good
    /// let _ = FooBuilder::default().timeout(30).build();
    /// ```
    #[clippy::version = "1.87.0"]
    pub MISSING_BUILDER_FIELDS,
    correctness,
    "calling `.build()` on a builder with missing required fields"
}

declare_lint_pass!(MissingBuilderFields => [MISSING_BUILDER_FIELDS]);

impl<'tcx> LateLintPass<'tcx> for MissingBuilderFields {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        let hir::ExprKind::MethodCall(method_segment, receiver, args, _) = expr.kind else {
            return;
        };

        if method_segment.ident.name.as_str() != "build" || !args.is_empty() {
            return;
        }

        // Get the receiver type — should be FooBuilder (peel &mut T since setters return &mut Self)
        let receiver_ty = cx.typeck_results().expr_ty(receiver).peel_refs();
        let ty::Adt(builder_adt, _) = receiver_ty.kind() else {
            return;
        };

        // Get the return type of .build() — may be Result<Foo, E> or Foo directly
        let return_ty = cx.typeck_results().expr_ty(expr);
        let target_ty = if let ty::Adt(adt, args) = return_ty.kind()
            && cx.tcx.is_diagnostic_item(sym::Result, adt.did())
        {
            args.type_at(0)
        } else {
            return_ty
        };

        let ty::Adt(target_adt, _) = target_ty.kind() else {
            return;
        };

        // Heuristic: FooBuilder builds Foo — confirm the names match
        let builder_name = cx.tcx.def_path_str(builder_adt.did());
        let target_name = cx.tcx.def_path_str(target_adt.did());
        if !builder_name.ends_with("Builder") || builder_name[..builder_name.len() - "Builder".len()] != target_name {
            return;
        }

        // Get the body of the build() method to determine required fields.
        // Both #[derive(Builder)] and #[builder(default)] are stripped before the HIR,
        // so we analyze the generated build() body instead.
        let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
            return;
        };
        let Some(local_def_id) = method_def_id.as_local() else {
            return;
        };
        let Some(body) = cx.tcx.hir_maybe_body_owned_by(local_def_id) else {
            return;
        };

        // Walk the build() body looking for patterns that indicate a required field:
        //   match self.FIELD { ..., None => return ..., ... }
        //   self.FIELD.ok_or(...)
        let required_fields = collect_required_fields(body.value);

        if required_fields.is_empty() {
            return;
        }

        // Walk the caller's method chain to see which setters were called.
        // Returns None when the chain root is a local variable (we can't know its history).
        let Some(called) = collect_method_chain(receiver) else {
            return;
        };

        let missing: Vec<&str> = required_fields
            .iter()
            .filter(|&&name| !called.contains(&name))
            .map(Symbol::as_str)
            .collect();

        if missing.is_empty() {
            return;
        }

        span_lint_and_help(
            cx,
            MISSING_BUILDER_FIELDS,
            expr.span,
            format!(
                "builder is missing required field{}: {}",
                if missing.len() == 1 { "" } else { "s" },
                missing.join(", ")
            ),
            None,
            "call the setter for each required field before `.build()`",
        );
    }
}

/// Walk a method call chain and collect all method names, from outermost to root.
/// Returns `None` when the chain root is a local variable — in that case we cannot
/// know which setters were called before this expression, so we skip to avoid false positives.
fn collect_method_chain(mut expr: &hir::Expr<'_>) -> Option<Vec<Symbol>> {
    let mut methods = Vec::new();
    loop {
        if let hir::ExprKind::MethodCall(seg, recv, _, _) = expr.kind {
            methods.push(seg.ident.name);
            expr = recv;
        } else {
            // If the chain root is a local variable we can't track its prior setters.
            if path_to_local_with_projections(expr).is_some() {
                return None;
            }
            return Some(methods);
        }
    }
}

/// Walk a `build()` method body and collect names of required fields.
/// Required fields are identified by two patterns `derive_builder` generates:
///   1. `match self.FIELD { ..., None => return ..., ... }`
///   2. `self.FIELD.ok_or(...)` / `self.FIELD.ok_or_else(...)`
fn collect_required_fields(body: &hir::Expr<'_>) -> Vec<Symbol> {
    let mut fields = Vec::new();
    for_each_expr_without_closures(body, |expr| -> ControlFlow<(), ()> {
        // Pattern 1: match self.FIELD { ..., None => return ..., ... }
        if let hir::ExprKind::Match(scrutinee, arms, _) = expr.kind
            && let hir::ExprKind::Field(_, field_ident) = scrutinee.kind
            && arms.iter().any(|arm| is_none_pat(arm.pat) && is_return(arm.body))
        {
            fields.push(field_ident.name);
            return ControlFlow::Continue(());
        }
        // Pattern 2: self.FIELD.ok_or(...) / self.FIELD.ok_or_else(...)
        if let hir::ExprKind::MethodCall(seg, recv, _, _) = expr.kind
            && matches!(seg.ident.name.as_str(), "ok_or" | "ok_or_else")
            && let hir::ExprKind::Field(_, field_ident) = recv.kind
        {
            fields.push(field_ident.name);
            return ControlFlow::Continue(());
        }
        ControlFlow::Continue(())
    });
    fields
}

fn is_none_pat(pat: &hir::Pat<'_>) -> bool {
    if let PatKind::Expr(pat_expr) = pat.kind
        && let PatExprKind::Path(QPath::Resolved(_, path)) = pat_expr.kind
    {
        path.segments.last().is_some_and(|seg| seg.ident.name == sym::None)
    } else {
        false
    }
}

fn is_return(expr: &hir::Expr<'_>) -> bool {
    matches!(expr.kind, hir::ExprKind::Ret(_))
}
