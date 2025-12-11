use clippy_config::Conf;
use clippy_config::types::{DisallowedPath, create_disallowed_map};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::paths::PathNS;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefIdMap;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Adt, TyCtxt};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Denies the configured fields in clippy.toml
    ///
    /// Note: Even though this lint is warn-by-default, it will only trigger if
    /// fields are defined in the clippy.toml file.
    ///
    /// ### Why is this bad?
    /// Some fields are undesirable in certain contexts, and it's beneficial to
    /// lint for them as needed.
    ///
    /// ### Example
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// disallowed-fields = [
    ///     # Can use a string as the path of the disallowed field.
    ///     "std::ops::Range::start",
    ///     # Can also use an inline table with a `path` key.
    ///     { path = "std::ops::Range::start" },
    ///     # When using an inline table, can add a `reason` for why the field
    ///     # is disallowed.
    ///     { path = "std::ops::Range::start", reason = "The start of the range is not used" },
    /// ]
    /// ```
    ///
    /// ```rust
    /// use std::ops::Range;
    ///
    /// let range = Range { start: 0, end: 1 };
    /// println!("{}", range.start); // `start` is disallowed in the config.
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// use std::ops::Range;
    ///
    /// let range = Range { start: 0, end: 1 };
    /// println!("{}", range.end); // `end` is _not_ disallowed in the config.
    /// ```
    #[clippy::version = "1.93.0"]
    pub DISALLOWED_FIELDS,
    style,
    "declaration of a disallowed field use"
}

pub struct DisallowedFields {
    disallowed: DefIdMap<(&'static str, &'static DisallowedPath)>,
}

impl DisallowedFields {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf) -> Self {
        let (disallowed, _) = create_disallowed_map(
            tcx,
            &conf.disallowed_fields,
            PathNS::Value,
            |def_kind| matches!(def_kind, DefKind::Field),
            "field",
            false,
        );
        Self { disallowed }
    }
}

impl_lint_pass!(DisallowedFields => [DISALLOWED_FIELDS]);

impl<'tcx> LateLintPass<'tcx> for DisallowedFields {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let (id, span) = match &expr.kind {
            ExprKind::Path(path) if let Res::Def(_, id) = cx.qpath_res(path, expr.hir_id) => (id, expr.span),
            ExprKind::Field(e, ident) => {
                // Very round-about way to get the field `DefId` from the expr: first we get its
                // parent `Ty`. Then we go through all its fields to find the one with the expected
                // name and get the `DefId` from it.
                if let Some(parent_ty) = cx.typeck_results().expr_ty_opt(e)
                    && let Adt(adt_def, ..) = parent_ty.kind()
                    && let Some(field_def_id) = adt_def.all_fields().find_map(|field| {
                        if field.name == ident.name {
                            Some(field.did)
                        } else {
                            None
                        }
                    })
                {
                    (field_def_id, ident.span)
                } else {
                    return;
                }
            },
            _ => return,
        };
        if let Some(&(path, disallowed_path)) = self.disallowed.get(&id) {
            span_lint_and_then(
                cx,
                DISALLOWED_FIELDS,
                span,
                format!("use of a disallowed field `{path}`"),
                disallowed_path.diag_amendment(span),
            );
        }
    }
}
