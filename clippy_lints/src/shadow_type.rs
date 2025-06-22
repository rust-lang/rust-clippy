use clippy_utils::diagnostics::span_lint_and_then;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{GenericParamKind, Generics, HirId, Item, ItemKind, Mod};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::{Ident, Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for generic parameters that shadow types in scope.
    ///
    /// ### Why restrict this?
    /// To avoid confusion and potential bugs, as it can lead to
    /// misunderstandings about which type is being referred to.
    ///
    /// ### Example
    /// ```no_run
    /// struct Foo;
    /// struct Bar<Foo> { f: Foo }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// struct Foo;
    /// struct Bar<F> { f: F } // use different generic parameter name
    /// ```
    #[clippy::version = "1.89.0"]
    pub SHADOW_TYPE_GENERIC,
    restriction,
    "shadowing of type in scope by generic parameter"
}

#[derive(Default)]
pub(crate) struct ShadowTypeGeneric {
    module_types: FxHashMap<Symbol, Span>,
}

impl_lint_pass!(ShadowTypeGeneric => [SHADOW_TYPE_GENERIC]);

impl<'tcx> LateLintPass<'tcx> for ShadowTypeGeneric {
    fn check_mod(&mut self, cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>, _: HirId) {
        self.module_types.clear();
        let items = module.item_ids.iter().map(|&id| cx.tcx.hir_item(id));
        for item in items {
            if item.span.in_external_macro(cx.sess().source_map()) || item.span.from_expansion() {
                continue;
            }

            if let ItemKind::Enum(ident, _, _) | ItemKind::Struct(ident, _, _) = item.kind {
                self.module_types.insert(ident.name, ident.span);
            }
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if item.span.in_external_macro(cx.sess().source_map()) || item.span.from_expansion() {
            return;
        }
        if let ItemKind::Enum(ident, generics, _) | ItemKind::Struct(ident, generics, _) = item.kind {
            self.check(cx, ident, generics);
        }
    }
}

impl ShadowTypeGeneric {
    fn check(&self, cx: &LateContext<'_>, ident: Ident, generics: &Generics<'_>) {
        // Look for generic parameters such as `T`.
        let generic_params = generics
            .params
            .iter()
            .filter(|gen_param| matches!(gen_param.kind, GenericParamKind::Type { .. }));

        // Match generic parameters with module types and split into spans lists.
        let (gen_param_spans, type_spans): (Vec<_>, Vec<_>) = generic_params
            .filter_map(|gen_param| {
                self.module_types
                    .get(&gen_param.name.ident().name)
                    .map(|type_span| (gen_param.span, type_span))
            })
            .unzip();

        let (msg, help) = match gen_param_spans.len() {
            0 => {
                // No generic parameters shadowing types in scope
                return;
            },
            1 => (
                format!("generic parameter in `{ident}` shadows type in scope"),
                "consider using a different name for the generic parameter",
            ),
            _ => (
                format!("generic parameters in `{ident}` shadow types in scope"),
                "consider using different names for the generic parameters",
            ),
        };

        span_lint_and_then(cx, SHADOW_TYPE_GENERIC, gen_param_spans, msg, |diag| {
            diag.span_labels(
                type_spans,
                &format!("this type is being shadowed by a generic parameter in `{ident}`"),
            );
            diag.help(help);
        });
    }
}
