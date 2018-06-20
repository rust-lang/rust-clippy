use rustc::lint::*;
use rustc::hir::*;
use crate::utils::span_lint;

/// **What it does:** Checks for enums named `Sex` or `Gender` with only two variants
///
/// **Why is this bad?** This is likely to indicate code that cannot properly
/// model the data it is trying to represent.
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
/// enum Sex {
///     Female,
///     Male,
/// }
/// struct Person {
///     sex: Sex,
///     // other fields
/// }
/// ```
///
/// Could be written:
///
/// ```rust
/// struct Person {
///     sex: &str,
///     // other fields
/// }
/// ```
declare_clippy_lint! {
    pub ENUM_SEX,
    pedantic,
    "enum named `Sex` or `Gender` with only two variants"
}

#[derive(Copy, Clone)]
pub struct EnumSex;

impl LintPass for EnumSex {
    fn get_lints(&self) -> LintArray {
        lint_array!(ENUM_SEX)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for EnumSex {
    fn check_item(&mut self, cx: &LateContext, item: &Item) {
        if item.name.as_str() == "Sex" || item.name.as_str() == "Gender" {
            if let ItemEnum(ref enumdef, _) = item.node {
                if enumdef.variants.len() == 2 {
                    span_lint(cx, ENUM_SEX, item.span, "enum named `Sex` or `Gender` with only two variants");
                }
            }
        }
    }
}
