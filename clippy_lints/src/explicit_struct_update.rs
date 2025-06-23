use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{snippet, snippet_indent};
use rustc_errors::Applicability;
use rustc_hir::{self as hir, ExprKind, StructTailExpr};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for struct initializations where a field is being set to the value of the same field from another struct of the same type.
    /// ### Why is this bad?
    /// This can be done more concisely using struct update syntax.
    /// ### Example
    /// ```no_run
    /// struct Foo {
    ///     a: i32,
    ///     b: i32,
    ///     c: i32,
    /// }
    ///
    /// let my_foo = Foo {
    ///     a: 1,
    ///     b: 2,
    ///     c: 3,
    /// };
    ///
    /// let my_new_foo = Foo {
    ///     a: 5,
    ///     b: my_foo.b,
    ///     c: my_foo.c,
    /// };
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct Foo {
    ///     a: i32,
    ///     b: i32,
    ///     c: i32,
    /// }
    ///
    /// let my_foo = Foo {
    ///     a: 1,
    ///     b: 2,
    ///     c: 3,
    /// };
    ///
    /// let my_new_foo = Foo {
    ///     a: 5,
    ///     ..my_foo
    /// };
    /// ```
    #[clippy::version = "1.89.0"]
    pub EXPLICIT_STRUCT_UPDATE,
    complexity,
    "explicit struct updates in struct instantiations"
}
declare_lint_pass!(ExplicitStructUpdate => [EXPLICIT_STRUCT_UPDATE]);

impl<'tcx> LateLintPass<'tcx> for ExplicitStructUpdate {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>) {
        let (path, fields) = match expr.kind {
            ExprKind::Struct(path, fields, tail) => {
                match tail {
                    StructTailExpr::None => (path, fields),
                    _ => {
                        // if there is a tail expression, we don't want to lint
                        return;
                    },
                }
            },
            _ => return,
        };

        // the type of the struct
        let ty = cx.typeck_results().expr_ty(expr);

        // collect the fields that are being initialized with the same field from another struct of the same
        // type
        let update_fields: Option<Vec<(&rustc_hir::Expr<'_>, &rustc_hir::Expr<'_>)>> =
            fields.iter().fold(Some(Vec::new()), |mut acc, f| {
                let v = match acc {
                    Some(ref mut v) => v,
                    None => return None,
                };

                if let ExprKind::Field(base_expr, field_ident) = f.expr.kind {
                    if let Some(last) = v.last() {
                        match (last.1.kind, base_expr.kind) {
                            (
                                ExprKind::Path(hir::QPath::Resolved(_, hir::Path { res: res_a, .. })),
                                ExprKind::Path(hir::QPath::Resolved(_, hir::Path { res: res_b, .. })),
                            ) if res_a != res_b => return None, /* if we detect instantiation from multiple bases, we */
                            // don't want to lint
                            _ => (),
                        }
                    }

                    if cx.typeck_results().expr_ty(base_expr) == ty && f.ident == field_ident {
                        // accumulate the expressions mapping to the actual field expression, and the expression of the
                        // base struct, we do this so we can determine if the base struct is the same for all
                        v.push((f.expr, base_expr));
                    }
                }

                acc
            });

        let (update_base, update_fields): (_, Vec<_>) = match update_fields {
            // we only care about the field expressions at this point
            Some(fields) if !fields.is_empty() => (fields[0].1, fields.iter().map(|x| x.0.hir_id).collect()),
            // no lint if there's no fields or multiple bases
            _ => return,
        };

        // the field assignments we are keeping
        let non_update_fields_spans: Vec<_> = fields
            .iter()
            .filter_map(|f| {
                if !update_fields.contains(&f.expr.hir_id) {
                    Some(f.span)
                } else {
                    None
                }
            })
            .collect();

        let struct_indent = snippet_indent(cx, expr.span).unwrap_or_default();
        let field_indent = format!("{struct_indent}    ");
        let struct_type = snippet(cx, path.span(), "");
        let struct_fields = non_update_fields_spans
            .iter()
            .fold(String::new(), |mut acc, &field_span| {
                acc.push_str(&field_indent);
                acc.push_str(&snippet(cx, field_span, ""));
                acc.push_str(",\n");
                acc
            });
        let struct_update_snip = snippet(cx, update_base.span, "");

        let sugg = format!("{struct_type} {{\n{struct_fields}{field_indent}..{struct_update_snip}\n{struct_indent}}}");

        let msg = "you seem to be updating a struct field with the same field from another struct of the same type";

        span_lint_and_then(cx, EXPLICIT_STRUCT_UPDATE, expr.span, msg, |diag| {
            diag.span_suggestion(
                expr.span,
                "consider using struct update syntax instead",
                sugg,
                Applicability::MachineApplicable,
            );
        });
    }
}
