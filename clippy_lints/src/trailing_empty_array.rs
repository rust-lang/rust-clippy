use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::has_repr_attr;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Const;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Displays a warning when a struct with a trailing zero-sized array is declared without a `repr` attribute.
    ///
    /// ### Why is this bad?
    /// Zero-sized arrays aren't very useful in Rust itself, so such a struct is likely being created to pass to C code or in some other situation where control over memory layout matters (for example, in conjunction with manual allocation to make it easy to compute the offset of the array). Either way, `#[repr(C)]` (or another `repr` attribute) is needed.
    ///
    /// ### Example
    /// ```rust
    /// struct RarelyUseful {
    ///     some_field: u32,
    ///     last: [u32; 0],
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// #[repr(C)]
    /// struct MoreOftenUseful {
    ///     some_field: usize,
    ///     last: [u32; 0],
    /// }
    /// ```
    #[clippy::version = "1.58.0"]
    pub TRAILING_EMPTY_ARRAY,
    nursery,
    "struct with a trailing zero-sized array but without `#[repr(C)]` or another `repr` attribute"
}
declare_lint_pass!(TrailingEmptyArray => [TRAILING_EMPTY_ARRAY]);

impl<'tcx> LateLintPass<'tcx> for TrailingEmptyArray {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if is_struct_with_trailing_zero_sized_array(cx, item) && !has_repr_attr(cx, item.hir_id()) {
            span_lint_and_help(
                cx,
                TRAILING_EMPTY_ARRAY,
                item.span,
                "trailing zero-sized array in a struct which is not marked with a `repr` attribute",
                None,
                &format!(
                    "consider annotating `{}` with `#[repr(C)]` or another `repr` attribute",
                    cx.tcx.def_path_str(item.owner_id.to_def_id())
                ),
            );
        }
    }
}

fn is_struct_with_trailing_zero_sized_array(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    if_chain! {
        // First check if last field is an array
        if let ItemKind::Struct(data, _) = &item.kind;
        if let Some(last_field) = data.fields().last();
        if let rustc_hir::TyKind::Array(_, rustc_hir::ArrayLen::Body(length)) = last_field.ty.kind;

        // Then check if that that array zero-sized
        let length = Const::from_anon_const(cx.tcx, length.def_id);
        let length = length.try_eval_target_usize(cx.tcx, cx.param_env);
        if let Some(length) = length;
        then {
            length == 0
        } else {
            false
        }
    }
}
