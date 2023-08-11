use clippy_utils::consts::{constant, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::VecArgs;
use clippy_utils::macros::root_macro_call_first_node;
use clippy_utils::msrvs::{Msrv, ITER_ONCE_AND_EMPTY};
use clippy_utils::source::snippet_opt;
use clippy_utils::{expr_use_ctxt, get_parent_expr, is_lang_item_or_ctor, last_path_segment, path_res, std_or_core};
use rustc_errors::{Applicability, Diagnostic};
use rustc_hir::{Expr, ExprKind, GenericArg, LangItem};
use rustc_lint::{LateContext, Lint, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::ty;
use rustc_span::{sym, Span};

use super::{ITER_ON_EMPTY_COLLECTIONS, ITER_ON_SINGLE_ITEMS};

#[derive(Clone, Copy, Debug)]
enum Variant<'tcx> {
    SomeToOnce,
    NoneToEmpty,
    OneLenToOnce(&'tcx Expr<'tcx>),
    ZeroLenToEmpty,
}

impl<'tcx> Variant<'tcx> {
    fn as_lint(self) -> &'static Lint {
        match self {
            Self::SomeToOnce | Self::OneLenToOnce(_) => ITER_ON_SINGLE_ITEMS,
            Self::NoneToEmpty | Self::ZeroLenToEmpty => ITER_ON_EMPTY_COLLECTIONS,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SomeToOnce => "Some",
            Self::NoneToEmpty => "None",
            Self::OneLenToOnce(_) => "[T; 1]",
            Self::ZeroLenToEmpty => "[T; 0]",
        }
    }

    fn desc(self) -> &'static str {
        match self {
            Self::SomeToOnce | Self::OneLenToOnce(_) => "iterator with only one element",
            Self::NoneToEmpty | Self::ZeroLenToEmpty => "empty iterator",
        }
    }
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, msrv: &Msrv) {
    if !msrv.meets(ITER_ONCE_AND_EMPTY) {
        return;
    }

    let (variant, is_vec, sugg_span) = match expr.kind {
        // `[T; 1]`
        ExprKind::Array([one]) => (Variant::OneLenToOnce(one), false, expr.span),
        // `[T; 0]`
        ExprKind::Array([]) => (Variant::ZeroLenToEmpty, false, expr.span),
        // `Some`
        ExprKind::Call(path, _) if let Some(def_id) = path_res(cx, path).opt_def_id()
            && is_lang_item_or_ctor(cx, def_id, LangItem::OptionSome) =>
        {
            (Variant::SomeToOnce, false, expr.span)
        },
        // `None`
        ExprKind::Path(qpath) if let Some(def_id) = cx.qpath_res(&qpath, expr.hir_id).opt_def_id()
            && is_lang_item_or_ctor(cx, def_id, LangItem::OptionNone) =>
        {
            (Variant::NoneToEmpty, false, expr.span)
        }
        // `vec![...]`
        _ if let Some(mac_call) = root_macro_call_first_node(cx, expr)
            && let Some(vec_args) = VecArgs::hir(cx, expr) =>
        {
            (
                match vec_args {
                    VecArgs::Vec(elems) => match elems {
                        // `vec![...]`
                        [one] => Variant::OneLenToOnce(one),
                        // `vec![]`
                        [] => Variant::ZeroLenToEmpty,
                        _ => return,
                    }
                    VecArgs::Repeat(elem, len)
                    if let Some(Constant::Int(len)) = constant(cx, cx.typeck_results(), len) =>
                    {
                        match len {
                            // `vec![...; 1]`
                            1 => Variant::OneLenToOnce(elem),
                            // `vec![...; 0]`
                            0 => Variant::ZeroLenToEmpty,
                            _ => return,
                        }
                    }
                    VecArgs::Repeat(_, _) => return,
                },
                true,
                mac_call.span.source_callsite(),
            )
        },
        _ => return,
    };

    // `vec![]` must be external
    if !is_vec && in_external_macro(cx.sess(), expr.span) {
        return;
    }

    if let Some(parent) = get_parent_expr(cx, expr)
        && let ExprKind::MethodCall(path, recv, args, _) = parent
            .peel_blocks()
            .peel_borrows()
            .kind
    {
        if recv.hir_id == expr.hir_id
            && matches!(path.ident.name, sym::iter | sym::iter_mut | sym::into_iter)
            && expr_use_ctxt(cx, parent).is_some_and(|ctxt| !ctxt.is_ty_unified)
        {
            emit_lint(cx, expr, Some(path.ident.as_str()), parent.span, variant, |_| {});
        } else if let Some(arg_index) = args.iter().position(|arg| arg.hir_id == expr.hir_id)
            && let Some(def_id) = cx.typeck_results().type_dependent_def_id(parent.hir_id)
            && let arg_ty = cx.tcx.fn_sig(def_id).instantiate_identity().inputs().skip_binder()[arg_index].peel_refs()
            && cx
                .tcx
                .param_env(def_id)
                .caller_bounds()
                .iter()
                .any(|p| {
                    matches!(
                        p.kind().no_bound_vars(),
                        Some(p) if matches!(
                            p,
                            ty::ClauseKind::Trait(trit) if trit.trait_ref.self_ty() == arg_ty
                                && cx.tcx.is_diagnostic_item(sym::Iterator, trit.def_id())
                        ),
                    )
                })
        {
            emit_lint(cx, expr, None, sugg_span, variant, |diag| {
                diag.note("this method is generic over `Iterator`");
            });
        }
    }
}

