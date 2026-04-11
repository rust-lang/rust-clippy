use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::DefId;
use rustc_hir::{ImplItem, ImplItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;
use rustc_span::symbol::Symbol;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for struct/union methods that shadow private struct/union fields
    ///
    /// ### Why is this bad?
    /// Having a method and a field with the same name can cause confusion
    ///
    /// ### Example
    /// ```no_run
    /// struct Foo {
    ///   bar: i32
    /// }
    ///
    /// impl Foo {
    ///   fn bar(self) {}
    /// }
    /// ```
    ///
    /// Instead use distinct identifiers:
    /// ```no_run
    /// struct Foo {
    ///   bar: i32,
    /// }
    ///
    /// impl Foo {
    ///   fn other(self) {}
    /// }
    /// ```
    #[clippy::version = "1.96.0"]
    pub METHOD_SHADOW_FIELD,
    restriction,
    "method shadows the name of a field"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for public struct/union methods that shadow public struct/union fields
    ///
    /// ### Why is this bad?
    /// Having a method and a field with the same name can cause confusion
    ///
    /// ### Example
    /// ```no_run
    /// struct Foo {
    ///   pub bar: i32
    /// }
    ///
    /// impl Foo {
    ///   fn bar(self) {}
    /// }
    /// ```
    ///
    /// Instead make the field private or use distinct identifiers:
    /// ```no_run
    /// struct Foo {
    ///   bar: i32,
    /// }
    ///
    /// impl Foo {
    ///   fn bar(self) {}
    /// }
    /// ```
    #[clippy::version = "1.96.0"]
    pub METHOD_SHADOW_PUBLIC_FIELD,
    style,
    "method shadows the name of a public field"
}

impl_lint_pass!(MethodShadowField => [
    METHOD_SHADOW_FIELD,
    METHOD_SHADOW_PUBLIC_FIELD,
]);

pub struct MethodShadowField {
    avoid_breaking_exported_api: bool,
    field_cache: FxHashMap<DefId, FxHashMap<Symbol, (bool, DefId)>>,
}

impl MethodShadowField {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            avoid_breaking_exported_api: conf.avoid_breaking_exported_api,
            field_cache: FxHashMap::default(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for MethodShadowField {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'tcx>) {
        if impl_item.span.from_expansion() {
            // it's from a macro
            return;
        }

        // queries function signature
        let ImplItemKind::Fn(ref signature, _) = impl_item.kind else {
            // not a function
            return;
        };

        if !signature.decl.implicit_self.has_implicit_self() {
            // not a method
            return;
        }

        if self.avoid_breaking_exported_api && cx.effective_visibilities.is_exported(impl_item.owner_id.def_id) {
            // the api is exported
            return;
        }

        // get the ty of the parent
        let impl_def_id = cx.tcx.local_parent(impl_item.owner_id.def_id);
        let ty = cx.tcx.type_of(impl_def_id).instantiate_identity();

        if cx.tcx.impl_opt_trait_ref(impl_def_id).is_some() {
            // it's a trait impl
            return;
        }

        // we filter out enums
        if let ty::Adt(adt_def, _) = ty.kind()
            && (adt_def.is_struct() || adt_def.is_union())
        {
            let adt_did = adt_def.did();

            // per impl caching for fast lookup
            let field_map = self.field_cache.entry(adt_did).or_insert_with(|| {
                let mut map = FxHashMap::default();
                let variant = adt_def.non_enum_variant();
                for field in &variant.fields {
                    let is_pub = cx.tcx.visibility(field.did).is_public();
                    map.insert(field.name, (is_pub, field.did));
                }
                map
            });

            let method_name = impl_item.ident.name;

            if let Some(&(is_public, field_did)) = field_map.get(&method_name) {
                if is_public {
                    span_lint_and_help(
                        cx,
                        METHOD_SHADOW_PUBLIC_FIELD,
                        impl_item.ident.span,
                        "method shadows a public field",
                        cx.tcx.def_ident_span(field_did),
                        "consider making the field private or use a distinct name",
                    );
                } else {
                    span_lint_and_help(
                        cx,
                        METHOD_SHADOW_FIELD,
                        impl_item.ident.span,
                        "method shadows a field",
                        cx.tcx.def_ident_span(field_did),
                        "consider renaming the method or field",
                    );
                }
            }
        }
    }
}
