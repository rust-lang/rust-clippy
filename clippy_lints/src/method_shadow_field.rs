use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{ImplItem, ImplItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

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

declare_lint_pass!(MethodShadowField => [
    METHOD_SHADOW_FIELD,
    METHOD_SHADOW_PUBLIC_FIELD,
]);

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

        let method_name = &impl_item.ident.name;

        // get the ty of the parent
        let impl_def_id = cx.tcx.local_parent(impl_item.owner_id.def_id);
        let ty = cx.tcx.type_of(impl_def_id).instantiate_identity();

        // we filter out enums
        if let ty::Adt(adt_def, _) = ty.kind()
            && (adt_def.is_struct() || adt_def.is_union())
        {
            let variant = adt_def.non_enum_variant();
            // query fields and checks name collision, public/private
            for field in &variant.fields {
                if field.name != *method_name {
                    continue;
                }

                if cx.tcx.visibility(field.did).is_public() {
                    span_lint_and_help(
                        cx,
                        METHOD_SHADOW_PUBLIC_FIELD,
                        impl_item.ident.span,
                        "method shadows a public field",
                        cx.tcx.def_ident_span(field.did),
                        "consider making the field private or use a distinct name",
                    );
                } else {
                    span_lint_and_help(
                        cx,
                        METHOD_SHADOW_FIELD,
                        impl_item.ident.span,
                        "method shadows a field",
                        cx.tcx.def_ident_span(field.did),
                        "consider renaming the method or field",
                    );
                }
                return;
            }
        }
    }
}
