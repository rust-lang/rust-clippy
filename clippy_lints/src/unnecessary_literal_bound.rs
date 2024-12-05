use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{ReturnType, ReturnVisitor, visit_returns};
use rustc_ast::BorrowKind;
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, Expr, ExprKind, FnDecl, FnRetTy, Lit, MutTy, Mutability, PrimTy, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects functions that are written to return a reference to a literal that could return a static reference but instead return a lifetime-bounded reference.
    ///
    /// ### Why is this bad?
    ///
    /// This leaves the caller unable to use the reference as `'static`, causing unneccessary allocations or confusion.
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

enum ReturnTy {
    Str,
    Slice,
    Array,
}

fn fetch_return_mode(cx: &LateContext<'_>, hir_ty: &Ty<'_>) -> Option<ReturnTy> {
    match &hir_ty.kind {
        TyKind::Array(_, _) => Some(ReturnTy::Array),
        TyKind::Slice(_) => Some(ReturnTy::Slice),
        TyKind::Path(other) => {
            if let Res::PrimTy(PrimTy::Str) = cx.qpath_res(other, hir_ty.hir_id) {
                Some(ReturnTy::Str)
            } else {
                None
            }
        },
        _ => None,
    }
}

struct LiteralReturnVisitor {
    return_mode: ReturnTy,
}

impl ReturnVisitor for LiteralReturnVisitor {
    type Result = std::ops::ControlFlow<()>;
    fn visit_return(&mut self, kind: ReturnType<'_>) -> Self::Result {
        let expr = match kind {
            ReturnType::Implicit(e) | ReturnType::Explicit(e) => e,
            ReturnType::UnitReturnExplicit(_) | ReturnType::MissingElseImplicit(_) => {
                panic!("Function which returns a type has a unit return!")
            },
            ReturnType::DivergingImplicit(_) => {
                // If this block is implicitly returning `!`, it can return `&'static T`.
                return Self::Result::Continue(());
            },
        };

        let returns_literal = match self.return_mode {
            ReturnTy::Str => matches!(
                expr.kind,
                ExprKind::Lit(Lit {
                    node: LitKind::Str(..),
                    ..
                })
            ),
            ReturnTy::Slice | ReturnTy::Array => matches!(
                expr.kind,
                ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, Expr {
                    kind: ExprKind::Array(_),
                    ..
                })
            ),
        };

        if returns_literal {
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

        // Check for `-> &str/&[T]/&[T; N]`
        let FnRetTy::Return(ret_hir_ty) = decl.output else {
            return;
        };

        let Some(inner_hir_ty) = extract_anonymous_ref(ret_hir_ty) else {
            return;
        };

        let Some(return_mode) = fetch_return_mode(cx, inner_hir_ty) else {
            return;
        };

        // Check for all return statements returning literals
        if visit_returns(LiteralReturnVisitor { return_mode }, body.value).is_continue() {
            let mut applicability = Applicability::MachineApplicable;
            let snippet = snippet_with_applicability(cx, inner_hir_ty.span, "..", &mut applicability);
            span_lint_and_sugg(
                cx,
                UNNECESSARY_LITERAL_BOUND,
                ret_hir_ty.span,
                "returning a literal unnecessarily tied to the lifetime of arguments",
                "try",
                format!("&'static {snippet}"),
                applicability,
            );
        }
    }
}
