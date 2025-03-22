use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_trait_item;
use clippy_utils::source::snippet;
use rustc_hir::{ExprKind, StructTailExpr};
use rustc_lint::{LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Check struct initialization uses `..*::default()` pattern to skip rest of struct field initialization.
    ///
    /// ### Why restrict this?
    /// Using `..*::default()` can hide field initialization when new fields are added to structs,
    /// potentially leading to bugs where developers forget to explicitly set values for new fields.
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
            && let ExprKind::Struct(_, _, StructTailExpr::Base(base)) = &expr.kind
            && let ExprKind::Call(func, _) = base.kind
            && is_trait_item(cx, func, sym::Default)
        {
            span_lint_and_help(
                cx,
                STRUCT_FIELDS_REST_DEFAULT,
                base.span,
                format!(
                    "should not use `..{}` to omit rest of struct field initialization",
                    snippet(cx, base.span, "")
                ),
                Some(expr.span),
                "explicitly specify all fields or use other base value instead of `..*::default()`",
            );
        }
    }
}
