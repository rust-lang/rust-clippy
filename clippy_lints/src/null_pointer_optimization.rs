use clippy_utils::{diagnostics::span_lint_and_help, is_lang_item_or_ctor, last_path_segment, match_def_path, paths};
use rustc_hir::{
    def::{DefKind, Res},
    GenericArg, LangItem, Ty, TyKind,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `T<Option<U>>` where `T` is a type that has
    /// [null pointer optimization](https://doc.rust-lang.org/core/option/#representation).
    ///
    /// ### Why is this bad?
    /// It's slower, as `Option` can use `null` as `None`, instead of adding another layer of
    /// indirection.
    ///
    /// ### Example
    /// ```rust
    /// struct MyWrapperType<T>(Box<Option<T>>);
    /// ```
    /// Use instead:
    /// ```rust
    /// struct MyWrapperType<T>(Option<Box<T>>);
    /// ```
    #[clippy::version = "1.72.0"]
    pub NULL_POINTER_OPTIMIZATION,
    perf,
    "checks for `U<Option<T>>` where `U` is a type that has null pointer optimization"
}
impl_lint_pass!(NullPointerOptimization => [NULL_POINTER_OPTIMIZATION]);

#[derive(Clone, Copy)]
pub struct NullPointerOptimization {
    pub avoid_breaking_exported_api: bool,
}

impl LateLintPass<'_> for NullPointerOptimization {
    fn check_ty(&mut self, cx: &LateContext<'_>, ty: &Ty<'_>) {
        if let TyKind::Path(qpath) = ty.kind
            && let res = cx.qpath_res(&qpath, ty.hir_id)
            && let Res::Def(DefKind::Struct, def_id) = res
        {
            if !(is_lang_item_or_ctor(cx, def_id, LangItem::OwnedBox)
                || match_def_path(cx, def_id, &paths::PTR_NON_NULL))
            {
                return;
            }

            if let Some(args) = last_path_segment(&qpath).args
                && let GenericArg::Type(option_ty) = args.args[0]
                && let TyKind::Path(option_qpath) = option_ty.kind
                && let res = cx.qpath_res(&option_qpath, option_ty.hir_id)
                && let Res::Def(.., def_id) = res
                && is_lang_item_or_ctor(cx, def_id, LangItem::Option)
            {
                let outer_ty = last_path_segment(&qpath).ident.name;
                span_lint_and_help(
                    cx,
                    NULL_POINTER_OPTIMIZATION,
                    ty.span,
                    &format!("usage of `{outer_ty}<Option<T>>`"),
                    None,
                    &format!("consider using `Option<{outer_ty}<T>>` instead, as it will grant better performance. For more information, see\n\
                                https://doc.rust-lang.org/core/option/#representation"),
                );
            }
        }
    }
}
