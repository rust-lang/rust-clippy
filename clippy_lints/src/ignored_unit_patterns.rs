use clippy_utils::diagnostics::span_lint_and_sugg;
use hir::{Node, PatKind};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::Mutability;
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `_` in patterns of type `()`.
    ///
    /// ### Why is this bad?
    /// Matching with `()` explicitly instead of `_` outlines
    /// the fact that the pattern contains no data. Also it
    /// would detect a type change that `_` would ignore.
    ///
    /// ### Example
    /// ```no_run
    /// match std::fs::create_dir("tmp-work-dir") {
    ///     Ok(_) => println!("Working directory created"),
    ///     Err(s) => eprintln!("Could not create directory: {s}"),
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// match std::fs::create_dir("tmp-work-dir") {
    ///     Ok(()) => println!("Working directory created"),
    ///     Err(s) => eprintln!("Could not create directory: {s}"),
    /// }
    /// ```
    #[clippy::version = "1.73.0"]
    pub IGNORED_UNIT_PATTERNS,
    pedantic,
    "suggest replacing `_` by `()` in patterns where appropriate"
}
declare_lint_pass!(IgnoredUnitPatterns => [IGNORED_UNIT_PATTERNS]);

impl<'tcx> LateLintPass<'tcx> for IgnoredUnitPatterns {
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx hir::Pat<'tcx>) {
        if matches!(pat.kind, PatKind::Wild)
            && !pat.span.from_expansion()
            && let pat_ty = cx.typeck_results().pat_ty(pat)
            && let (pat_ty, refs) = peel_refs_with_mutabilities(pat_ty)
            && pat_ty.is_unit()
        {
            match cx.tcx.parent_hir_node(pat.hir_id) {
                Node::Param(param) if matches!(cx.tcx.parent_hir_node(param.hir_id), Node::Item(_)) => {
                    // Ignore function parameters
                    return;
                },
                Node::LetStmt(local) if local.ty.is_some() => {
                    // Ignore let bindings with explicit type
                    return;
                },
                _ => {},
            }

            let sugg = refs.into_iter().map(Mutability::ref_prefix_str).chain(["()"]).collect();
            span_lint_and_sugg(
                cx,
                IGNORED_UNIT_PATTERNS,
                pat.span,
                "matching over `()` is more explicit",
                "use `()` instead of `_`",
                sugg,
                Applicability::MachineApplicable,
            );
        }
    }
}

/// Like [`rustc_middle::ty::Ty::peel_refs`], but returns the mutability of each peeled ref
///
/// `&&mut &mut &mut T` returns `(T, [Not, Mut, Mut, Mut])`
fn peel_refs_with_mutabilities(ty: Ty<'_>) -> (Ty<'_>, Vec<Mutability>) {
    let (mut ty, mut refs) = (ty, vec![]);
    while let ty::Ref(_, dest_ty, mutbl) = ty.kind() {
        ty = *dest_ty;
        refs.push(*mutbl);
    }
    (ty, refs)
}