fn emit_lint<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    method_name: Option<&str>,
    sugg_span: Span,
    variant: Variant<'_>,
    f: impl FnOnce(&mut Diagnostic),
) {
    let Some(std_or_core) = std_or_core(cx) else {
        return;
    };
    let sugg_fn = format!(
        "{}::iter::{}",
        std_or_core,
        match variant {
            Variant::SomeToOnce | Variant::OneLenToOnce(_) => "once",
            Variant::NoneToEmpty | Variant::ZeroLenToEmpty => "empty",
        },
    );
    let ref_prefix = ref_prefix(method_name.unwrap_or_default());
    let Some(turbofish_or_args) = (match variant {
        // `Some<T>(1)` -> `once<T>(1)`
        Variant::SomeToOnce if let ExprKind::Call(path, [arg]) = expr.kind => snippet_opt(cx, arg.span).map(|s| {
            format!(
                "{}({ref_prefix}{s})",
                opt_ty_argument(cx, path).unwrap_or_default()
            )
        }),
        // `None::<T>` -> `empty::<T>()`
        Variant::NoneToEmpty => {
            // Retain generic arguments if they're present
            Some(format!(
                "{}()",
                opt_ty_argument(cx, expr).unwrap_or_default()
            ))
        }
        // `vec![1]` -> `once(1)`
        Variant::OneLenToOnce(one) => {
            snippet_opt(cx, one.span).map(|s| format!("({ref_prefix}{s})"))
        }
        // `vec![]` -> `empty()`
        Variant::ZeroLenToEmpty => Some("()".to_owned()),
        Variant::SomeToOnce => unreachable!("only constructed on `ExprKind::Call`"),
    }) else {
        return;
    };

    span_lint_and_then(
        cx,
        variant.as_lint(),
        expr.span.source_callsite(),
        &format!("usage of `{}` to create an {}", variant.as_str(), variant.desc()),
        |diag| {
            diag.span_suggestion(
                sugg_span,
                format!("use `{sugg_fn}` instead"),
                format!("{sugg_fn}{turbofish_or_args}"),
                Applicability::MaybeIncorrect,
            );
            f(diag);
        },
    );
}

fn ref_prefix(method_name: &str) -> &str {
    match method_name {
        "iter" => "&",
        "iter_mut" => "&mut ",
        _ => "",
    }
}

fn opt_ty_argument<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<String> {
    if let ExprKind::Path(qpath) = expr.kind
        && let Some(args) = last_path_segment(&qpath).args
        && let [GenericArg::Type(ty)] = args.args
    {
        return snippet_opt(cx, ty.span).map(|s| format!("::<{s}>"));
    }

    None
}
