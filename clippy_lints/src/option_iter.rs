use clippy_utils::{
    diagnostics::span_lint_and_then,
    get_parent_expr, is_from_proc_macro, is_lang_item_or_ctor, is_trait_method, last_path_segment,
    msrvs::{Msrv, ITER_ONCE_AND_EMPTY},
    path_res,
    source::snippet_opt,
    std_or_core,
};
use rustc_errors::Applicability;
use rustc_errors::Diagnostic;
use rustc_hir::{Expr, ExprKind, FnRetTy, GenericArg, GenericBound, ItemKind, LangItem, OwnerNode, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{sym, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `Some(x)` and `None` when used as iterators.
    ///
    /// ### Why is this bad?
    /// The `std::iter::once` and `std::iter::empty` functions should be used instead.
    ///
    /// ### Example
    /// ```rust,ignore
    /// Some(x).into_iter();
    /// None::<T>.into_iter();
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// std::iter::once(x);
    /// std::iter::empty::<T>();
    /// ```
    #[clippy::version = "1.72.0"]
    pub OPTION_ITER,
    style,
    "checks for `Some(x).into_iter()` and `None.into_iter()`"
}
impl_lint_pass!(OptionIter => [OPTION_ITER]);

#[derive(Clone)]
pub struct OptionIter {
    msrv: Msrv,
}

impl OptionIter {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

#[derive(Clone, Copy)]
enum Variant {
    SomeToOnce,
    NoneToEmpty,
}

impl Variant {
    fn as_str(self) -> &'static str {
        match self {
            Variant::SomeToOnce => "Some",
            Variant::NoneToEmpty => "None",
        }
    }

    fn desc(self) -> &'static str {
        match self {
            Variant::SomeToOnce => "iterator with only one element",
            Variant::NoneToEmpty => "empty iterator",
        }
    }

    fn sugg_fn(self, cx: &LateContext<'_>) -> Option<String> {
        Some(format!(
            "{}::iter::{}",
            std_or_core(cx)?,
            match self {
                Variant::SomeToOnce => "once",
                Variant::NoneToEmpty => "empty",
            },
        ))
    }

    fn turbofish_or_args_snippet(self, cx: &LateContext<'_>, recv: &Expr<'_>) -> Option<String> {
        match self {
            Variant::SomeToOnce if let ExprKind::Call(_, [arg]) = recv.kind => {
                snippet_opt(cx, arg.span).map(|s| format!("({s})"))
            },
            Variant::NoneToEmpty => {
                if let ExprKind::Path(qpath) = recv.kind
                    && let [GenericArg::Type(ty)] = last_path_segment(&qpath).args?.args
                {
                    snippet_opt(cx, ty.span).map(|s| format!("::<{s}>()"))
                } else {
                    Some(String::new())
                }
            },
            Variant::SomeToOnce => None,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for OptionIter {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if !self.msrv.meets(ITER_ONCE_AND_EMPTY) {
            return;
        }

        // `path_res` won't detect `Call`. Only use this there
        let path = if let ExprKind::Call(path, _) = expr.kind {
            path
        } else {
            expr
        };

        if let Some(def_id) = path_res(cx, path).opt_def_id()
            && let is_some = is_lang_item_or_ctor(cx, def_id, LangItem::OptionSome)
            && let is_none = is_lang_item_or_ctor(cx, def_id, LangItem::OptionNone)
            && (is_some || is_none)
        {
            let variant = match (is_some, is_none) {
                (true, false) => Variant::SomeToOnce,
                (false, true) => Variant::NoneToEmpty,
                _ => unreachable!(),
            };

            if let Some(parent) = get_parent_expr(cx, expr)
                && let ExprKind::MethodCall(path, recv, args, _) = parent
                    .peel_blocks()
                    .peel_borrows()
                    .kind
            {
                if recv.hir_id == expr.hir_id && matches!(path.ident.as_str(), "iter" | "iter_mut" | "into_iter") {
                    emit_lint(cx, expr, Some(parent.span), variant, |_| {});
                } else if args.iter().any(|arg| arg.hir_id == expr.hir_id) && is_trait_method(cx, parent, sym::Iterator)
                {
                    emit_lint(
                        cx,
                        expr,
                        None,
                        variant,
                        |diag| {
                            diag.span_note(path.ident.span, "this method is part of the `Iterator` trait");
                        },
                    );
                }
            // Returning `impl IntoIterator<T>`
            } else if let OwnerNode::Item(item) = cx.tcx.hir().owner(cx.tcx.hir().get_parent_item(expr.hir_id))
                && let ItemKind::Fn(sig, _, body) = item.kind
                && let body = cx.tcx.hir().body(body)
                && let FnRetTy::Return(hir_ret_ty) = sig.decl.output
                && let TyKind::OpaqueDef(item_id, _, _) = hir_ret_ty.kind
                && let Some(into_iterator_trait) = cx.tcx.get_diagnostic_item(sym::IntoIterator)
                && let ItemKind::OpaqueTy(opaque) = cx.tcx.hir().item(item_id).kind
                && opaque
                    .bounds
                    .iter()
                    .filter_map(GenericBound::trait_ref)
                    .any(|trit| trit.trait_def_id().is_some_and(|def_id| def_id == into_iterator_trait))
            {
                let mut should_lint = false;
                if let Some(parent) = get_parent_expr(cx, expr) && let ExprKind::Ret(_) = parent.kind {
                    should_lint = true;
                } else if let ExprKind::Block(block, _) = body.value.kind
                    && let Some(final_block_expr) = block.expr
                    && final_block_expr.hir_id == expr.hir_id
                {
                    should_lint = true;
                }

                if should_lint {
                    emit_lint(
                        cx,
                        expr,
                        None,
                        variant,
                        |diag| {
                            diag.span_note(
                                hir_ret_ty.span,
                                "this function returns a type implementing `IntoIterator`",
                            );
                        },
                    );
                }
            }
        }
    }

    extract_msrv_attr!(LateContext);
}

fn emit_lint<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    sugg_span: Option<Span>,
    variant: Variant,
    f: impl FnOnce(&mut Diagnostic),
) {
    let Some(sugg_fn) = variant.sugg_fn(cx) else {
        return;
    };
    let Some(turbofish_or_args) = variant.turbofish_or_args_snippet(cx, expr) else {
        return;
    };

    if !is_from_proc_macro(cx, expr) {
        span_lint_and_then(
            cx,
            OPTION_ITER,
            expr.span,
            &format!("usage of `{}` to create an {}", variant.as_str(), variant.desc()),
            |diag| {
                diag.span_suggestion(
                    sugg_span.unwrap_or(expr.span),
                    format!("use `{sugg_fn}` instead"),
                    format!("{sugg_fn}{turbofish_or_args}"),
                    Applicability::MachineApplicable,
                );
                f(diag);
            },
        );
    }
}
