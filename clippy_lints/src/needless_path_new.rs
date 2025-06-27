use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::path_res;
use clippy_utils::source::snippet;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{List, Ty, TyCtxt};
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

        let parameters = sig.inputs();

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
