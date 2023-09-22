use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use clippy_utils::is_trait_impl_item;
use hir::intravisit::FnKind;
use hir::{Body, FnDecl};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::def_id::LocalDefId;
use rustc_span::symbol::Ident;
use rustc_span::{Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Looks for methods in trait impls and struct impls with the same name,
    /// as well as method call sites.
    ///
    /// ### Why is this bad?
    /// It is easy to accidentally override a trait method impl with a method
    /// of the same name in a struct impl. Inherent methods are preferred if
    /// the call site is unqualified, and naming conflicts will happen silently.
    ///
    /// ### Example
    /// ```rust
    /// trait MyTrait {
    ///     fn ambiguous(&self);
    /// }
    ///
    /// struct Base;
    ///
    /// impl Base {
    ///     fn ambiguous(&self) {
    ///         println!("struct impl");
    ///     }
    /// }
    ///
    /// impl MyTrait for Base {
    ///     fn ambiguous(&self) {
    ///         println!("trait impl");
    ///     }
    /// }
    ///
    /// Base.ambiguous(); // prints "struct impl"
    /// ```
    /// Use instead:
    /// ```rust
    /// trait MyTrait {
    ///     fn ambiguous(&self);
    /// }
    ///
    /// struct Base;
    ///
    /// impl Base {
    ///     fn unambiguous(&self) {
    ///         println!("struct impl");
    ///     }
    /// }
    ///
    /// impl MyTrait for Base {
    ///     fn ambiguous(&self) {
    ///         println!("trait impl");
    ///     }
    /// }
    ///
    /// Base.unambiguous(); // prints "struct impl"
    /// Base.ambiguous(); // prints "trait impl"
    /// ```
    #[clippy::version = "1.74.0"]
    pub AMBIGUOUS_METHOD_CALLS,
    pedantic,
    "declarations and calls for same-named methods in struct impls and trait impls"
}

#[derive(Clone)]
pub struct AmbiguousMethodCalls<'tcx> {
    trait_methods: FxHashMap<(Ty<'tcx>, Symbol), Span>,
    inherent_methods: Vec<(Ty<'tcx>, Symbol, Span)>,
    call_sites: FxHashMap<(Ty<'tcx>, Symbol), Vec<Span>>,
}

impl<'tcx> AmbiguousMethodCalls<'tcx> {
    pub fn new() -> Self {
        Self {
            trait_methods: FxHashMap::default(),
            inherent_methods: Vec::default(),
            call_sites: FxHashMap::default(),
        }
    }

    fn insert_method(&mut self, is_trait_impl: bool, ty: Ty<'tcx>, ident: Ident) {
        if is_trait_impl {
            self.trait_methods.insert((ty, ident.name), ident.span);
        } else {
            self.inherent_methods.push((ty, ident.name, ident.span));
        }
    }

    fn insert_call_site(&mut self, ty: Ty<'tcx>, ident: Ident, span: Span) {
        if let Some(spans) = self.call_sites.get_mut(&(ty, ident.name)) {
            spans.push(span);
        } else {
            self.call_sites.insert((ty, ident.name), vec![span]);
        }
    }
}

impl_lint_pass!(AmbiguousMethodCalls<'_> => [AMBIGUOUS_METHOD_CALLS]);

impl<'tcx> LateLintPass<'tcx> for AmbiguousMethodCalls<'tcx> {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'_>,
        _: &'tcx Body<'_>,
        _: Span,
        def_id: LocalDefId,
    ) {
        if let FnKind::Method(ident, _) = kind {
            let hir_id = cx.tcx.hir().local_def_id_to_hir_id(def_id);
            let is_trait_impl = is_trait_impl_item(cx, hir_id);
            let parent_id = cx.tcx.hir().get_parent_item(hir_id);

            // Calling type_of on a method's default impl causes an ICE
            if let Some(hir::Node::Item(item)) = cx.tcx.hir().find(hir::HirId::from(parent_id)) {
                if let hir::ItemKind::Trait(..) = item.kind {
                    return;
                }
            }

            let parent_ty = cx.tcx.type_of(parent_id.to_def_id()).skip_binder();
            self.insert_method(is_trait_impl, parent_ty, ident);
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>) {
        if let hir::ExprKind::MethodCall(path, recv, _, call_span) = &expr.kind {
            let recv_ty = cx.typeck_results().expr_ty(recv).peel_refs();

            self.insert_call_site(recv_ty, path.ident, *call_span);
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        for (ty, name, span) in &self.inherent_methods {
            let k = &(*ty, *name);
            if self.trait_methods.contains_key(k) {
                span_lint(
                    cx,
                    AMBIGUOUS_METHOD_CALLS,
                    *self.trait_methods.get(k).unwrap(),
                    "ambiguous trait method name",
                );
                span_lint_and_help(
                    cx,
                    AMBIGUOUS_METHOD_CALLS,
                    *span,
                    "ambiguous struct method name",
                    None,
                    "consider renaming the struct impl's method",
                );

                if let Some(spans) = self.call_sites.get(k) {
                    for span in spans {
                        span_lint_and_help(
                            cx,
                            AMBIGUOUS_METHOD_CALLS,
                            *span,
                            "ambiguous method call",
                            None,
                            "consider renaming the struct impl's method or explicitly qualifying the call site",
                        );
                    }
                }
            }
        }
    }
}
