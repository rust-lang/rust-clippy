use crate::rustc_lint::LintContext as _;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_from_proc_macro;
use itertools::Itertools;
use rustc_abi::VariantIdx;
use rustc_lint::LateLintPass;
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Disallows the use of rest patterns when destructuring structs.
    ///
    /// ### Why is this bad?
    /// It might lead to unhandled fields when the struct changes.
    ///
    /// ### Example
    /// ```no_run
    /// struct S {
    ///     a: u8,
    ///     b: u8,
    ///     c: u8,
    /// }
    ///
    /// let s = S { a: 1, b: 2, c: 3 };
    ///
    /// let S { a, b, .. } = s;
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct S {
    ///     a: u8,
    ///     b: u8,
    ///     c: u8,
    /// }
    ///
    /// let s = S { a: 1, b: 2, c: 3 };
    ///
    /// let S { a, b, c: _ } = s;
    /// ```
    #[clippy::version = "1.89.0"]
    pub REST_WHEN_DESTRUCTURING_STRUCT,
    nursery,
    "rest (..) in destructuring expression"
}
declare_lint_pass!(RestWhenDestructuringStruct => [REST_WHEN_DESTRUCTURING_STRUCT]);

impl<'tcx> LateLintPass<'tcx> for RestWhenDestructuringStruct {
    fn check_pat(&mut self, cx: &rustc_lint::LateContext<'tcx>, pat: &'tcx rustc_hir::Pat<'tcx>) {
        if pat.span.in_external_macro(cx.tcx.sess.source_map()) {
            return;
        }

        if is_from_proc_macro(cx, pat) {
            return;
        }

        if let rustc_hir::PatKind::Struct(path, fields, true) = pat.kind {
            let qty = cx.typeck_results().qpath_res(&path, pat.hir_id);
            let ty = cx.typeck_results().pat_ty(pat);
            if let ty::Adt(a, _) = ty.kind() {
                let vid = qty
                    .opt_def_id()
                    .map_or(VariantIdx::ZERO, |x| a.variant_index_with_id(x));
                let mut rest_fields = a.variants()[vid]
                    .fields
                    .iter()
                    .map(|field| field.ident(cx.tcx))
                    .filter(|pf| !fields.iter().any(|x| x.ident == *pf))
                    .map(|x| format!("{x}: _"));
                let fmt_fields = rest_fields.join(", ");

                let sm = cx.sess().source_map();

                // It is not possible to get the span of the et cetera symbol at HIR level
                // so we have to get it in a bit of a roundabout way:

                // Find the end of the last field if any.
                let last_field = fields.iter().last().map(|x| x.span.shrink_to_hi());
                // If no last field take the whole pattern.
                let last_field = last_field.unwrap_or(pat.span.shrink_to_lo());
                // Create a new span starting and ending just before the first .
                let before_dot = sm.span_extend_to_next_char(last_field, '.', true).shrink_to_hi();
                // Extend the span to the end of the line
                let rest_of_line = sm.span_extend_to_next_char(before_dot, '\n', true);
                // Shrink the span so it only contains dots
                let dotdot = sm.span_take_while(rest_of_line, |x| *x == '.');

                span_lint_and_then(
                    cx,
                    REST_WHEN_DESTRUCTURING_STRUCT,
                    pat.span,
                    "struct destructuring with rest (..)",
                    |diag| {
                        diag.span_suggestion_verbose(
                            dotdot,
                            "consider explicitly ignoring remaining fields with wildcard patterns (x: _)",
                            fmt_fields,
                            rustc_errors::Applicability::MachineApplicable,
                        );
                    },
                );
            }
        }
    }
}
