use clippy_utils::diagnostics::span_lint_and_then;
use rustc_hir::{Pat, PatKind, QPath};
use rustc_lint::LateContext;
use rustc_middle::ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unnecessary '..' pattern binding on struct when all fields are explicitly matched.
    ///
    /// ### Why restrict this?
    /// Correctness and readability. It's like having a wildcard pattern after
    /// matching all enum variants explicitly.
    ///
    /// ### Example
    /// ```no_run
    /// # struct A { a: i32 }
    /// let a = A { a: 5 };
    ///
    /// match a {
    ///     A { a: 5, .. } => {},
    ///     _ => {},
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # struct A { a: i32 }
    /// # let a = A { a: 5 };
    /// match a {
    ///     A { a: 5 } => {},
    ///     _ => {},
    /// }
    /// ```
    #[clippy::version = "1.43.0"]
    pub REST_PAT_IN_FULLY_BOUND_STRUCTS,
    restriction,
    "a match on a struct that binds all fields but still uses the wildcard pattern"
}

pub(crate) fn check(cx: &LateContext<'_>, pat: &Pat<'_>) {
    if !pat.span.from_expansion()
        && let PatKind::Struct(QPath::Resolved(_, path), fields, true) = pat.kind
        && let Some(def_id) = path.res.opt_def_id()
        && let ty = cx.tcx.type_of(def_id).instantiate_identity()
        && let ty::Adt(def, _) = ty.kind()
        && (def.is_struct() || def.is_union())
        && fields.len() == def.non_enum_variant().fields.len()
        && !def.non_enum_variant().is_field_list_non_exhaustive()
    {
        #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
        span_lint_and_then(
            cx,
            REST_PAT_IN_FULLY_BOUND_STRUCTS,
            pat.span,
            "unnecessary use of `..` pattern in struct binding. All fields were already bound",
            |diag| {
                diag.help("consider removing `..` from this binding");
            },
        );
    }
}
