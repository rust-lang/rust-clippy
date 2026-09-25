use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::sym;
use rustc_ast::LitKind;
use rustc_hir::def::Res;
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability, UnOp};
use rustc_lint::{Applicability, LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty::{self, Ty};
use rustc_span::Symbol;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unnecessary type conversions when calling a function or method that accepts a
    /// &str instead.
    ///
    /// ### Why is this bad?
    /// Performing needless type conversions in some situations undermines performance by allocating
    /// heap memory and executing more instructions than needed. Directly using `&str` is also
    /// easier to comprehend and more idiomatic.
    ///
    /// ### Example
    /// ```no_run
    /// let mut s = String::from("hello");
    /// s.push_str(&String::from(" world"));
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut s = String::from("hello");
    /// s.push_str(" world");
    /// ```
    #[clippy::version = "1.102.0"]
    pub COERCE_TO_STR,
    perf,
    "use `&str` instead of unnecessary type conversions"
}

declare_lint_pass!(CoerceToStr => [COERCE_TO_STR]);

impl LateLintPass<'_> for CoerceToStr {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }

        if let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, _) | ExprKind::Unary(UnOp::Deref, _) = expr.kind {
            let str_ref = Ty::new_imm_ref(cx.tcx, cx.tcx.lifetimes.re_erased, cx.tcx.types.str_);
            let str_ptr = Ty::new_ptr(cx.tcx, cx.tcx.types.str_, Mutability::Not);
            if let Some(literal) = get_literal_from_call_expression(cx, expr)
                && let Some(expr_ty_coerced) = cx.typeck_results().expr_ty_adjusted_opt(expr)
                && (expr_ty_coerced == str_ref || expr_ty_coerced == str_ptr)
            {
                span_lint_and_sugg(
                    cx,
                    COERCE_TO_STR,
                    expr.span,
                    "unnecesary type conversion, use `&str` instead",
                    "use",
                    format!(r#""{}""#, literal.as_str()),
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}

const DIAGNOSTIC_ITEMS: &[Symbol] = &[sym::Cow, sym::Rc, sym::Arc];

fn get_literal_from_call_expression(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Symbol> {
    let mut maybe_call_expr = expr;

    while let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner) | ExprKind::Unary(UnOp::Deref, inner) =
        maybe_call_expr.kind
    {
        maybe_call_expr = inner;
    }

    let (s, def_id, func) = if let ExprKind::Call(func, args) = maybe_call_expr.kind
        && let ExprKind::Path(qpath) = func.kind
        && args.len() == 1
        && let Some(first_arg) = args.first()
        && let ExprKind::Lit(ref lit) = first_arg.kind
        && let LitKind::Str(s, _) = lit.node
        && let Res::Def(_, def_id) = cx.typeck_results().qpath_res(&qpath, func.hir_id)
    {
        (s, def_id, func)
    } else {
        return None;
    };

    if let Some(trait_) = cx.tcx.trait_of_assoc(def_id)
        && (cx.tcx.is_diagnostic_item(sym::Into, trait_) || cx.tcx.is_diagnostic_item(sym::from_fn, def_id))
    {
        Some(s)
    } else if let Some(ty) = cx.typeck_results().node_type_opt(func.hir_id)
        && let ty::FnDef(fn_def_id, sig_args) = ty.kind()
        && let Some(sig_args) = sig_args.no_bound_vars()
    {
        let output_ty = cx
            .tcx
            .fn_sig(*fn_def_id)
            .instantiate(cx.tcx, sig_args)
            .skip_binder()
            .output();

        if output_ty.boxed_ty().is_some() || DIAGNOSTIC_ITEMS.iter().any(|i| output_ty.is_diag_item(cx, *i)) {
            Some(s)
        } else {
            None
        }
    } else {
        None
    }
}
