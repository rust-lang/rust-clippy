use crate::clippy_utils::res::MaybeDef as _;
use clippy_utils::diagnostics::span_lint_and_sugg;
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
    #[clippy::version = "1.101.0"]
    pub COERCE_TO_STR,
    perf,
    "use `&str` instead of unnecesary type conversions"
}

declare_lint_pass!(CoerceToStr => [COERCE_TO_STR]);

impl LateLintPass<'_> for CoerceToStr {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let typeck = cx.typeck_results();

        let (args, sig, skip_receiver, body_owner) = match expr.kind {
            ExprKind::MethodCall(_, _, args, _) if let Some(def_id) = typeck.type_dependent_def_id(expr.hir_id) => (
                args,
                cx.tcx.fn_sig(def_id).instantiate_identity().skip_normalization(),
                true,
                cx.tcx.hir_enclosing_body_owner(expr.hir_id),
            ),
            ExprKind::Call(func, args)
                if let Some(ty) = typeck.node_type_opt(func.hir_id)
                    && let ty::FnDef(def_id, _) = ty.kind() =>
            {
                (
                    args,
                    cx.tcx.fn_sig(*def_id).instantiate_identity().skip_normalization(),
                    false,
                    cx.tcx.hir_enclosing_body_owner(func.hir_id),
                )
            },
            _ => return,
        };

        let inputs_output = cx.tcx.instantiate_bound_regions_with_erased(sig.inputs_and_output());
        let param_types = &inputs_output[usize::from(skip_receiver)..inputs_output.len() - 1];

        let str_ref = Ty::new_imm_ref(cx.tcx, cx.tcx.lifetimes.re_erased, cx.tcx.types.str_);
        for (arg, param_ty) in args.iter().zip(param_types) {
            if let Some(literal) = get_literal_from_call_expression(cx, arg)
                && rustc_hir_typeck::can_coerce(cx.tcx, cx.param_env, body_owner, str_ref, *param_ty)
            {
                span_lint_and_sugg(
                    cx,
                    COERCE_TO_STR,
                    arg.span,
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
