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
    restriction,
    "rest (..) in destructuring expression"
}
declare_lint_pass!(RestWhenDestructuringStruct => [REST_WHEN_DESTRUCTURING_STRUCT]);

impl<'tcx> LateLintPass<'tcx> for RestWhenDestructuringStruct {
    fn check_pat(&mut self, cx: &rustc_lint::LateContext<'tcx>, pat: &'tcx rustc_hir::Pat<'tcx>) {
        if let rustc_hir::PatKind::Struct(path, fields, Some(dotdot)) = pat.kind
            && !pat.span.in_external_macro(cx.tcx.sess.source_map())
            && !is_from_proc_macro(cx, pat)
            && let qty = cx.typeck_results().qpath_res(&path, pat.hir_id)
            && let ty = cx.typeck_results().pat_ty(pat)
            && let ty::Adt(a, _) = ty.kind()
        {
            let vid = qty
                .opt_def_id()
                .map_or(VariantIdx::ZERO, |x| a.variant_index_with_id(x));

            let leave_dotdot = a.variants()[vid]
                .fields
                .iter()
                .any(|f| !f.vis.is_accessible_from(cx.tcx.parent_module(pat.hir_id), cx.tcx));

            let mut rest_fields = a.variants()[vid]
                .fields
                .iter()
                .filter(|f| f.vis.is_accessible_from(cx.tcx.parent_module(pat.hir_id), cx.tcx))
                .filter(|pf| !fields.iter().any(|x| x.ident.name == pf.name))
                .map(|x| format!("{}: _", x.ident(cx.tcx)));

            let mut fmt_fields = rest_fields.join(", ");

            if fmt_fields.is_empty() && leave_dotdot {
                // The struct is non_exhaustive, from a non-local crate and all public fields are explicitly named.
                return;
            }

            if leave_dotdot {
                fmt_fields.push_str(", ..");
            }

            let message = if a.variants()[vid].fields.is_empty() {
                "consider remove rest pattern (`..`)"
            } else if fields.is_empty() {
                "consider explicitly ignoring fields with wildcard patterns (`x: _`)"
            } else {
                "consider explicitly ignoring remaining fields with wildcard patterns (`x: _`)"
            };

            span_lint_and_then(
                cx,
                REST_WHEN_DESTRUCTURING_STRUCT,
                pat.span,
                "struct destructuring with rest (`..`)",
                |diag| {
                    diag.span_suggestion_verbose(
                        dotdot,
                        message,
                        fmt_fields,
                        rustc_errors::Applicability::MachineApplicable,
                    );
                },
            );
        }
    }
}
