use super::NULL_POINTER_OPTIMIZATION;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{is_lang_item_or_ctor, last_path_segment, match_def_path, paths};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{GenericArg, LangItem, QPath, Ty, TyKind};
use rustc_lint::LateContext;

pub(super) fn check(cx: &LateContext<'_>, ty: &Ty<'_>, qpath: &QPath<'_>, res: Res) -> bool {
    if let Res::Def(DefKind::Struct, def_id) = res
        && (is_lang_item_or_ctor(cx, def_id, LangItem::OwnedBox) || match_def_path(cx, def_id, &paths::PTR_NON_NULL))
        && let Some(args) = last_path_segment(qpath).args
        && let GenericArg::Type(option_ty) = args.args[0]
        && let TyKind::Path(option_qpath) = option_ty.kind
        && let res = cx.qpath_res(&option_qpath, option_ty.hir_id)
        && let Res::Def(.., def_id) = res
        && is_lang_item_or_ctor(cx, def_id, LangItem::Option)
    {
        let outer_ty = last_path_segment(qpath).ident.name;
        span_lint_and_help(
            cx,
            NULL_POINTER_OPTIMIZATION,
            ty.span,
            &format!("usage of `{outer_ty}<Option<T>>`"),
            None,
            &format!(
                "consider using `Option<{outer_ty}<T>>` instead, as it will grant better performance. For more information, see\n\
                 https://doc.rust-lang.org/core/option/#representation",
            ),
        );

        return true;
    }

    false
}
