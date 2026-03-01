use clippy_config::Conf;
use clippy_config::types::{DisallowedPath, create_disallowed_map};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::paths::PathNS;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::def_id::DefIdMap;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_session::impl_lint_pass;

use std::borrow::Cow;

declare_clippy_lint! {
    /// ### What it does
    /// Denies the configured methods and functions in clippy.toml
    ///
    /// Note: Even though this lint is warn-by-default, it will only trigger if
    /// methods are defined in the clippy.toml file.
    ///
    /// ### Why is this bad?
    /// Some methods are undesirable in certain contexts, and it's beneficial to
    /// lint for them as needed.
    ///
    /// ### Example
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// disallowed-methods = [
    ///     # Can use a string as the path of the disallowed method.
    ///     "std::boxed::Box::new",
    ///     # Can also use an inline table with a `path` key.
    ///     { path = "std::time::Instant::now" },
    ///     # When using an inline table, can add a `reason` for why the method
    ///     # is disallowed.
    ///     { path = "std::vec::Vec::leak", reason = "no leaking memory" },
    ///     # Can also add a `replacement` that will be offered as a suggestion.
    ///     { path = "std::sync::Mutex::new", reason = "prefer faster & simpler non-poisonable mutex", replacement = "parking_lot::Mutex::new" },
    ///     # Replacement can be specified as a bare name. If you do this, the method name will be
    ///     # replaced without altering the rest of the call.
    ///     #     self.bad_method() becomes self.good_method()
    ///     #     Type::bad_method() becomes Type::good_method()
    ///     { path = "crate::Type::bad_method", replacement = "good_method" }
    ///     # If replacement is specifed as a full path, the call will be converted to a fully-qualified call.
    ///     #     self.evil_method() becomes crate::free_function(self)
    ///     #     Type::evil_method() becomes crate::free_function()
    ///     { path = "crate::Type::evil_method", replacement = "crate::free_function" }
    ///     # This would normally error if the path is incorrect, but with `allow-invalid` = `true`,
    ///     # it will be silently ignored
    ///     { path = "std::fs::InvalidPath", reason = "use alternative instead", allow-invalid = true },
    /// ]
    /// ```
    ///
    /// ```rust,ignore
    /// let xs = vec![1, 2, 3, 4];
    /// xs.leak(); // Vec::leak is disallowed in the config.
    /// // The diagnostic contains the message "no leaking memory".
    ///
    /// let _now = Instant::now(); // Instant::now is disallowed in the config.
    ///
    /// let _box = Box::new(3); // Box::new is disallowed in the config.
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// let mut xs = Vec::new(); // Vec::new is _not_ disallowed in the config.
    /// xs.push(123); // Vec::push is _not_ disallowed in the config.
    /// ```
    #[clippy::version = "1.49.0"]
    pub DISALLOWED_METHODS,
    style,
    "use of a disallowed method call"
}

pub struct DisallowedMethods {
    disallowed: DefIdMap<(&'static str, &'static DisallowedPath)>,
}

impl DisallowedMethods {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf) -> Self {
        let (disallowed, _) = create_disallowed_map(
            tcx,
            &conf.disallowed_methods,
            PathNS::Value,
            |def_kind| {
                matches!(
                    def_kind,
                    DefKind::Fn | DefKind::Ctor(_, CtorKind::Fn) | DefKind::AssocFn
                )
            },
            "function",
            false,
        );
        Self { disallowed }
    }
}

impl_lint_pass!(DisallowedMethods => [DISALLOWED_METHODS]);

impl<'tcx> LateLintPass<'tcx> for DisallowedMethods {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.desugaring_kind().is_some() {
            return;
        }
        let (id, span) = match &expr.kind {
            ExprKind::Path(call_path) if let Res::Def(_, id) = cx.qpath_res(call_path, expr.hir_id) => {
                if let Some(&(path, disallowed_path)) = self.disallowed.get(&id)
                    && let Some(replacement) = disallowed_path.replacement()
                    && !replacement.contains("::")
                    && let &rustc_hir::QPath::Resolved(
                        _,
                        &rustc_hir::Path {
                            segments: &[.., method_name_in_call],
                            ..
                        },
                    )
                    | &rustc_hir::QPath::TypeRelative(_, &method_name_in_call) = call_path
                {
                    // FQP method call with non-fully-qualified replacement.
                    // To support this, we need to only replace the last node in the path, not the whole thing.
                    span_lint_and_then(
                        cx,
                        DISALLOWED_METHODS,
                        expr.span,
                        format!("use of a disallowed method `{path}`"),
                        disallowed_path.diag_amendment(method_name_in_call.ident.span, Applicability::MaybeIncorrect),
                    );
                    return;
                }
                (id, expr.span)
            },
            ExprKind::MethodCall(name, self_expr, params_exprs, call_span)
                if let Some(id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) =>
            {
                if let Some(&(path, disallowed_path)) = self.disallowed.get(&id)
                    && let Some(replacement) = disallowed_path.replacement()
                    && replacement.contains("::")
                {
                    span_lint_and_then(
                        cx,
                        DISALLOWED_METHODS,
                        name.ident.span,
                        format!("use of a disallowed method `{path}`"),
                        |diag| {
                            // FnCtxt is not exported, so we cannot use its `lookup_method` method.
                            // Instead, we convert to a FQP if the user supplies a `::` path here.
                            // Not MachineApplicable because adjustments (auto deref etc) aren't handled.
                            let mut applicability = Applicability::MaybeIncorrect;
                            let self_snippet = snippet_with_applicability(cx, self_expr.span, "..", &mut applicability);
                            let (comma, params_snippet) = if let (Some(first), Some(last)) =
                                (params_exprs.first(), params_exprs.last())
                            {
                                (
                                    ", ",
                                    snippet_with_applicability(cx, first.span.to(last.span), "..", &mut applicability),
                                )
                            } else {
                                ("", Cow::default())
                            };
                            diag.span_suggestion(
                                self_expr.span.to(*call_span),
                                disallowed_path.reason(),
                                format!("{replacement}({self_snippet}{comma}{params_snippet})"),
                                applicability,
                            );
                        },
                    );
                    return;
                }
                (id, name.ident.span)
            },
            _ => return,
        };
        if let Some(&(path, disallowed_path)) = self.disallowed.get(&id) {
            span_lint_and_then(
                cx,
                DISALLOWED_METHODS,
                span,
                format!("use of a disallowed method `{path}`"),
                disallowed_path.diag_amendment(span, Applicability::MaybeIncorrect),
            );
        }
    }
}
