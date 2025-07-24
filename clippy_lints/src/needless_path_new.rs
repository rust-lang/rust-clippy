use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::path_res;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, GenericPredicates, ParamTy, Ty};
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
    #[clippy::version = "1.90.0"]
    pub NEEDLESS_PATH_NEW,
    nursery,
    "an argument passed to a function  that accepts `impl AsRef<Path>` \
    being enclosed in `Path::new` when the argument implements the trait"
}

declare_lint_pass!(NeedlessPathNew => [NEEDLESS_PATH_NEW]);

fn is_used_anywhere_else<'a>(param_ty: &'_ ParamTy, mut other_sig_tys: impl Iterator<Item = Ty<'a>>) -> bool {
    other_sig_tys.any(|sig_ty| {
        sig_ty.walk().any(|generic_arg| {
            if let Some(ty) = generic_arg.as_type()
                && let ty::Param(pt) = ty.kind()
                && pt == param_ty
            {
                true
            } else {
                false
            }
        })
    })
}

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        let tcx = cx.tcx;

        let (fn_did, args) = match e.kind {
            ExprKind::Call(callee, args)
                if let Res::Def(DefKind::Fn | DefKind::AssocFn | DefKind::Ctor(_, CtorKind::Fn), did) =
                    path_res(cx, callee) =>
            {
                (did, args)
            },
            ExprKind::MethodCall(_, _, args, _)
                if let Some(did) = cx.typeck_results().type_dependent_def_id(e.hir_id) =>
            {
                (did, args)
            },
            _ => return,
        };

        let sig = tcx.fn_sig(fn_did).skip_binder().skip_binder();

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

        let has_required_preds = |_param_ty: &ParamTy, _preds: GenericPredicates<'_>| -> bool { true };

        // as far as I understand, `ExprKind::MethodCall` doesn't include the receiver in `args`,
        // but does in `sig.inputs()` -- so we iterate over both in `rev`erse in order to line
        // them up starting from the _end_
        //
        // and for `ExprKind::Call` this is basically a no-op
        iter::zip(sig.inputs().iter().rev(), args.iter().rev())
            .enumerate()
            .for_each(|(arg_idx, (arg_ty, arg))| {
                // we want `argument` to be `Path::new(x)`
                if let ExprKind::Call(path_new, [x]) = arg.kind
                    && is_path_new(path_new)
                    && let ty::Param(arg_param_ty) = arg_ty.kind()
                    && !is_used_anywhere_else(
                        arg_param_ty,
                        sig.inputs()
                            .iter()
                            // `arg_idx` is based on the reversed order, so we need to reverse as well
                            .rev()
                            .enumerate()
                            .filter_map(|(i, input)| (i != arg_idx).then_some(*input)),
                    )
                    && has_required_preds(arg_param_ty, cx.tcx.predicates_of(fn_did))
                {
                    span_lint_and_sugg(
                        cx,
                        NEEDLESS_PATH_NEW,
                        arg.span,
                        "the expression enclosed in `Path::new` implements `AsRef<Path>`",
                        "remove the enclosing `Path::new`",
                        format!("{}", snippet(cx, x.span, "..")),
                        Applicability::MachineApplicable,
                    );
                }
            })
    }
}
