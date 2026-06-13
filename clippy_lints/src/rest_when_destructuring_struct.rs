use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_from_proc_macro;
use itertools::Itertools;
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

declare_lint_pass!(RestWhenDestructuringStruct => [
    REST_PATTERN_ACCESSIBLE_FIELD,
    UNNECESSARY_REST_PATTERN,
]);

impl<'tcx> LateLintPass<'tcx> for RestWhenDestructuringStruct {
    fn check_pat(&mut self, cx: &rustc_lint::LateContext<'tcx>, pat: &'tcx rustc_hir::Pat<'tcx>) {
        if let rustc_hir::PatKind::Struct(path, fields, Some(dotdot)) = pat.kind
            && let qty = cx.typeck_results().qpath_res(&path, pat.hir_id)
            && let ty = cx.typeck_results().pat_ty(pat)
            && let ty::Adt(a, _) = ty.kind()
            && let Some(vid) = qty.opt_def_id().map(|x| a.variant_index_with_id(x))
            && let Some(variant) = a.variants().get(vid)
            && !pat.span.in_external_macro(cx.tcx.sess.source_map())
            && !is_from_proc_macro(cx, pat)
        {
            let mut rest_pattern_accessible_field = true;
            let mut unnecessary_rest_pattern = true;

            if variant.fields.is_empty() {
                // Its a empty struct so by definition there cannot be a rest pattern that "accesses" a field.
                //
                // This can be caught by the sister lint `unnecessary_rest_pattern`.
                rest_pattern_accessible_field = false;
            }

            let rest_fields_iter = variant
                .fields
                .iter()
                .filter(|f| f.vis.is_accessible_from(cx.tcx.parent_module(pat.hir_id), cx.tcx))
                .filter(|pf| !fields.iter().any(|x| x.ident.name == pf.name));

            let unnamed_field_count = rest_fields_iter.clone().count();
            if unnamed_field_count > 0 {
                // There are unnamed fields so dotdot is not unnecessary.
                unnecessary_rest_pattern = false;
                if !rest_pattern_accessible_field {
                    return;
                }
            }

            let leave_dotdot = variant
                .fields
                .iter()
                .any(|f| !f.vis.is_accessible_from(cx.tcx.parent_module(pat.hir_id), cx.tcx));

            if leave_dotdot {
                // The dotdot is needed
                unnecessary_rest_pattern = false;
                if !rest_pattern_accessible_field {
                    return;
                }
            }

            if unnecessary_rest_pattern {
                let message = "consider removing the unnecessary rest pattern (`..`)";

                span_lint_and_then(
                    cx,
                    UNNECESSARY_REST_PATTERN,
                    pat.span,
                    "unnecessary rest pattern (`..`)",
                    |diag| {
                        diag.span_suggestion_verbose(
                            dotdot,
                            message,
                            "",
                            rustc_errors::Applicability::MachineApplicable,
                        );
                    },
                );
            }

            if rest_pattern_accessible_field {
                let module = cx.tcx.parent_module(pat.hir_id);
                let f = |f: &&ty::FieldDef| {
                    (fields.iter().all(|x| x.ident.name != f.name)) && f.vis.is_accessible_from(module, cx.tcx)
                };

                let Some(first) = variant.fields.iter().position(|x| f(&x)) else {
                    return;
                };

                let fmt_fields = variant
                    .fields
                    .iter()
                    .skip(first)
                    .filter(f)
                    .format_with(", ", |f, write| write(&format_args!("{}: _", f.name.as_str())));

                let fmt_fields = if leave_dotdot {
                    format!("{fmt_fields}, ..")
                } else {
                    fmt_fields.to_string()
                };

                let message = if fields.is_empty() {
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
}
