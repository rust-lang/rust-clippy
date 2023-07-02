use clippy_utils::{
    diagnostics::span_lint_and_then, get_parent_expr, is_from_proc_macro, match_def_path, path_res, paths::PATH_NEW,
    ty::is_type_diagnostic_item,
};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::def_id::DefId;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{lint::in_external_macro, ty};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{sym, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// TODO please do soon
    ///
    /// ### Why is this bad?
    /// TODO please
    ///
    /// ### Example
    /// ```rust
    /// // TODO
    /// ```
    /// Use instead:
    /// ```rust
    /// // TODO
    /// ```
    #[clippy::version = "1.72.0"]
    pub BARE_DOS_DEVICE_NAMES,
    suspicious,
    "usage of paths that, on Windows, will implicitly refer to a DOS device"
}
declare_lint_pass!(BareDosDeviceNames => [BARE_DOS_DEVICE_NAMES]);

impl<'tcx> LateLintPass<'tcx> for BareDosDeviceNames {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !in_external_macro(cx.sess(), expr.span)
            && let ExprKind::Lit(arg) = expr.kind
            && let LitKind::Str(str_sym, _) = arg.node
            && matches!(
                &*str_sym.as_str().to_ascii_lowercase(),
                "aux"
                    | "con"
                    | "conin$"
                    // ^^^^^^
                    | "conout$"
                    // ^^^^^^^
                    // TODO: Maybe these two can be an exception.
                    //
                    // Using `CONIN$` and `CONOUT$` is common enough in other languages that it may
                    // trip up a couple newbies coming to rust. Besides, it's unlikely someone will
                    // ever use `CONIN$` as a filename.
                    | "com1"
                    | "com2"
                    | "com3"
                    | "com4"
                    | "com5"
                    | "com6"
                    | "com7"
                    | "com8"
                    | "com9"
                    | "lpt1"
                    | "lpt2"
                    | "lpt3"
                    | "lpt4"
                    | "lpt5"
                    | "lpt6"
                    | "lpt7"
                    | "lpt8"
                    | "lpt9"
                    | "nul"
                    | "prn"
                )
            && let Some(parent) = get_parent_expr(cx, expr)
            && (is_path_buf_from_or_path_new(cx, parent) || is_path_ty(cx, expr, parent))
            && !is_from_proc_macro(cx, expr)
        {
            span_lint_and_then(
                cx,
                BARE_DOS_DEVICE_NAMES,
                expr.span,
                "this path refers to a DOS device",
                |diag| {
                    // Suggest making current behavior explicit
                    diag.span_suggestion_verbose(
                        expr.span,
                        "if this is intended, try",
                        format!(r#""\\.\{str_sym}""#),
                        Applicability::MaybeIncorrect,
                    );

                    // Suggest making the code refer to a file or folder in the current directory
                    diag.span_suggestion_verbose(
                        expr.span,
                        "if this was intended to point to a file or folder, try",
                        format!("\"./{str_sym}\""),
                        Applicability::MaybeIncorrect,
                    );
                }
            );
        }
    }
}

/// Gets whether the `Expr` is an argument to `Path::new` or `PathBuf::from`. The caller must
/// provide the parent `Expr`, for performance's sake.
///
/// TODO: We can likely refactor this like we did with `LINTED_TRAITS`.
fn is_path_buf_from_or_path_new(cx: &LateContext<'_>, parent: &Expr<'_>) -> bool {
    if let ExprKind::Call(path, _) = parent.kind
        && let ExprKind::Path(qpath) = path.kind
        && let QPath::TypeRelative(ty, last_segment) = qpath
        && let Some(call_def_id) = path_res(cx, path).opt_def_id()
        && let Some(ty_def_id) = path_res(cx, ty).opt_def_id()
        && (match_def_path(cx, call_def_id, &PATH_NEW)
            // `PathBuf::from` is unfortunately tricky, as all we end up having for `match_def_path`
            // is `core::convert::From::from`, not `std::path::PathBuf::from`. Basically useless.
            || cx.tcx.is_diagnostic_item(sym::PathBuf, ty_def_id) && last_segment.ident.as_str() == "from")
    {
        return true;
    }

    false
}

/// Gets the `DefId` and arguments of `expr`, if it's a `Call` or `MethodCall`
fn get_def_id_and_args<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<(DefId, &'tcx [Expr<'tcx>])> {
    match expr.kind {
        ExprKind::Call(path, args) => Some((path_res(cx, path).opt_def_id()?, args)),
        ExprKind::MethodCall(_, _, args, _) => Some((cx.typeck_results().type_dependent_def_id(expr.hir_id)?, args)),
        _ => None,
    }
}

/// Given a `Ty`, returns whether it is likely a path type, like `Path` or `PathBuf`. Also returns
/// true if it's `impl AsRef<Path>`, `T: AsRef<Path>`, etc. You get the idea.
fn is_path_ty<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, parent: &'tcx Expr<'tcx>) -> bool {
    const LINTED_TRAITS: &[(Symbol, Symbol)] = &[
        (sym::AsRef, sym::Path),
        (sym::Into, sym::PathBuf),
        (sym::Into, sym::Path),
        // TODO: Let's add more traits here.
    ];

    let Some((callee, callee_args)) = get_def_id_and_args(cx, parent) else {
        return false;
    };
    let Some(arg_index) = callee_args.iter().position(|arg| arg.hir_id == expr.hir_id) else {
        return false;
    };
    let arg_ty = cx.tcx.fn_sig(callee).subst_identity().inputs().skip_binder()[arg_index].peel_refs();

    // If we find `PathBuf` or `Path`, no need to check `impl <trait>` or `T`.
    if let Some(def) = arg_ty.ty_adt_def()
        && let def_id = def.did()
        && (cx.tcx.is_diagnostic_item(sym::PathBuf, def_id) || cx.tcx.is_diagnostic_item(sym::Path, def_id))
    {
        return true;
    }

    for predicate in cx
        .tcx
        .param_env(callee)
        .caller_bounds()
        .iter()
        .filter_map(|predicate| predicate.kind().no_bound_vars())
    {
        if let ty::ClauseKind::Trait(trit) = predicate
            && trit.trait_ref.self_ty() == arg_ty
            // I believe `0` is always `Self`, so `T` or `impl <trait>`
            && let [_, subst] = trit.trait_ref.substs.as_slice()
            && let Some(as_ref_ty) = subst.as_type()
        {
            for (trait_sym, ty_sym) in LINTED_TRAITS {
                if cx.tcx.is_diagnostic_item(*trait_sym, trit.trait_ref.def_id)
                    && is_type_diagnostic_item(cx, as_ref_ty, *ty_sym)
                {
                    return true;
                }
            }
        }
    }

    false
}
