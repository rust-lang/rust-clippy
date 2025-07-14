use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::path_res;
use clippy_utils::source::snippet;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, List, ParamTy, Ty, TyCtxt};
use rustc_session::impl_lint_pass;
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

impl_lint_pass!(NeedlessPathNew<'_> => [NEEDLESS_PATH_NEW]);

pub struct NeedlessPathNew<'tcx> {
    path_ty: Option<Ty<'tcx>>,
    asref_def_id: Option<DefId>,
}

impl<'tcx> NeedlessPathNew<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            path_ty: (tcx.get_diagnostic_item(sym::Path))
                .map(|path_def_id| Ty::new_adt(tcx, tcx.adt_def(path_def_id), List::empty())),
            asref_def_id: tcx.get_diagnostic_item(sym::AsRef),
        }
    }
}

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

impl<'tcx> LateLintPass<'tcx> for NeedlessPathNew<'tcx> {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) {
        let tcx = cx.tcx;

        let Some(path_ty) = self.path_ty else {
            return;
        };

        let Some(asref_def_id) = self.asref_def_id else {
            return;
        };

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

        let implements_asref_path = |arg| implements_trait(cx, arg, asref_def_id, &[path_ty.into()]);

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
                    && implements_asref_path(*arg_ty)
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
