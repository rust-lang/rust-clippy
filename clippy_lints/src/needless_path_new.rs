use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;
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
    #[clippy::version = "1.90.0"]
    pub NEEDLESS_PATH_NEW,
    nursery,
    "an argument passed to a function  that accepts `impl AsRef<Path>` \
    being enclosed in `Path::new` when the argument implements the trait"
}

declare_lint_pass!(NeedlessPathNew => [NEEDLESS_PATH_NEW]);

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        match e.kind {
            ExprKind::Call(fn_expr, args) if let ExprKind::Path(ref path) = fn_expr.kind => {
                check_arguments(
                    cx,
                    &mut args.iter(),
                    cx.typeck_results().expr_ty(fn_expr),
                    &rustc_hir_pretty::qpath_to_string(&cx.tcx, path),
                    "function",
                );
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

fn flag() {
    span_lint_and_help(
        cx,
        NEEDLESS_PATH_NEW,
        expr.span,
        "`Path::new` used",
        None,
        "consider removing `Path::new`",
    );
}

fn check_arguments<'tcx>(
    cx: &LateContext<'tcx>,
    arguments: &mut dyn Iterator<Item = &'tcx Expr<'tcx>>,
    type_definition: Ty<'tcx>,
    name: &str,
    fn_kind: &str,
) {
    if let ty::FnDef(..) | ty::FnPtr(..) = type_definition.kind() {
        let parameters = type_definition.fn_sig(cx.tcx).skip_binder().inputs();
        for (argument, parameter) in iter::zip(arguments, parameters) {
            if let ty::Ref(_, _, Mutability::Not) | ty::RawPtr(_, Mutability::Not) = parameter.kind()
                && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, _) = argument.kind
            {
                span_lint(
                    cx,
                    UNNECESSARY_MUT_PASSED,
                    argument.span,
                    format!("the {fn_kind} `{name}` doesn't need a mutable reference"),
                );
            }
        }
    }
    if let ExprKind::MethodCall(method_name, receiver, args, _) = expr.kind
        && method_name.ident.as_str() == "unwrap"
        && let ExprKind::Call(func, args1) = receiver.kind
        && let ExprKind::Path(ref qpath) = func.kind
        // && match_qpath(qpath, &["fs", "copy"])
        && args1.len() == 2
        && let ExprKind::Call(func1, args2) = args1[0].kind
        && let ExprKind::Path(ref qpath1) = func1.kind
        && match_qpath(qpath1, &["path", "Path", "new"])
        && args2.len() == 1
        && let ExprKind::Lit(ref lit) = args2[0].kind
        && let LitKind::Str(s, _) = lit.node
        && s.as_str() == "foo"
        && let ExprKind::Lit(ref lit1) = args1[1].kind
        && let LitKind::Str(s1, _) = lit1.node
        // && s1.as_str() == "a"
        && args.is_empty()
    {
        true
    } else {
        false
    }
}
