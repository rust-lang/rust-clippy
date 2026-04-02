use super::{BIND_INSTEAD_OF_MAP, contains_return};
use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_and_then};
use clippy_utils::peel_blocks;
use clippy_utils::source::{snippet, snippet_with_context};
use clippy_utils::visitors::find_all_ret_expressions;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::def::{CtorKind, CtorOf, DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{LangItem, QPath};
use rustc_lint::LateContext;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

#[derive(Clone, Copy)]
pub(super) struct CheckSite<'a, 'ctx> {
    pub cx: &'a LateContext<'ctx>,
    pub expr: Span,
    pub recv: &'ctx hir::Expr<'ctx>,
    pub method_name: Span,
    pub arg: &'ctx hir::Expr<'ctx>,
}

pub(super) fn check_and_then_some(site: CheckSite<'_, '_>) -> bool {
    check(site, "and_then", LangItem::OptionSome, "map")
}

pub(super) fn check_and_then_ok(site: CheckSite<'_, '_>) -> bool {
    check(site, "and_then", LangItem::ResultOk, "map")
}

pub(super) fn check_or_else_err(site: CheckSite<'_, '_>) -> bool {
    check(site, "or_else", LangItem::ResultErr, "map_err")
}

fn check(
    CheckSite {
        cx,
        expr,
        recv,
        method_name,
        arg,
    }: CheckSite<'_, '_>,
    bad_method_name: &'static str,
    wrapper_variant: LangItem,
    good_method_name: &'static str,
) -> bool {
    if let Some(wrapper_variant) = cx.tcx.lang_items().get(wrapper_variant)
        && let receiver_ty = cx.tcx.parent(wrapper_variant)
        && let Some(adt) = cx.typeck_results().expr_ty(recv).ty_adt_def()
        && adt.did() == receiver_ty
    {
        BindInsteadOfMap {
            receiver_ty,
            bad_method_name,
            wrapper_variant,
            good_method_name,
        }
        .check(cx, expr, recv, method_name, arg)
    } else {
        false
    }
}

struct BindInsteadOfMap {
    receiver_ty: DefId,
    bad_method_name: &'static str,
    wrapper_variant: DefId,
    good_method_name: &'static str,
}

impl BindInsteadOfMap {
    fn lint_msg(&self, tcx: TyCtxt<'_>) -> String {
        format!(
            "using `{}.{}(|x| {}(y))`, which is more succinctly expressed as `{}(|x| y)`",
            tcx.item_name(self.receiver_ty),
            self.bad_method_name,
            tcx.item_name(self.wrapper_variant),
            self.good_method_name,
        )
    }

    // Fast path for when the closure is `|x| Some(..)` / `|x| Ok(..)` / `|x| Err(..)`.
    fn lint_closure_simple(
        &self,
        cx: &LateContext<'_>,
        expr: Span,
        recv: Span,
        closure_body: &hir::Expr<'_>,
        closure_args_span: Span,
    ) -> bool {
        if let hir::ExprKind::Call(callee, [inner_expr]) = closure_body.kind
            && let hir::ExprKind::Path(QPath::Resolved(_, path)) = callee.kind
            && self.is_wrapper_variant_ctor(cx, path.res)
            && !contains_return(inner_expr)
        {
            let msg = self.lint_msg(cx.tcx);
            let mut applicability = Applicability::MachineApplicable;
            let (some_inner_snip, _is_macro_call) =
                snippet_with_context(cx, inner_expr.span, closure_body.span.ctxt(), "_", &mut applicability);

            let closure_args_snip = snippet(cx, closure_args_span, "..");
            let option_snip = snippet(cx, recv, "..");
            let sugg = format!(
                "{option_snip}.{}({closure_args_snip} {some_inner_snip})",
                self.good_method_name
            );
            span_lint_and_sugg(cx, BIND_INSTEAD_OF_MAP, expr, msg, "try", sugg, applicability);
            true
        } else {
            false
        }
    }

    fn lint_closure(&self, cx: &LateContext<'_>, expr: Span, method_name: Span, closure_body: &hir::Expr<'_>) -> bool {
        let mut suggs = Vec::new();

        if find_all_ret_expressions(cx, closure_body, |ret_expr| {
            if !ret_expr.span.from_expansion()
                && let hir::ExprKind::Call(func_path, [arg]) = ret_expr.kind
                && let hir::ExprKind::Path(QPath::Resolved(_, path)) = func_path.kind
                && self.is_wrapper_variant_ctor(cx, path.res)
                && !contains_return(arg)
            {
                suggs.push((ret_expr.span, arg.span.source_callsite()));
                true
            } else {
                false
            }
        }) {
            span_lint_and_then(cx, BIND_INSTEAD_OF_MAP, expr, self.lint_msg(cx.tcx), |diag| {
                diag.multipart_suggestion(
                    format!("use `{}` instead", self.good_method_name),
                    std::iter::once((method_name, self.good_method_name.into()))
                        .chain(
                            suggs
                                .into_iter()
                                .map(|(span1, span2)| (span1, snippet(cx, span2, "_").into())),
                        )
                        .collect(),
                    Applicability::MachineApplicable,
                );
            });
            true
        } else {
            false
        }
    }

    fn check(
        &self,
        cx: &LateContext<'_>,
        expr: Span,
        recv: &hir::Expr<'_>,
        method_name: Span,
        arg: &hir::Expr<'_>,
    ) -> bool {
        match arg.kind {
            hir::ExprKind::Closure(&hir::Closure { body, fn_decl_span, .. }) => {
                let closure_body = peel_blocks(cx.tcx.hir_body(body).value);

                self.lint_closure_simple(cx, expr, recv.span, closure_body, fn_decl_span)
                    || self.lint_closure(cx, expr, method_name, closure_body)
            },
            // `.and_then(Some)` / `.and_then(Ok)` / `.or_else(Err)`, which is no-op
            hir::ExprKind::Path(QPath::Resolved(_, path)) if self.is_wrapper_variant_ctor(cx, path.res) => {
                span_lint_and_sugg(
                    cx,
                    BIND_INSTEAD_OF_MAP,
                    expr,
                    format!(
                        "using `{}.{}({})`, which is a no-op",
                        cx.tcx.item_name(self.receiver_ty),
                        self.bad_method_name,
                        cx.tcx.item_name(self.wrapper_variant),
                    ),
                    "use the expression directly",
                    snippet(cx, recv.span, "..").into(),
                    Applicability::MachineApplicable,
                );
                true
            },
            _ => false,
        }
    }

    fn is_wrapper_variant_ctor(&self, cx: &LateContext<'_>, res: Res) -> bool {
        let Res::Def(DefKind::Ctor(CtorOf::Variant, CtorKind::Fn), id) = res else {
            return false;
        };
        cx.tcx.parent(id) == self.wrapper_variant
    }
}
