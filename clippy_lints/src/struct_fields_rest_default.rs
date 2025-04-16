use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_trait_item;
use clippy_utils::source::snippet;

use rustc_hir::{ExprKind, QPath, StructTailExpr};
use rustc_lint::{LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;
use rustc_span::{Symbol, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Check if struct initialization uses derive `Default` with `..*::default()` pattern
    /// to skip rest of struct field initialization.
    ///
    /// ### Why restrict this?
    /// Using `..*::default()` can hide field initialization when new fields are added to structs,
    /// potentially leading to bugs where developers forget to explicitly set values for new fields.
    ///
    /// ### Limitations
    /// Only check the derive `Default` with `..*::default()` pattern,
    /// because when developer manually implements `Default` or uses other base value,
    /// it means that they know what they are doing rather than just being lazy.
    ///
    /// ### Example
    /// ```no_run
    /// #[derive(Default)]
    /// struct Foo {
    ///     a: i32,
    ///     b: i32,
    ///     // when add new filed `c`
    ///     c: i32,
    /// }
    ///
    /// let _ = Foo {
    ///     a: Default::default(),
    ///     ..Default::default()
    ///     // developer may forget to explicitly set field `c` and cause bug
    /// };
    /// ```
    /// Use instead:
    /// ```no_run
    /// #[derive(Default)]
    /// struct Foo {
    ///     a: i32,
    ///     b: i32,
    ///     // when add new filed `c`
    ///     c: i32,
    /// }
    ///
    /// // make the compiler check for new fields to avoid bug.
    /// let _ = Foo {
    ///     a: Default::default(),
    ///     b: Default::default(),
    ///     c: Default::default(),
    /// };
    ///
    /// impl Foo {
    ///     fn get_foo() -> Self {
    ///         Foo{ a: 0, b: 0, c: 0}
    ///     }
    /// }
    ///
    /// // or avoid using `..*::default()`
    /// let _ = Foo {
    ///     a: Default::default(),
    ///     ..Foo::get_foo()
    /// };
    /// ```
    #[clippy::version = "1.87.0"]
    pub STRUCT_FIELDS_REST_DEFAULT,
    restriction,
    "should not use `..*::default()` pattern to omit rest of struct field initialization"
}

declare_lint_pass!(StructFieldsDefault => [STRUCT_FIELDS_REST_DEFAULT]);

impl<'tcx> LateLintPass<'tcx> for StructFieldsDefault {
    fn check_expr(&mut self, cx: &rustc_lint::LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if !expr.span.in_external_macro(cx.sess().source_map())
            && let ExprKind::Struct(QPath::Resolved(_, _), _, StructTailExpr::Base(base)) = &expr.kind
            && let ExprKind::Call(func, _) = base.kind
            && is_trait_item(cx, func, sym::Default)
            && is_struct_trait_from_derive(cx, expr, sym::Default)
        {
            span_lint_and_help(
                cx,
                STRUCT_FIELDS_REST_DEFAULT,
                base.span,
                format!(
                    "usage of `..{}` to initialize struct fields",
                    snippet(cx, base.span, "")
                ),
                Some(expr.span),
                "explicitly specify all fields or use other base value instead of `..*::default()`",
            );
        }
    }
}

fn is_struct_trait_from_derive<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
    trait_item: Symbol,
) -> bool {
    if let Some(default_trait_id) = cx.tcx.get_diagnostic_item(trait_item) {
        let impls = cx.tcx.trait_impls_of(default_trait_id);
        for (impl_ty, impl_def_ids) in impls.non_blanket_impls() {
            if let Some(impl_struct_def_id) = impl_ty.def()
                && let ty::Adt(curr_struct, _) = cx.typeck_results().expr_ty(expr).kind()
                && curr_struct.did() == impl_struct_def_id
            {
                // we found the struct what we need, skip the rest.
                return impl_def_ids
                    .iter()
                    .any(|&impl_def_id| cx.tcx.is_automatically_derived(impl_def_id));
            }
        }
    }
    false
}
