use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::ty::is_type_lang_item;
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.collect::<Vec<String>>().join("")` on iterators.
    ///
    /// ### Why is this bad?
    /// `.collect::<String>()` is more concise and might be more performant
    ///
    /// ### Example
    /// ```no_run
    /// let vector = vec!["hello",  "world"];
    /// let output = vector.iter().map(|item| item.to_uppercase()).collect::<Vec<String>>().join("");
    /// println!("{}", output);
    /// ```
    /// The correct use would be:
    /// ```no_run
    /// let vector = vec!["hello",  "world"];
    /// let output = vector.iter().map(|item| item.to_uppercase()).collect::<String>();
    /// println!("{}", output);
    /// ```
    /// ### Known problems
    /// While `.collect::<String>()` is sometimes more performant, there are cases where
    /// using `.collect::<String>()` over `.collect::<Vec<String>>().join("")`
    /// will prevent loop unrolling and will result in a negative performance impact.
    ///
    /// Additionally, differences have been observed between aarch64 and x86_64 assembly output,
    /// with aarch64 tending to producing faster assembly in more cases when using `.collect::<String>()`
    #[clippy::version = "1.61.0"]
    pub UNNECESSARY_JOIN,
    pedantic,
    "using `.collect::<Vec<String>>().join(\"\")` on an iterator"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    join_self_arg: &'tcx Expr<'tcx>,
    join_arg: &'tcx Expr<'tcx>,
    span: Span,
) {
    let applicability = Applicability::MachineApplicable;
    let collect_output_adjusted_type = cx.typeck_results().expr_ty_adjusted(join_self_arg);
    if let ty::Ref(_, ref_type, _) = collect_output_adjusted_type.kind()
        // the turbofish for collect is ::<Vec<String>>
        && let ty::Slice(slice) = ref_type.kind()
        && is_type_lang_item(cx, *slice, LangItem::String)
        // the argument for join is ""
        && let ExprKind::Lit(spanned) = &join_arg.kind
        && let LitKind::Str(symbol, _) = spanned.node
        && symbol.is_empty()
    {
        span_lint_and_sugg(
            cx,
            UNNECESSARY_JOIN,
            span.with_hi(expr.span.hi()),
            r#"called `.collect::<Vec<String>>().join("")` on an iterator"#,
            "consider using",
            "collect::<String>()".to_owned(),
            applicability,
        );
    }
}
