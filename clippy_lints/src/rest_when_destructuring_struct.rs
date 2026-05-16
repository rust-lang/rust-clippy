use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_from_proc_macro;
use itertools::Itertools;
use rustc_abi::VariantIdx;
use rustc_lint::LateLintPass;
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Disallow the use of rest patterns for accesible fields
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
    #[clippy::version = "1.94.0"]
    pub REST_PATTERN_ACCESSIBLE_FIELD,
    restriction,
    "rest (..) used for accessible field"
}

declare_clippy_lint! {
    /// ### What it does
    /// Disallow the use of rest patterns that are unnecessary.
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
    /// let S { a, b, c, .. } = s;
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
    /// let S { a, b, c } = s;
    /// ```
    #[clippy::version = "1.94.0"]
    pub UNNECESSARY_REST_PATTERN,
    restriction,
    "unnecessary rest (..) in destructuring expression"
}

declare_lint_pass!(RestPatternAccessibleField => [REST_PATTERN_ACCESSIBLE_FIELD]);

declare_lint_pass!(UnnecessaryRestPattern => [UNNECESSARY_REST_PATTERN]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryRestPattern {
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

            if leave_dotdot {
                // There are inaccesible fields so the dotdot is not unnecessary.
                return;
            }

            let unnamed_field_count = a.variants()[vid]
                .fields
                .iter()
                .filter(|f| f.vis.is_accessible_from(cx.tcx.parent_module(pat.hir_id), cx.tcx))
                .filter(|pf| !fields.iter().any(|x| x.ident.name == pf.name))
                .count();

            if unnamed_field_count > 0 {
                // There are unnamed fields so dotdot is not unnecessary.
                return;
            }

            let message = "consider removing the unnecessary rest pattern (`..`)";

            span_lint_and_then(
                cx,
                UNNECESSARY_REST_PATTERN,
                pat.span,
                "unnecessary rest pattern (`..`)",
                |diag| {
                    diag.span_suggestion_verbose(dotdot, message, "", rustc_errors::Applicability::MachineApplicable);
                },
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for RestPatternAccessibleField {
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

            if a.variants()[vid].fields.is_empty() {
                // Its a empty struct so by definition there cannot be a rest pattern that "accesses" a field.
                //
                // This can be caught by the sister lint `unnecessary_rest_pattern`.
                return;
            }

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

            if fmt_fields.is_empty() {
                // The struct either has no unnamed member or only has unnameable members left.
                return;
            }

            if leave_dotdot {
                fmt_fields.push_str(", ..");
            }

            let message = if a.variants()[vid].fields.is_empty() {
                "consider removing the rest pattern (`..`)"
            } else if fields.is_empty() {
                "consider explicitly ignoring fields with wildcard patterns (`x: _`)"
            } else {
                "consider explicitly ignoring remaining fields with wildcard patterns (`x: _`)"
            };

            span_lint_and_then(
                cx,
                REST_PATTERN_ACCESSIBLE_FIELD,
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
