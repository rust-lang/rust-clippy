use clippy_utils::diagnostics::{span_lint, span_lint_and_sugg};
use clippy_utils::source::SpanRangeExt;
use clippy_utils::sym;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use std::cmp::Ordering;
use std::fmt;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of the `offset` pointer method with an integer
    /// literal.
    ///
    /// ### Why is this bad?
    /// The `add` and `sub` methods more accurately express the intent.
    ///
    /// ### Example
    /// ```no_run
    /// let vec = vec![b'a', b'b', b'c'];
    /// let ptr = vec.as_ptr();
    ///
    /// unsafe {
    ///     ptr.offset(-8);
    /// }
    /// ```
    ///
    /// Could be written:
    ///
    /// ```no_run
    /// let vec = vec![b'a', b'b', b'c'];
    /// let ptr = vec.as_ptr();
    ///
    /// unsafe {
    ///     ptr.sub(8);
    /// }
    /// ```
    #[clippy::version = "CURRENT_RUSTC_VERSION"]
    pub PTR_OFFSET_BY_LITERAL,
    complexity,
    "unneeded pointer offset"
}

declare_lint_pass!(PtrOffsetByLiteral => [PTR_OFFSET_BY_LITERAL]);

impl<'tcx> LateLintPass<'tcx> for PtrOffsetByLiteral {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // Check if the expressions is a ptr.offset or ptr.wrapping_offset method call
        let Some((receiver_expr, arg_expr, method)) = expr_as_ptr_offset_call(cx, expr) else {
            return;
        };

        // Check if the argument to the method call is a (negated) literal.
        let Some(literal) = expr_as_literal(arg_expr) else {
            return;
        };

        let msg = format!("use of `{method}` with a literal");
        if let Some(sugg) = build_suggestion(cx, method, receiver_expr, literal) {
            span_lint_and_sugg(
                cx,
                PTR_OFFSET_BY_LITERAL,
                expr.span,
                msg,
                "try",
                sugg,
                Applicability::MachineApplicable,
            );
        } else {
            span_lint(cx, PTR_OFFSET_BY_LITERAL, expr.span, msg);
        }
    }
}

fn get_literal_bits<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<u128> {
    let ExprKind::Lit(lit) = expr.kind else {
        return None;
    };

    let LitKind::Int(packed_u128, _) = lit.node else {
        return None;
    };

    Some(packed_u128.get())
}

// If the given expression is a (negated) literal, return its value.
fn expr_as_literal<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<i128> {
    if let Some(literal_bits) = get_literal_bits(expr) {
        // The value must fit in a isize, so we can't have overflow here.
        return Some(literal_bits as i128);
    }

    if let ExprKind::Unary(UnOp::Neg, inner) = expr.kind {
        if let Some(literal_bits) = get_literal_bits(inner) {
            return Some(-1 * literal_bits as i128);
        }
    }

    None
}

// If the given expression is a ptr::offset  or ptr::wrapping_offset method call, return the
// receiver, the arg of the method call, and the method.
fn expr_as_ptr_offset_call<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>, Method)> {
    if let ExprKind::MethodCall(path_segment, arg_0, [arg_1], _) = &expr.kind
        && cx.typeck_results().expr_ty(arg_0).is_raw_ptr()
    {
        if path_segment.ident.name == sym::offset {
            return Some((arg_0, arg_1, Method::Offset));
        }
        if path_segment.ident.name == sym::wrapping_offset {
            return Some((arg_0, arg_1, Method::WrappingOffset));
        }
    }
    None
}

fn build_suggestion(cx: &LateContext<'_>, method: Method, receiver_expr: &Expr<'_>, literal: i128) -> Option<String> {
    let receiver = receiver_expr.span.get_source_text(cx)?;

    let new_method = match Ord::cmp(&literal, &0) {
        Ordering::Greater => match method {
            Method::Offset => "add",
            Method::WrappingOffset => "wrapping_add",
        },
        Ordering::Equal => return Some(format!("{receiver}")),
        Ordering::Less => match method {
            Method::Offset => "sub",
            Method::WrappingOffset => "wrapping_sub",
        },
    };

    let literal = literal.unsigned_abs();
    Some(format!("{receiver}.{new_method}({literal})"))
}

#[derive(Copy, Clone)]
enum Method {
    Offset,
    WrappingOffset,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offset => write!(f, "offset"),
            Self::WrappingOffset => write!(f, "wrapping_offset"),
        }
    }
}
