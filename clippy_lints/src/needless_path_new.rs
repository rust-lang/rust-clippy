use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::ty::implements_trait;
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
    let implements_asref_path = |arg| {
        cx.tcx.get_diagnostic_item(sym::AsRef).map_or(false, |id| {
            implements_trait(cx, cx.typeck_results().expr_ty(arg), id, &[])
        })
    };

    if let ty::FnDef(..) | ty::FnPtr(..) = type_definition.kind() {
        let parameters = type_definition.fn_sig(cx.tcx).skip_binder().inputs();
        for (argument, parameter) in iter::zip(arguments, parameters) {
            // if implements_asref_path(argument) &&
            if let ExprKind::Call(path_new, path_new_arg) = argument.kind
                && path_new_arg.len() == 1
                && implements_asref_path(&path_new_arg[0])
                && let ty::Ref(_, _, Mutability::Not) | ty::RawPtr(_, Mutability::Not) = parameter.kind()
            {
                span_lint_and_help(
                    cx,
                    NEEDLESS_PATH_NEW,
                    argument.span,
                    "`Path::new` used",
                    None,
                    "consider removing `Path::new`",
                );
            }
        }
    }
}
