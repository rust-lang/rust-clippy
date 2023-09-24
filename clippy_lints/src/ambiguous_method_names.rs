use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::is_trait_impl_item;
use clippy_utils::ty::implements_trait;
use hir::{ImplItem, Item};
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::def_id::LocalDefId;
use rustc_span::symbol::Ident;

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
pub struct AmbiguousMethodNames {
    // Keeps track of trait methods
    trait_methods: Vec<(LocalDefId, Ident)>,
    // Keeps track of inherent methods
    inherent_methods: Vec<(LocalDefId, Ident)>,
}

impl AmbiguousMethodNames {
    pub fn new() -> Self {
        Self {
            trait_methods: Vec::default(),
            inherent_methods: Vec::default(),
        }
    }
}

impl_lint_pass!(AmbiguousMethodNames => [AMBIGUOUS_METHOD_NAMES]);

impl<'tcx> LateLintPass<'tcx> for AmbiguousMethodNames {
    // Check trait impls
    fn check_item(&mut self, _: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        if let hir::ItemKind::Trait(_, _, _, _, tr_items) = item.kind {
            for tr_item in tr_items {
                if let hir::AssocItemKind::Fn { .. } = tr_item.kind {
                    self.trait_methods.push((item.owner_id.def_id, tr_item.ident))
                }
            }
        }
    }

    // Check inherent methods
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'_>) {
        if let hir::ImplItemKind::Fn(..) = impl_item.kind {
            let hir_id = cx.tcx.hir().local_def_id_to_hir_id(impl_item.owner_id.def_id);
            if !is_trait_impl_item(cx, hir_id) {
                let struct_id = cx.tcx.hir().get_parent_item(hir_id);
                self.inherent_methods.push((struct_id.def_id, impl_item.ident))
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        for (r#trait, ident) in &self.trait_methods {
            for (r#struct, inherent_ident) in &self.inherent_methods {
                let struct_ty = cx.tcx.type_of(r#struct.to_def_id()).skip_binder();
                if implements_trait(cx, struct_ty, r#trait.to_def_id(), &[]) && ident.name == inherent_ident.name {
                    span_lint_and_note(
                        cx,
                        AMBIGUOUS_METHOD_NAMES,
                        inherent_ident.span,
                        "ambiguous inherent method name",
                        Some(ident.span),
                        "trait method defined here",
                    );
                }
            }
        }
    }
}
