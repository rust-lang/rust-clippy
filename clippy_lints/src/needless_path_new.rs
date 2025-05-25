use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::path_res;
use clippy_utils::source::snippet;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;
use rustc_span::sym;
use std::iter;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    /// Too verbose
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
    #[clippy::version = "1.88.0"]
    pub NEEDLESS_PATH_NEW,
    nursery,
    "default lint description"
}

declare_lint_pass!(NeedlessPathNew => [NEEDLESS_PATH_NEW]);

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        match e.kind {
            ExprKind::Call(fn_expr, args) => {
                if let ExprKind::Path(ref path) = fn_expr.kind {
                    check_arguments(
                        cx,
                        &mut args.iter(),
                        cx.typeck_results().expr_ty(fn_expr),
                        &rustc_hir_pretty::qpath_to_string(&cx.tcx, path),
                        "function",
                    );
                }
            },
            ExprKind::MethodCall(path, receiver, arguments, _)
                if let Some(def_id) = cx.typeck_results().type_dependent_def_id(e.hir_id) =>
            {
                let args = cx.typeck_results().node_args(e.hir_id);
                let method_type = cx.tcx.type_of(def_id).instantiate(cx.tcx, args);
                check_arguments(
                    cx,
                    &mut iter::once(receiver).chain(arguments.iter()),
                    method_type,
                    path.ident.as_str(),
                    "method",
                );
            },
            _ => (),
        }
    }
}

fn check_arguments<'tcx>(
    cx: &LateContext<'tcx>,
    arguments: &mut dyn Iterator<Item = &'tcx Expr<'tcx>>,
    type_definition: Ty<'tcx>,
    name: &str,
    fn_kind: &str,
) {
    // whether `func` is `Path::new`
    let is_path_new = |func: &Expr<'_>| {
        if let ExprKind::Path(ref qpath) = func.kind
            && let QPath::TypeRelative(ty, path) = qpath
            && let Some(did) = path_res(cx, *ty).opt_def_id()
            && cx.tcx.is_diagnostic_item(sym::Path, did)
            && path.ident.name == sym::new
        {
            true
        } else {
            false
        }
    };

    let implements_asref_path = |arg| {
        if let Some(path_def_id) = cx.tcx.get_diagnostic_item(sym::Path)
            && let path_ty = cx.tcx.type_of(path_def_id).skip_binder()
            && let Some(asref_def_id) = cx.tcx.get_diagnostic_item(sym::AsRef)
        {
            implements_trait(cx, arg, asref_def_id, &[path_ty.into()])
        } else {
            false
        }
    };

    if let ty::FnDef(..) | ty::FnPtr(..) = type_definition.kind() {
        let parameters = type_definition.fn_sig(cx.tcx).skip_binder().inputs();
        for (argument, parameter) in iter::zip(arguments, parameters) {
            if let ExprKind::Call(func, args) = argument.kind
                && is_path_new(func)
                // I guess this check is superfluous,
                // since we know how many parameters `Path::new` takes
                && args.len() == 1
                && implements_asref_path(cx.typeck_results().expr_ty(&args[0]))
                && implements_asref_path(*parameter)
            {
                span_lint_and_sugg(
                    cx,
                    NEEDLESS_PATH_NEW,
                    argument.span,
                    "the expression enclosed in `Path::new` implements `AsRef<Path>`",
                    "remove the enclosing `Path::new`",
                    format!("{}", snippet(cx, args[0].span, "..")),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}
