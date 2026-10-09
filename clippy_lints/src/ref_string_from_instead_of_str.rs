use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sym;
use rustc_ast::LitKind;
use rustc_hir::def::Res;
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability};
use rustc_lint::{Applicability, LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty::{self, Ty};
use rustc_span::Symbol;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for unnecesary String allocations when calling a function or method that accepts a
    /// &str instead.
    ///
    /// ### Why is this bad?
    /// Allocating a String when it is not needed undermines peformance by allocating heap memory
    /// and executing more instructions than needed.
    ///
    /// ### Example
    /// ```no_run
    /// let mut s = String::from("hello");
    /// s.push_str(&" world".to_string());
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut s = String::from("hello");
    /// s.push_str(" world");
    /// ```
    #[clippy::version = "1.100.0"]
    pub REF_STRING_FROM_INSTEAD_OF_STR,
    perf,
    "use `&str` instead of allocating a new `String`"
}

declare_lint_pass!(RefTringFromInsteadOfStr => [REF_STRING_FROM_INSTEAD_OF_STR]);

impl LateLintPass<'_> for RefTringFromInsteadOfStr {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let typeck = cx.typeck_results();

        let (args, param_types, body_owner): (&[Expr<'_>], &[Ty<'_>], _) = match expr.kind {
            ExprKind::MethodCall(_, _, args, _) if let Some(def_id) = typeck.type_dependent_def_id(expr.hir_id) => (
                args,
                &cx.tcx
                    .fn_sig(def_id)
                    .instantiate(cx.tcx, typeck.node_args(expr.hir_id))
                    .skip_binder()
                    .inputs()[1..],
                cx.tcx.hir_enclosing_body_owner(expr.hir_id),
            ),
            ExprKind::Call(func, args)
                if let Some(ty) = typeck.node_type_opt(func.hir_id)
                    && let ty::FnDef(def_id, sig_args) = ty.kind()
                    && let Some(sig_args) = sig_args.no_bound_vars() =>
            {
                (
                    args,
                    cx.tcx
                        .fn_sig(*def_id)
                        .instantiate(cx.tcx, sig_args)
                        .skip_binder()
                        .inputs(),
                    cx.tcx.hir_enclosing_body_owner(func.hir_id),
                )
            },
            _ => return,
        };

        let str_ref = Ty::new_imm_ref(cx.tcx, cx.tcx.lifetimes.re_erased, cx.tcx.types.str_);
        for (arg, param_ty) in args.iter().zip(param_types) {
            if let Some(literal) = get_literal_from_string_alloc(cx, arg)
                && rustc_hir_typeck::can_coerce(cx.tcx, cx.param_env, body_owner, *param_ty, str_ref)
            {
                span_lint_and_sugg(
                    cx,
                    REF_STRING_FROM_INSTEAD_OF_STR,
                    arg.span,
                    "this creates a needless `String` allocation, use `&str` instead",
                    "use",
                    format!(r#""{}""#, literal.as_str()),
                    Applicability::MaybeIncorrect,
                );
            }
        }
    }
}

fn get_literal_from_string_alloc(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Symbol> {
    if let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner) = expr.kind
        && let ExprKind::Call(func, args) = inner.kind
        && let ExprKind::Path(qpath) = func.kind
        && let Some(first_arg) = args.first()
        && let ExprKind::Lit(ref lit) = first_arg.kind
        && let LitKind::Str(s, _) = lit.node
        && let Res::Def(_, def_id) = cx.typeck_results().qpath_res(&qpath, func.hir_id)
        && let Some(trait_) = cx.tcx.trait_of_assoc(def_id)
        && (cx.tcx.is_diagnostic_item(sym::Into, trait_) || cx.tcx.is_diagnostic_item(sym::from_fn, def_id))
    {
        Some(s)
    } else {
        None
    }
}
