use clippy_utils::diagnostics::span_lint_and_note;
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
    pub AMBIGUOUS_METHOD_NAMES,
    pedantic,
    "declarations and calls for same-named methods in struct impls and trait impls"
}

#[derive(Clone)]
pub struct AmbiguousMethodNames<'tcx> {
    trait_methods: FxHashMap<(Ty<'tcx>, Symbol), Span>,
    inherent_methods: Vec<(Ty<'tcx>, Symbol, Span)>,
}

impl<'tcx> AmbiguousMethodNames<'tcx> {
    pub fn new() -> Self {
        Self {
            trait_methods: FxHashMap::default(),
            inherent_methods: Vec::default(),
        }
    }

    fn insert_method(&mut self, is_trait_impl: bool, ty: Ty<'tcx>, ident: Ident) {
        if is_trait_impl {
            self.trait_methods.insert((ty, ident.name), ident.span);
        } else {
            self.inherent_methods.push((ty, ident.name, ident.span));
        }
    }
}

impl_lint_pass!(AmbiguousMethodNames<'_> => [AMBIGUOUS_METHOD_NAMES]);

impl<'tcx> LateLintPass<'tcx> for AmbiguousMethodNames<'tcx> {
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
            if let Some(hir::Node::Item(item)) = cx.tcx.hir().find(hir::HirId::from(parent_id))
                && let hir::ItemKind::Trait(..) = item.kind
            {
                return;
            }

            let parent_ty = cx.tcx.type_of(parent_id.to_def_id()).skip_binder();
            self.insert_method(is_trait_impl, parent_ty, ident);
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        for (ty, name, span) in &self.inherent_methods {
            let k = &(*ty, *name);
            if let Some(tm_span) = self.trait_methods.get(k) {
                span_lint_and_note(
                    cx,
                    AMBIGUOUS_METHOD_NAMES,
                    *span,
                    "ambiguous inherent method name",
                    Some(*tm_span),
                    "trait method defined here",
                );
            }
        }
    }
}
