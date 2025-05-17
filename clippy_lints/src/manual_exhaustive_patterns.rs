use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_res_lang_ctor, path_res};
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::LangItem::{OptionNone, OptionSome, ResultErr, ResultOk};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Pat, PatExprKind, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Looks for combined OR patterns that go though all of the possible variants of a built in enum or of boolean,
    /// e.g. `(Some(_) | None, 6)` can be rewritten as `(_, 6)`.
    ///
    /// ### Why is this bad?
    /// `_` is more concise and makes it clear that all possible values are captured. This is more obvious when matches are on more elements
    ///
    /// ### Example
    /// ```no_run
    /// let x = None;
    /// let foo = matches!(x, Some(_) | None);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x = None;
    /// let foo = matches!(x, _);
    /// ```
    #[clippy::version = "1.88.0"]
    pub MANUAL_EXHAUSTIVE_PATTERNS,
    complexity,
    "manually writing exhaustive patterns using a combined OR pattern (`|`)"
}
declare_lint_pass!(ManualExhaustivePatterns => [MANUAL_EXHAUSTIVE_PATTERNS]);

impl LateLintPass<'_> for ManualExhaustivePatterns {
    fn check_pat(&mut self, cx: &LateContext<'_>, pat: &'_ Pat<'_>) {
        if let PatKind::Or(pats) = pat.kind
            && pats.len() == 2
            && !pat.span.in_external_macro(cx.sess().source_map())
        {
            if let Some(x) = extract_bool(pats[0]) {
                if let Some(y) = extract_bool(pats[1])
                    && x != y
                {
                    apply_lint(cx, pat);
                }
                return;
            }

            let left = extract_built_in_enum(cx, pats[0]);

            if left != BuiltinEnum::Other {
                let right = extract_built_in_enum(cx, pats[1]);
                if right != BuiltinEnum::Other && left != right {
                    apply_lint(cx, pat);
                }
            }
        }
    }
}

fn apply_lint(cx: &LateContext<'_>, pat: &'_ Pat<'_>) {
    span_lint_and_then(
        cx,
        MANUAL_EXHAUSTIVE_PATTERNS,
        pat.span,
        "this OR pattern can be rewritten using a wildcard",
        |diag| {
            diag.span_suggestion(pat.span, "try", format!("_"), Applicability::MachineApplicable);
        },
    );
}

#[derive(PartialEq, Eq, Debug)]
enum BuiltinEnum {
    WildSome,
    WildOk,
    WildErr,
    None,
    Other,
}

// switch to custom enum response
fn extract_built_in_enum(cx: &LateContext<'_>, pattern: Pat<'_>) -> BuiltinEnum {
    if let PatKind::TupleStruct(QPath::Resolved(None, path), inner, _) = pattern.kind
        && let Res::Def(DefKind::Ctor(..), def_id) = path.res
        && let PatKind::Wild = inner[0].kind
    {
        if clippy_utils::is_lang_item_or_ctor(cx, def_id, OptionSome) {
            return BuiltinEnum::WildSome;
        }
        if clippy_utils::is_lang_item_or_ctor(cx, def_id, ResultOk) {
            return BuiltinEnum::WildOk;
        }
        if clippy_utils::is_lang_item_or_ctor(cx, def_id, ResultErr) {
            return BuiltinEnum::WildErr;
        }
    }
    if is_res_lang_ctor(cx, path_res(cx, &pattern), OptionNone) {
        return BuiltinEnum::None;
    }
    return BuiltinEnum::Other;
}

fn extract_bool(pattern: Pat<'_>) -> Option<bool> {
    if let PatKind::Expr(pat_expr) = pattern.kind
        && let PatExprKind::Lit { lit, .. } = pat_expr.kind
        && let LitKind::Bool(x) = lit.node
    {
        return Some(x);
    }
    None
}
