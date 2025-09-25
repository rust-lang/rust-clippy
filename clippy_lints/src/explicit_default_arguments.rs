use std::borrow::Cow;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{GenericArg, QPath, TyKind};
use rustc_hir_analysis::lower_ty;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{EarlyBinder, GenericParamDefKind};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the usage of a generic argument when the type already defines a default.
    ///
    /// ### Why is this bad?
    /// It is redundant and adds visual clutter.
    ///
    /// ### Example
    /// ```no_run
    /// type Result<T = ()> = core::result::Result<T, MyError>;
    /// fn foo() -> Result<()> {
    ///     Ok(())
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// type Result<T = ()> = core::result::Result<T, MyError>;
    /// fn foo() -> Result {
    ///     Ok(())
    ///}
    /// ```
    #[clippy::version = "1.90.0"]
    pub EXPLICIT_DEFAULT_ARGUMENTS,
    style,
    "default lint description"
}

declare_lint_pass!(ExplicitDefaultArguments => [EXPLICIT_DEFAULT_ARGUMENTS]);

// TODO: Refactor and improve naming. Make it right.
// TODO: Will type inference be an issue?
impl<'tcx> LateLintPass<'tcx> for ExplicitDefaultArguments {
    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &'tcx rustc_hir::Ty<'_, rustc_hir::AmbigArg>) {
        // TODO: Double-check for ICEs and other issues
        if let TyKind::Path(qpath) = ty.kind
            && let QPath::Resolved(_, path) = qpath
            && let Some(last_path_segment) = path.segments.last()
            && let Some(generic_args) = last_path_segment.args
            && let Res::Def(DefKind::TyAlias, def_id) = cx.qpath_res(&qpath, ty.hir_id)
            && !generic_args.args.is_empty()
        {
            // FIXME: Use a simpler comparision instead of type lowering to avoid issues
            let default_generic_arg_types: Vec<_> = cx
                .tcx
                .generics_of(def_id)
                .own_params
                .iter()
                .map(|generic_param| {
                    if let GenericParamDefKind::Type { has_default, .. } = generic_param.kind
                        && has_default
                    {
                        Some(cx.tcx.type_of(generic_param.def_id))
                    } else {
                        None
                    }
                })
                .collect();
            let generic_arg_types: Vec<_> = generic_args
                .args
                .iter()
                .map(|arg| {
                    if let GenericArg::Type(ty) = arg {
                        Some((ty, lower_ty(cx.tcx, ty.as_unambig_ty())))
                    } else {
                        None
                    }
                })
                .collect();

            // This method works because generic parameters will always be trailing, as it is enforced by the
            // compiler and syntactically impossble either way.
            let mut first_default: Option<usize> = None;
            let mut defaults: Vec<Cow<'_, str>> = vec![];
            for (i, (actual, default)) in generic_arg_types.iter().zip(default_generic_arg_types).enumerate() {
                let Some(actual) = actual else {
                    continue;
                };
                let Some(default) = default else {
                    continue;
                };
                if EarlyBinder::bind(actual.1) != default {
                    continue;
                }
                first_default.get_or_insert(i);

                defaults.push(snippet(cx, actual.0.span, "<default>"));
            }
            let path_str = {
                let mut string = String::new();
                for (i, segment) in path.segments.iter().enumerate() {
                    string.push_str(&segment.ident.to_string());
                    if i < path.segments.len() - 1 {
                        string.push_str("::");
                    }
                }
                string
            };
            let sugg = if let Some(first_default) = first_default
                && first_default > 0
            {
                let mut string = path_str;
                let mut iter = generic_arg_types.iter().enumerate();
                string.push('<');
                while let Some((i, Some((ty, _)))) = iter.next() {
                    if i >= first_default {
                        break;
                    }
                    string.push_str(&snippet(cx, ty.span, "<default>"));
                    if i + 1 < first_default {
                        string.push_str(", ");
                    }
                }
                string.push('>');
                string
            } else {
                if snippet(cx, ty.span, "<error>") == "outer::NestedResult<usize>" {
                    for segment in path.segments {
                        println!("seg: {:?}", segment.ident);
                    }
                }
                path_str
            };
            if defaults.is_empty() {
                return;
            }
            // TODO: Use constants for strings when possible
            let msg = if defaults.len() == 1 {
                format!("unnecessary generics, `{}` already is the default", defaults[0])
            } else {
                let mut defaults_str = String::new();
                for (i, default) in defaults.iter().enumerate() {
                    defaults_str.push('`');
                    defaults_str.push_str(default);
                    defaults_str.push('`');

                    if i < defaults.len() - 1 {
                        defaults_str.push_str(", ");
                    }
                }
                format!("unnecessary generics, [{defaults_str}] already are already the defaults",)
            };
            println!("ty span: {}", snippet(cx, ty.span, "<error>"));
            span_lint_and_sugg(
                cx,
                EXPLICIT_DEFAULT_ARGUMENTS,
                ty.span,
                "use",
                msg,
                sugg,
                rustc_errors::Applicability::MachineApplicable,
            );
        }
    }
}
