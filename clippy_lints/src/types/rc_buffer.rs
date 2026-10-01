use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeResPath as _;
use clippy_utils::source::snippet;
use clippy_utils::{qpath_generic_tys, sym};
use rustc_hir::def_id::DefId;
use rustc_hir::{QPath, Ty, TyKind};
use rustc_lint::LateContext;
use std::borrow::Cow;

use super::RC_BUFFER;

pub(super) fn check(cx: &LateContext<'_>, hir_ty: &Ty<'_>, qpath: &QPath<'_>, def_id: DefId) -> bool {
    let rc = match cx.tcx.get_diagnostic_name(def_id) {
        Some(sym::Rc) => "Rc",
        Some(sym::Arc) => "Arc",
        _ => return false,
    };
    if let Some(ty) = qpath_generic_tys(qpath).next()
        && let Some(alternate) = match_buffer_type(cx, ty)
    {
        span_lint_and_then(
            cx,
            RC_BUFFER,
            hir_ty.span,
            format!("usage of `{rc}<T>` when `T` is a buffer type"),
            |diag| {
                diag.help(format!(
                    "consider using `{alternate}` if length-changing mutations are not needed"
                ));
                diag.note("changing the type requires reallocating the buffer");
            },
        );
        true
    } else {
        false
    }
}

fn match_buffer_type(cx: &LateContext<'_>, ty: &Ty<'_>) -> Option<Cow<'static, str>> {
    let id = ty.basic_res().opt_def_id()?;
    let path = match cx.tcx.get_diagnostic_name(id) {
        Some(sym::OsString) => "std::ffi::OsStr".into(),
        Some(sym::PathBuf) => "std::path::Path".into(),
        Some(sym::Vec) => {
            let TyKind::Path(vec_qpath) = &ty.kind else {
                return None;
            };
            let vec_generic_ty = qpath_generic_tys(vec_qpath).next()?;
            let snippet = snippet(cx, vec_generic_ty.span, "_");
            format!("[{snippet}]").into()
        },
        _ if Some(id) == cx.tcx.lang_items().string() => "str".into(),
        _ => return None,
    };
    Some(path)
}
