use crate::utils::{
    last_path_segment, numeric_literal::NumericLiteral, qpath_res, snippet_opt, span_lint_and_help, span_lint_and_sugg,
};
use if_chain::if_chain;
use rustc_ast::{LitIntType, LitKind};
use rustc_errors::Applicability;
use rustc_hir::{
    def::{DefKind, Res},
    BinOpKind, BindingAnnotation, Expr, ExprKind, ItemKind, Lit, Node, PatKind, QPath,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;

declare_clippy_lint! {
    /// **What it does:** Checks for use of `^` operator when exponentiation was probably intended.
    /// A caret is commonly an ASCII-compatible/keyboard-accessible way to write down exponentiation in docs,
    /// readmes, and comments, and copying and pasting a formula can inadvertedly introduce this error.
    /// Moreover, `^` means exponentiation in other programming languages.
    ///
    /// **Why is this bad?** This is most probably a mistake.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// // Bad
    /// let a = 2 ^ 16;
    /// let b = 10 ^ 4;
    ///
    /// // Good
    /// let a = 1 << 16;
    /// let b = 10i32.pow(4);
    /// ```
    pub XOR_USED_AS_POW,
    correctness,
    "use of `^` operator when exponentiation was probably intended"
}

declare_lint_pass!(XorUsedAsPow => [XOR_USED_AS_POW]);

impl LateLintPass<'_> for XorUsedAsPow {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        let parent_id = cx.tcx.hir().get_parent_item(expr.hir_id);
        if let Some(Node::Item(parent_item)) = cx.tcx.hir().find(parent_id) {
            if let ItemKind::Enum(_, _) = parent_item.kind {
                return;
            }
        }

        if_chain! {
            if !in_external_macro(cx.sess(), expr.span);
            if let ExprKind::Binary(op, left, right) = &expr.kind;
            if BinOpKind::BitXor == op.node;
            if let ExprKind::Lit(lhs) = &left.kind;
            if let Some((lhs_val, lhs_type)) = unwrap_dec_int_literal(cx, lhs);
            then {
                match &right.kind {
                    ExprKind::Lit(rhs) => {
                        if let Some((rhs_val, _)) = unwrap_dec_int_literal(cx, rhs) {
                            report_with_lit(cx, lhs_val, rhs_val, expr.span);
                        }
                    }
                    ExprKind::Path(qpath) => {
                        match qpath_res(cx, qpath, right.hir_id) {
                            Res::Local(hir_id) => {
                                if_chain! {
                                    let node = cx.tcx.hir().get(hir_id);
                                    if let Node::Binding(pat) = node;
                                    if let PatKind::Binding(bind_ann, ..) = pat.kind;
                                    if !matches!(bind_ann, BindingAnnotation::RefMut |
                                            BindingAnnotation::Mutable);
                                    let parent_node = cx.tcx.hir().get_parent_node(hir_id);
                                    if let Some(Node::Local(parent_let_expr)) = cx.tcx.hir().find(parent_node);
                                    if let Some(init) = parent_let_expr.init;
                                    then {
                                        match init.kind {
                                            // immutable bindings that are initialized with literal
                                            ExprKind::Lit(..) => report_with_ident(cx, lhs_val, qpath, expr.span),
                                            // immutable bindings that are initialized with constant
                                            ExprKind::Path(ref path) => {
                                                let res = qpath_res(cx, path, init.hir_id);
                                                if let Res::Def(DefKind::Const, ..) = res {
                                                    report_with_ident(cx, lhs_val, qpath, expr.span);
                                                }
                                            }
                                            _ => {},
                                        }
                                    }
                                }
                            },
                            // constant
                            Res::Def(DefKind::Const, ..) => report_with_ident(cx, lhs_val, qpath, expr.span),
                            _ => {},
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn unwrap_dec_int_literal(cx: &LateContext<'_>, lit: &Lit) -> Option<(u128, LitIntType)> {
    if_chain! {
        if let LitKind::Int(val, val_type) = lit.node;
        if let Some(snippet) = snippet_opt(cx, lit.span);
        if let Some(decoded) = NumericLiteral::from_lit_kind(&snippet, &lit.node);
        if decoded.is_decimal();
        then {
            return Some((val, val_type));
        }
        else {
            return None;
        }
    }
}

fn report_with_ident(cx: &LateContext<'_>, lhs: u128, rhs: &QPath<'_>, span: Span) {
    match lhs {
        2 => {
            let ident = last_path_segment(rhs).ident.name.to_ident_string();
            report_pow_of_two(cx, format!("1 << {}", ident), span);
        },
        10 => report_pow_of_ten(cx, span),
        _ => {},
    }
}

fn report_with_lit(cx: &LateContext<'_>, lhs: u128, rhs: u128, span: Span) {
    if rhs > 127 {
        return;
    }
    match lhs {
        2 => {
            if rhs == 0 {
                report_pow_of_two(cx, format!("1"), span);
                return;
            }

            let lhs_str = if rhs <= 31 {
                "1_u32"
            } else if rhs <= 63 {
                "1_u64"
            } else {
                "1_u127"
            };

            report_pow_of_two(cx, format!("{} << {}", lhs_str, rhs), span);
        },
        10 => report_pow_of_ten(cx, span),
        _ => {},
    }
}

fn report_pow_of_two(cx: &LateContext<'_>, sugg: String, span: Span) {
    span_lint_and_sugg(
        cx,
        XOR_USED_AS_POW,
        span,
        "it appears you are trying to get a power of two, but `^` is not an exponentiation operator",
        "use a bitshift or constant instead",
        sugg,
        Applicability::MaybeIncorrect,
    )
}

fn report_pow_of_ten(cx: &LateContext<'_>, span: Span) {
    span_lint_and_help(
        cx,
        XOR_USED_AS_POW,
        span,
        "`^` is not an exponentiation operator but appears to have been used as one",
        None,
        "did you mean to use .pow()?",
    )
}
