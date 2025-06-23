use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::path_res;
use clippy_utils::source::snippet;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, List, Ty};
use rustc_session::declare_lint_pass;
use rustc_span::sym;
use std::iter;

declare_clippy_lint! {
    /// ### What it does
    /// Detects expressions being enclosed in `Path::new` when passed to a function that accepts
    /// `impl AsRef<Path>`, when the enclosed expression could be used.
    ///
    /// ### Why is this bad?
    /// It is unnecessarily verbose
    ///
    /// ### Example
    /// ```no_run
    /// # use std::{fs, path::Path};
    /// fs::write(Path::new("foo.txt"), "foo");
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::{fs, path::Path};
    /// fs::write("foo.txt", "foo");
    /// ```
    #[clippy::version = "1.89.0"]
    pub NEEDLESS_PATH_NEW,
    nursery,
    "an argument passed to a function  that accepts `impl AsRef<Path>` \
    being enclosed in `Path::new` when the argument implements the trait"
}

declare_lint_pass!(NeedlessPathNew => [NEEDLESS_PATH_NEW]);

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        match e.kind {
            ExprKind::Call(fn_expr, args) => {
                check_arguments(cx, &mut args.iter(), cx.typeck_results().expr_ty(fn_expr));
            },
            ExprKind::MethodCall(_, receiver, arguments, _)
                if let Some(def_id) = cx.typeck_results().type_dependent_def_id(e.hir_id) =>
            {
                let args = cx.typeck_results().node_args(e.hir_id);
                let method_type = cx.tcx.type_of(def_id).instantiate(cx.tcx, args);
                check_arguments(cx, &mut iter::once(receiver).chain(arguments.iter()), method_type);
            },
            _ => (),
        }
    }
}

fn check_arguments<'tcx>(
    cx: &LateContext<'tcx>,
    arguments: &mut dyn Iterator<Item = &'tcx Expr<'tcx>>,
    type_definition: Ty<'tcx>,
) {
    let tcx = cx.tcx;
    // whether `func` is `Path::new`
    let is_path_new = |func: &Expr<'_>| {
        if let ExprKind::Path(ref qpath) = func.kind
            && let QPath::TypeRelative(ty, path) = qpath
            && let Some(did) = path_res(cx, *ty).opt_def_id()
            && tcx.is_diagnostic_item(sym::Path, did)
            && path.ident.name == sym::new
        {
            true
        } else {
            false
        }
    };

    let Some(path_def_id) = tcx.get_diagnostic_item(sym::Path) else {
        return;
    };
    let path_ty = Ty::new_adt(tcx, tcx.adt_def(path_def_id), List::empty());
    let Some(asref_def_id) = tcx.get_diagnostic_item(sym::AsRef) else {
        return;
    };

    let implements_asref_path = |arg| implements_trait(cx, arg, asref_def_id, &[path_ty.into()]);

    if let ty::FnDef(def_id, ..) = type_definition.kind()
        // if there are any bound vars, just give up... we might be able to be smarter here
        && let Some(fn_sig) = type_definition.fn_sig(tcx).no_bound_vars()
    {
        let parameters = fn_sig.inputs();

        let bounds = tcx.param_env(def_id).caller_bounds();
        dbg!(bounds);

        for (argument, parameter) in iter::zip(arguments, parameters) {
            // we want `argument` to be `Path::new(x)`, which has one arg, x
            if let ExprKind::Call(func, [arg]) = argument.kind
                && is_path_new(func)
                && implements_asref_path(cx.typeck_results().expr_ty(arg))
                && implements_asref_path(*parameter)
            {
                span_lint_and_sugg(
                    cx,
                    NEEDLESS_PATH_NEW,
                    argument.span,
                    "the expression enclosed in `Path::new` implements `AsRef<Path>`",
                    "remove the enclosing `Path::new`",
                    format!("{}", snippet(cx, arg.span, "..")),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}
