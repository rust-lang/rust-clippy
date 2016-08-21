//! lint when there are large variants on an enum

use rustc::lint::*;
use rustc::hir::*;
use utils::span_help_and_lint;
use rustc::ty::layout::TargetDataLayout;

/// **What it does:** Checks for large variants on enums.
///
/// **Why is this bad?** Enum size is bounded by the largest variant. Having a large variant
/// can penalize the memory layout of that enum.
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
/// enum Test {
///    A(i32),
///    B([i32; 8000]),
/// }
/// ```
declare_lint! {
    pub ENUM_LARGE_VARIANT,
    Warn,
    "large variants on an enum"
}

#[derive(Copy,Clone)]
pub struct EnumLargeVariant {
    maximum_variant_size_allowed: u64,
}

impl EnumLargeVariant {
    pub fn new(maximum_variant_size_allowed: u64) -> Self {
        EnumLargeVariant {
            maximum_variant_size_allowed: maximum_variant_size_allowed,
        }
    }
}

impl LintPass for EnumLargeVariant {
    fn get_lints(&self) -> LintArray {
        lint_array!(ENUM_LARGE_VARIANT)
    }
}

impl LateLintPass for EnumLargeVariant {
    fn check_variant(&mut self, cx: &LateContext, variant: &Variant, _ :&Generics) {
        let data_layout = TargetDataLayout::parse(cx.sess());
        let param_env = cx.tcx.empty_parameter_environment();
        let infcx = cx.tcx.borrowck_fake_infer_ctxt(param_env);
        let mut variant_size = 0;

        for field in variant.node.data.fields() {
            let ty = cx.tcx.node_id_to_type(field.id);
            if let Ok(layout) = ty.layout(&infcx) {
                variant_size += layout.size(&data_layout).bytes();
            }
        }

        if variant_size > self.maximum_variant_size_allowed {
            span_help_and_lint(
                cx,
                ENUM_LARGE_VARIANT,
                variant.span,
                &format!("large enum variant found on variant `{}`", variant.node.name),
                "consider boxing the large branches to reduce the total size of the enum",
            );
        }
    }
}
