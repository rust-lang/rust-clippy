use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::Expr;
use rustc_ast::ast::{ExprKind, StructRest};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Check struct initialization uses `..Default::default()` pattern to skip rest of struct field initialization.
    ///
    /// ### Why restrict this?
    /// Using `..Default::default()`` can hide field initialization when new fields are added to structs,
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
    /// ```
    #[clippy::version = "1.87.0"]
    pub STRUCT_FIELDS_REST_DEFAULT,
    restriction,
    "should not use `..Default::default()` to omit rest of struct field initialization"
}

declare_lint_pass!(StructFieldsDefault => [STRUCT_FIELDS_REST_DEFAULT]);

impl EarlyLintPass for StructFieldsDefault {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if let ExprKind::Struct(struct_expr) = &expr.kind
            && let StructRest::Base(base) = &struct_expr.rest
            && !base.span.in_external_macro(cx.sess().source_map())
            && let ExprKind::Call(call, _) = &base.kind
            && let ExprKind::Path(_, path) = &call.kind
            && let [part1, part2] = path.segments.as_slice()
            && part1.ident.name == rustc_span::sym::Default
            && part2.ident.name == rustc_span::kw::Default
        {
            span_lint_and_help(
                cx,
                STRUCT_FIELDS_REST_DEFAULT,
                base.span,
                "should not use `..Default::default()` to omit rest of struct field initialization",
                Some(expr.span),
                "each field's initial value should be explicitly specified",
            );
        }
    }
}
