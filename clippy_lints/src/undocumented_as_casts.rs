use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_from_proc_macro;
use clippy_utils::source::text_has_marked_comment;
use rustc_hir::{Expr, ExprKind, MutTy, Mutability, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `as` casts that do not have a preceding `// CAST:` comment.
    ///
    /// By default, this lint enforces comments for all `as` casts through
    /// `check_undocumented_as_any_cast = true`.
    ///
    /// ### Why is this bad?
    /// `as` casts are powerful and potentially dangerous. Requiring a documentation comment
    /// ensures the developer has considered the safety and necessity of the conversion.
    ///
    /// Additional scenario-specific options (for example `as *mut` and `as *const`) exist so
    /// existing codebases can roll out enforcement in higher-risk areas first and incrementally
    /// add missing `// CAST:` comments.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let p: *mut u32 = &mut 42_u32;
    /// let _ = p as *mut i32;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// let p: *mut u32 = &mut 42_u32;
    /// // CAST: reason for the cast
    /// let _ = p as *mut i32;
    ///
    ///
    /// let q: *const u32 = &42_u32;
    /// // CAST: reason for the cast
    /// let _ = q as *const i32;
    /// ```
    #[clippy::version = "1.96.0"]
    pub UNDOCUMENTED_AS_CASTS,
    restriction,
    "`as` casts without a `// CAST:` explanation"
}

impl_lint_pass!(UndocumentedAsCasts => [UNDOCUMENTED_AS_CASTS]);

pub struct UndocumentedAsCasts {
    check_undocumented_as_any_cast: bool,
    check_undocumented_as_mut_ptr_cast: bool,
    check_undocumented_as_const_ptr_cast: bool,
}

impl UndocumentedAsCasts {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            check_undocumented_as_any_cast: conf.check_undocumented_as_any_cast,
            check_undocumented_as_mut_ptr_cast: conf.check_undocumented_as_mut_ptr_cast,
            check_undocumented_as_const_ptr_cast: conf.check_undocumented_as_const_ptr_cast,
        }
    }
}

#[derive(Clone, Copy)]
enum AsCastScenario {
    AnyAs,
    MutPtr,
    ConstPtr,
}

impl AsCastScenario {
    fn is_enabled(self, lint: &UndocumentedAsCasts) -> bool {
        match self {
            Self::AnyAs => lint.check_undocumented_as_any_cast,
            Self::MutPtr => lint.check_undocumented_as_mut_ptr_cast,
            Self::ConstPtr => lint.check_undocumented_as_const_ptr_cast,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::AnyAs => "`as` casts without a `// CAST:` explanation",
            Self::MutPtr => "`as *mut` casts without a `// CAST:` explanation",
            Self::ConstPtr => "`as *const` casts without a `// CAST:` explanation",
        }
    }
}

/// Returns all cast scenarios applicable to `cast_to`, with the most specific
/// scenario first. The caller should use the first enabled scenario.
fn as_cast_scenarios(cast_to: &rustc_hir::Ty<'_>) -> &'static [AsCastScenario] {
    match cast_to.kind {
        TyKind::Ptr(MutTy {
            mutbl: Mutability::Mut, ..
        }) => &[AsCastScenario::MutPtr, AsCastScenario::AnyAs],
        TyKind::Ptr(MutTy {
            mutbl: Mutability::Not, ..
        }) => &[AsCastScenario::ConstPtr, AsCastScenario::AnyAs],
        _ => &[AsCastScenario::AnyAs],
    }
}

impl<'tcx> LateLintPass<'tcx> for UndocumentedAsCasts {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        let source_map = cx.sess().source_map();
        if let ExprKind::Cast(_, cast_to) = expr.kind
            && !expr.span.in_external_macro(cx.sess().source_map())
            && !is_from_proc_macro(cx, expr)
            && let Some(scenario) = as_cast_scenarios(cast_to).iter().copied().find(|s| s.is_enabled(self))
            && let Ok(line_info) = source_map.lookup_line(expr.span.lo())
            && let Some(src) = line_info.sf.src.as_deref()
            && text_has_marked_comment(
                src,
                &line_info.sf.lines()[..=line_info.line],
                line_info.sf.start_pos,
                "CAST:",
                true,
            )
            .is_none()
        {
            span_lint_and_help(
                cx,
                UNDOCUMENTED_AS_CASTS,
                expr.span,
                scenario.message(),
                None,
                "consider adding a cast comment on the preceding line",
            );
        }
    }
}
