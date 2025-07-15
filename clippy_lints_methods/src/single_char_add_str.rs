use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{snippet_with_applicability, str_literal_to_char_literal};
use rustc_ast::BorrowKind;
use rustc_errors::Applicability;
use rustc_hir::{self as hir, ExprKind};
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Warns when using `push_str`/`insert_str` with a single-character string literal
    /// where `push`/`insert` with a `char` would work fine.
    ///
    /// ### Why is this bad?
    /// It's less clear that we are pushing a single character.
    ///
    /// ### Example
    /// ```no_run
    /// # let mut string = String::new();
    /// string.insert_str(0, "R");
    /// string.push_str("R");
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let mut string = String::new();
    /// string.insert(0, 'R');
    /// string.push('R');
    /// ```
    #[clippy::version = "1.49.0"]
    pub SINGLE_CHAR_ADD_STR,
    style,
    "`push_str()` or `insert_str()` used with a single-character string literal as parameter"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, receiver: &hir::Expr<'_>, args: &[hir::Expr<'_>]) {
    if let Some(fn_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) {
        if cx.tcx.is_diagnostic_item(sym::string_push_str, fn_def_id) {
            check_push_str(cx, expr, receiver, args);
        } else if cx.tcx.is_diagnostic_item(sym::string_insert_str, fn_def_id) {
            check_insert_str(cx, expr, receiver, args);
        }
    }
}

/// lint for length-1 `str`s as argument for `push_str`
fn check_push_str(cx: &LateContext<'_>, expr: &hir::Expr<'_>, receiver: &hir::Expr<'_>, args: &[hir::Expr<'_>]) {
    let mut applicability = Applicability::MachineApplicable;
    if let Some(extension_string) = str_literal_to_char_literal(cx, &args[0], &mut applicability, false) {
        let base_string_snippet =
            snippet_with_applicability(cx, receiver.span.source_callsite(), "..", &mut applicability);
        let sugg = format!("{base_string_snippet}.push({extension_string})");
        span_lint_and_sugg(
            cx,
            SINGLE_CHAR_ADD_STR,
            expr.span,
            "calling `push_str()` using a single-character string literal",
            "consider using `push` with a character literal",
            sugg,
            applicability,
        );
    }

    if let ExprKind::AddrOf(BorrowKind::Ref, _, arg) = &args[0].kind
        && let ExprKind::MethodCall(path_segment, method_arg, [], _) = &arg.kind
        && path_segment.ident.name == sym::to_string
        && (is_ref_char(cx, method_arg) || is_char(cx, method_arg))
    {
        let base_string_snippet =
            snippet_with_applicability(cx, receiver.span.source_callsite(), "..", &mut applicability);
        let extension_string =
            snippet_with_applicability(cx, method_arg.span.source_callsite(), "..", &mut applicability);
        let deref_string = if is_ref_char(cx, method_arg) { "*" } else { "" };

        let sugg = format!("{base_string_snippet}.push({deref_string}{extension_string})");
        span_lint_and_sugg(
            cx,
            SINGLE_CHAR_ADD_STR,
            expr.span,
            "calling `push_str()` using a single-character converted to string",
            "consider using `push` without `to_string()`",
            sugg,
            applicability,
        );
    }
}

/// lint for length-1 `str`s as argument for `insert_str`
fn check_insert_str(cx: &LateContext<'_>, expr: &hir::Expr<'_>, receiver: &hir::Expr<'_>, args: &[hir::Expr<'_>]) {
    let mut applicability = Applicability::MachineApplicable;
    if let Some(extension_string) = str_literal_to_char_literal(cx, &args[1], &mut applicability, false) {
        let base_string_snippet =
            snippet_with_applicability(cx, receiver.span.source_callsite(), "_", &mut applicability);
        let pos_arg = snippet_with_applicability(cx, args[0].span, "..", &mut applicability);
        let sugg = format!("{base_string_snippet}.insert({pos_arg}, {extension_string})");
        span_lint_and_sugg(
            cx,
            SINGLE_CHAR_ADD_STR,
            expr.span,
            "calling `insert_str()` using a single-character string literal",
            "consider using `insert` with a character literal",
            sugg,
            applicability,
        );
    }

    if let ExprKind::AddrOf(BorrowKind::Ref, _, arg) = &args[1].kind
        && let ExprKind::MethodCall(path_segment, method_arg, [], _) = &arg.kind
        && path_segment.ident.name == sym::to_string
        && (is_ref_char(cx, method_arg) || is_char(cx, method_arg))
    {
        let base_string_snippet =
            snippet_with_applicability(cx, receiver.span.source_callsite(), "..", &mut applicability);
        let extension_string =
            snippet_with_applicability(cx, method_arg.span.source_callsite(), "..", &mut applicability);
        let pos_arg = snippet_with_applicability(cx, args[0].span, "..", &mut applicability);
        let deref_string = if is_ref_char(cx, method_arg) { "*" } else { "" };

        let sugg = format!("{base_string_snippet}.insert({pos_arg}, {deref_string}{extension_string})");
        span_lint_and_sugg(
            cx,
            SINGLE_CHAR_ADD_STR,
            expr.span,
            "calling `insert_str()` using a single-character converted to string",
            "consider using `insert` without `to_string()`",
            sugg,
            applicability,
        );
    }
}

fn is_ref_char(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> bool {
    if cx.typeck_results().expr_ty(expr).is_ref()
        && let rustc_middle::ty::Ref(_, ty, _) = cx.typeck_results().expr_ty(expr).kind()
        && ty.is_char()
    {
        return true;
    }

    false
}

fn is_char(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> bool {
    cx.typeck_results().expr_ty(expr).is_char()
}
