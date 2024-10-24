use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::{ReturnType, ReturnVisitor, path_res, visit_returns};
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, ExprKind, FnDecl, FnRetTy, Lit, MutTy, Mutability, PrimTy, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects functions that are written to return `&str` that could return `&'static str` but instead return a `&'a str`.
    ///
    /// ### Why is this bad?
    ///
    /// This leaves the caller unable to use the `&str` as `&'static str`, causing unneccessary allocations or confusion.
    /// This is also most likely what you meant to write.
    ///
    /// ### Example
    /// ```no_run
    /// # struct MyType;
    /// impl MyType {
    ///     fn returns_literal(&self) -> &str {
    ///         "Literal"
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # struct MyType;
    /// impl MyType {
    ///     fn returns_literal(&self) -> &'static str {
    ///         "Literal"
    ///     }
    /// }
    /// ```
    /// Or, in case you may return a non-literal `str` in future:
    /// ```no_run
    /// # struct MyType;
    /// impl MyType {
    ///     fn returns_literal<'a>(&'a self) -> &'a str {
    ///         "Literal"
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.83.0"]
    pub UNNECESSARY_LITERAL_BOUND,
    pedantic,
    "detects &str that could be &'static str in function return types"
}

declare_lint_pass!(UnnecessaryLiteralBound => [UNNECESSARY_LITERAL_BOUND]);

fn extract_anonymous_ref<'tcx>(hir_ty: &Ty<'tcx>) -> Option<&'tcx Ty<'tcx>> {
    let TyKind::Ref(lifetime, MutTy { ty, mutbl }) = hir_ty.kind else {
        return None;
    };

    if !lifetime.is_anonymous() || !matches!(mutbl, Mutability::Not) {
        return None;
    }

    Some(ty)
}

struct LiteralReturnVisitor;

impl ReturnVisitor for LiteralReturnVisitor {
    type Result = std::ops::ControlFlow<()>;
    fn visit_return(&mut self, kind: ReturnType<'_>) -> Self::Result {
        let expr = match kind {
            ReturnType::Implicit(e) | ReturnType::Explicit(e) => e,
            ReturnType::UnitReturnExplicit(_) | ReturnType::MissingElseImplicit(_) => {
                panic!("Function which returns `&str` has a unit return!")
            },
            ReturnType::DivergingImplicit(_) => {
                // If this block is implicitly returning `!`, it can return `&'static str`.
                return Self::Result::Continue(());
            },
        };

        if matches!(
            expr.kind,
            ExprKind::Lit(Lit {
                node: LitKind::Str(..),
                ..
            })
        ) {
            Self::Result::Continue(())
        } else {
            Self::Result::Break(())
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryLiteralBound {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        span: Span,
        _: LocalDefId,
    ) {
        if span.from_expansion() {
            return;
        }

        // Checking closures would be a little silly
        if matches!(kind, FnKind::Closure) {
            return;
        }

        // Check for `-> &str`
        let FnRetTy::Return(ret_hir_ty) = decl.output else {
            return;
        };

        let Some(inner_hir_ty) = extract_anonymous_ref(ret_hir_ty) else {
            return;
        };

        if path_res(cx, inner_hir_ty) != Res::PrimTy(PrimTy::Str) {
            return;
        }

        // Check for all return statements returning literals
        if visit_returns(LiteralReturnVisitor, body.value).is_continue() {
            span_lint_and_sugg(
                cx,
                UNNECESSARY_LITERAL_BOUND,
                ret_hir_ty.span,
                "returning a `str` unnecessarily tied to the lifetime of arguments",
                "try",
                "&'static str".into(), // how ironic, a lint about `&'static str` requiring a `String` alloc...
                Applicability::MachineApplicable,
            );
        }
    }
}
