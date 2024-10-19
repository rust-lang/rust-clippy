use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use clippy_utils::{ReturnType, ReturnVisitor, is_entrypoint_fn, visit_returns};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{BodyId, FnRetTy, FnSig, ImplItemKind, Item, ItemKind, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TypeckResults;
use rustc_middle::ty::adjustment::{Adjust, Adjustment};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects functions that do not return, but do not have `!` as their return type.
    ///
    /// ### Why is this bad?
    ///
    /// Returning `!` is a more accurate API for your callers, and allows for optimisations/further linting.
    ///
    /// ### Example
    /// ```no_run
    /// # fn do_thing() {}
    /// fn run() {
    ///     loop {
    ///         do_thing();
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # fn do_thing() {}
    /// fn run() -> ! {
    ///     loop {
    ///         do_thing();
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.83.0"]
    pub NEVER_RETURNS,
    pedantic,
    "functions that never return, but are typed to"
}

#[derive(Clone, Copy)]
pub(crate) struct NeverReturns {
    avoid_breaking_exported_api: bool,
}

impl_lint_pass!(NeverReturns => [NEVER_RETURNS]);

impl NeverReturns {
    pub fn new(conf: &Conf) -> Self {
        Self {
            avoid_breaking_exported_api: conf.avoid_breaking_exported_api,
        }
    }

    fn check_item_fn(self, cx: &LateContext<'_>, sig: FnSig<'_>, def_id: LocalDefId, body_id: BodyId) {
        let returns_unit = if let FnRetTy::Return(ret_ty) = sig.decl.output {
            if let TyKind::Never = ret_ty.kind {
                return;
            }

            matches!(ret_ty.kind, TyKind::Tup([]))
        } else {
            true
        };

        if self.avoid_breaking_exported_api && cx.effective_visibilities.is_exported(def_id) {
            return;
        }

        // We shouldn't try to change the signature of a lang item!
        if cx.tcx.lang_items().from_def_id(def_id.to_def_id()).is_some() {
            return;
        }

        let body = cx.tcx.hir().body(body_id);
        let typeck_results = cx.tcx.typeck_body(body_id);
        let mut visitor = NeverReturnVisitor {
            typeck_results,
            returns_unit,
            found_implicit_return: false,
        };

        if visit_returns(&mut visitor, body.value).is_continue() && visitor.found_implicit_return {
            let mut applicability = Applicability::MachineApplicable;
            let (lint_span, mut snippet, sugg) = match sig.decl.output {
                FnRetTy::DefaultReturn(span) => (span, String::new(), " -> !"),
                FnRetTy::Return(ret_ty) => {
                    let snippet = if let Some(snippet) = snippet_opt(cx, ret_ty.span) {
                        format!(" a `{snippet}`")
                    } else {
                        applicability = Applicability::HasPlaceholders;
                        String::new()
                    };

                    (ret_ty.span, snippet, "!")
                },
            };

            snippet.insert_str(0, "function never returns, but is typed to return");
            span_lint_and_sugg(
                cx,
                NEVER_RETURNS,
                lint_span,
                snippet,
                "replace with",
                sugg.into(),
                applicability,
            );
        }
    }
}

impl LateLintPass<'_> for NeverReturns {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Fn(sig, _, body_id) = item.kind {
            let local_def_id = item.owner_id.def_id;
            if is_entrypoint_fn(cx, local_def_id.to_def_id()) {
                return;
            }

            self.check_item_fn(cx, sig, local_def_id, body_id);
        } else if let ItemKind::Impl(impl_) = item.kind {
            // Do not lint trait impls
            if impl_.of_trait.is_some() {
                return;
            }

            for impl_item in impl_.items {
                let ImplItemKind::Fn(sig, body_id) = cx.tcx.hir().impl_item(impl_item.id).kind else {
                    continue;
                };

                let local_def_id = item.owner_id.def_id;
                self.check_item_fn(cx, sig, local_def_id, body_id);
            }
        }
    }
}

struct NeverReturnVisitor<'tcx> {
    typeck_results: &'tcx TypeckResults<'tcx>,
    found_implicit_return: bool,
    returns_unit: bool,
}

impl ReturnVisitor for &mut NeverReturnVisitor<'_> {
    type Result = std::ops::ControlFlow<()>;

    fn visit_return(&mut self, kind: ReturnType<'_>) -> Self::Result {
        let expression = match kind {
            ReturnType::Explicit(expr) => expr,
            ReturnType::UnitReturnExplicit(_) => {
                return Self::Result::Break(());
            },
            ReturnType::Implicit(expr) | ReturnType::MissingElseImplicit(expr) => {
                self.found_implicit_return = true;
                expr
            },
            ReturnType::DivergingImplicit(_) => {
                // If this function returns unit, a diverging implicit may just
                // be an implicit unit return, in which case we should not lint.
                return if self.returns_unit {
                    Self::Result::Break(())
                } else {
                    Self::Result::Continue(())
                };
            },
        };

        if expression.span.from_expansion() {
            return Self::Result::Break(());
        }

        let adjustments = self.typeck_results.expr_adjustments(expression);
        if adjustments.iter().any(is_never_to_any) {
            Self::Result::Continue(())
        } else {
            Self::Result::Break(())
        }
    }
}

fn is_never_to_any(adjustment: &Adjustment<'_>) -> bool {
    matches!(adjustment.kind, Adjust::NeverToAny)
}
