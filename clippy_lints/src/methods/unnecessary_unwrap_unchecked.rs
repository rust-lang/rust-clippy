use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::{MaybeDef, MaybeQPath};
use clippy_utils::ty::{option_arg_ty, same_type_modulo_regions};
use clippy_utils::{is_from_proc_macro, last_path_segment, over, sym};
use rustc_hir::def::{DefKind, Namespace, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Body, Expr, ExprKind, PatKind, Safety};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::symbol::Ident;

use super::UNNECESSARY_UNWRAP_UNCHECKED;

#[derive(Clone, Copy, Debug)]
struct VariantAndIdent {
    variant: Variant,
    ident: Ident,
}

impl<'tcx> VariantAndIdent {
    fn new(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, recv: &Expr<'_>) -> Option<Self> {
        let expected_ret_ty = cx.typeck_results().expr_ty(expr);
        match recv.kind {
            // Construct `Variant::Fn(_)`, if applicable. This is necessary for us to handle
            // functions like `std::str::from_utf8_unchecked`.
            ExprKind::Call(path, _)
                if let ExprKind::Path(qpath) = path.kind
                    && let checked_def_id = path.res(cx).def_id()
                    && let parent = cx.tcx.parent(checked_def_id)
                    // Don't use `parent_module`. We only want to lint if its first parent is a `Mod`,
                    // i.e. if this is a free-standing function
                    && cx.tcx.def_kind(parent) == DefKind::Mod
                    && let children = parent.as_local().map_or_else(
                        || cx.tcx.module_children(parent),
                        // We must use a !query for local modules to prevent an ICE.
                        |parent| cx.tcx.module_children_local(parent),
                    )
                    // Make sure that there are other functions in this module
                    // (otherwise there couldn't be an unchecked version)
                    && children.len() > 1
                    && let Some(unchecked_ident) = unchecked_ident(last_path_segment(&qpath).ident)
                    && let Some(unchecked_def_id) = children.iter().find_map(|child| {
                        if child.ident == unchecked_ident
                            && let Res::Def(DefKind::Fn, def_id) = child.res
                        {
                            Some(def_id)
                        } else {
                            None
                        }
                    })
                    && same_functions_modulo_safety(cx, checked_def_id, unchecked_def_id, expected_ret_ty)
                    && (cx.tcx.visibility(unchecked_def_id)).is_at_least(cx.tcx.visibility(checked_def_id), cx.tcx) =>
            {
                Some(Self {
                    variant: Variant::Fn,
                    ident: unchecked_ident,
                })
            },
            // We unfortunately must handle `A::a(&a)` and `a.a()` separately, this handles the
            // former
            ExprKind::Call(path, _)
                if let ExprKind::Path(qpath) = path.kind
                    && let checked_def_id = path.res(cx).def_id()
                    && let parent = cx.tcx.parent(checked_def_id)
                    // Don't use `parent_impl`. We only want to lint if its first parent is an `Impl`
                    && matches!(cx.tcx.def_kind(parent), DefKind::Impl { .. })
                    && let Some(unchecked_ident) = unchecked_ident(last_path_segment(&qpath).ident)
                    && let Some(unchecked) = cx.tcx.associated_items(parent).find_by_ident_and_namespace(
                        cx.tcx,
                        unchecked_ident,
                        Namespace::ValueNS,
                        parent,
                    )
                    && let ty::AssocKind::Fn { has_self, .. } = unchecked.kind
                    && same_functions_modulo_safety(cx, checked_def_id, unchecked.def_id, expected_ret_ty)
                    && (cx.tcx.visibility(unchecked.def_id)).is_at_least(cx.tcx.visibility(checked_def_id), cx.tcx) =>
            {
                Some(Self {
                    variant: Variant::Assoc(AssocKind::new(has_self)),
                    ident: unchecked_ident,
                })
            },
            // ... And now the latter ^^
            ExprKind::MethodCall(segment, _, _, _)
                if let Some(checked_def_id) = cx.typeck_results().type_dependent_def_id(recv.hir_id)
                    && let parent = cx.tcx.parent(checked_def_id)
                    // Don't use `parent_impl`. We only want to lint if its first parent is an `Impl`
                    && matches!(cx.tcx.def_kind(parent), DefKind::Impl { .. })
                    && let Some(unchecked_ident) = unchecked_ident(segment.ident)
                    && let Some(unchecked) = cx.tcx.associated_items(parent).find_by_ident_and_namespace(
                        cx.tcx,
                        unchecked_ident,
                        Namespace::ValueNS,
                        parent,
                    )
                    && same_functions_modulo_safety(cx, checked_def_id, unchecked.def_id, expected_ret_ty)
                    && (cx.tcx.visibility(unchecked.def_id)).is_at_least(cx.tcx.visibility(checked_def_id), cx.tcx) =>
            {
                Some(Self {
                    variant: Variant::Assoc(AssocKind::Method),
                    ident: unchecked_ident,
                })
            },
            _ => None,
        }
    }

    fn msg(self) -> &'static str {
        // Don't use `format!` instead -- it won't be optimized out.
        match self.variant {
            Variant::Fn => "usage of `unwrap_unchecked` when an `_unchecked` variant of the function exists",
            Variant::Assoc(AssocKind::Fn) => {
                "usage of `unwrap_unchecked` when an `_unchecked` variant of the associated function exists"
            },
            Variant::Assoc(AssocKind::Method) => {
                "usage of `unwrap_unchecked` when an `_unchecked` variant of the method exists"
            },
        }
    }

    fn as_str(self) -> &'static str {
        match self.variant {
            Variant::Fn => "function",
            Variant::Assoc(AssocKind::Fn) => "associated function",
            Variant::Assoc(AssocKind::Method) => "method",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Variant {
    /// Free `fn` in a module
    Fn,
    /// Associated item from an `impl`
    Assoc(AssocKind),
}

fn unchecked_ident(checked_ident: Ident) -> Option<Ident> {
    let checked_ident = checked_ident.to_string();
    // Only add `_unchecked` if it doesn't already end with `_`
    (!checked_ident.ends_with('_')).then(|| Ident::from_str(&(checked_ident + "_unchecked")))
}

fn same_functions_modulo_safety<'tcx>(
    cx: &LateContext<'tcx>,
    checked_def_id: DefId,
    unchecked_def_id: DefId,
    unwrapped_ret_ty: Ty<'tcx>,
) -> bool {
    let hir_body = |def_id: DefId| -> Option<&'tcx Body<'tcx>> { cx.tcx.hir_maybe_body_owned_by(def_id.as_local()?) };
    let fn_sig = |def_id| cx.tcx.fn_sig(def_id).skip_binder().skip_binder();

    #[expect(clippy::items_after_statements)]
    /// Checks whether `ty` is a wrapper type (`Option` or `Result`), and if so, returns the "ok"
    /// variant type
    fn wrapper_arg_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        option_arg_ty(cx, ty).or_else(|| {
            if let ty::Adt(adt, args) = *ty.kind()
                && let [ok, _err] = &**args
                && let Some(ok) = ok.as_type()
                && adt.is_diag_item(cx, sym::Result)
            {
                Some(ok)
            } else {
                None
            }
        })
    }

    if match (hir_body(checked_def_id), hir_body(unchecked_def_id)) {
        // For local functions, we can get the parameter names. In that case, we want to make sure
        // that the latter are equal between the checked and unchecked versions.
        (Some(checked_body), Some(unchecked_body)) => {
            over(checked_body.params, unchecked_body.params, |p1, p2| {
                // We only allow simple params (plain bindings) for now, to stay on the safer side.
                if let PatKind::Binding(bm1, _, ident1, None) = p1.pat.kind
                    && let PatKind::Binding(bm2, _, ident2, None) = p2.pat.kind
                {
                    bm1 == bm2 && ident1 == ident2
                } else {
                    false
                }
            })
        }
        // For non-local functions, parameter names are not accessible. Oh well, we'll let it slip
        (None, None) => true,
        // If only one of the versions is non-local, then something weird happened. Bail just in case
        _ => false,
    }
        // Check that the functions have identical signatures, apart from safety, and return type (see below)
        && let checked_fn_sig @ ty::FnSig {
            inputs_and_output: _,
            c_variadic: checked_c_variadic,
            safety: Safety::Safe,
            abi: checked_abi,
        } = fn_sig(checked_def_id)
        && let unchecked_fn_sig @ ty::FnSig {
            inputs_and_output: _,
            c_variadic: unchecked_c_variadic,
            safety: Safety::Unsafe,
            abi: unchecked_abi,
        } = fn_sig(unchecked_def_id)
        && checked_c_variadic == unchecked_c_variadic
        && checked_abi == unchecked_abi
        // NOTE: the reason we use `same_type_modulo_regions` all over the place here is that
        // the regions of different functions will be distinct, even if they are called the same
        && over(checked_fn_sig.inputs(), unchecked_fn_sig.inputs(), |ty1, ty2| {
            same_type_modulo_regions(*ty1, *ty2)
        })
        // The checked version should return `Option<T>` or `Result<T, E>`,
        // and the unchecked version should return just `T`
        && same_type_modulo_regions(unchecked_fn_sig.output(), unwrapped_ret_ty)
        && wrapper_arg_ty(cx, checked_fn_sig.output())
            .is_some_and(|wrapped_ty| same_type_modulo_regions(wrapped_ty, unwrapped_ret_ty))
    {
        true
    } else {
        false
    }
}

/// This only exists so the help message shows `associated function` or `method`, depending on
/// whether it has a `self` parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AssocKind {
    /// No `self`: `fn new() -> Self`
    Fn,
    /// Has `self`: `fn ty<'tcx>(&self) -> Ty<'tcx>`
    Method,
}

impl AssocKind {
    fn new(fn_has_self_parameter: bool) -> Self {
        if fn_has_self_parameter { Self::Method } else { Self::Fn }
    }
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, recv: &Expr<'_>, span: Span) {
    if !expr.span.from_expansion()
        && let Some(variant) = VariantAndIdent::new(cx, expr, recv)
        && !is_from_proc_macro(cx, expr)
    {
        span_lint_and_help(
            cx,
            UNNECESSARY_UNWRAP_UNCHECKED,
            span,
            variant.msg(),
            None,
            format!(
                "call the {} `{}` instead, and remove the `unwrap_unchecked` call",
                variant.as_str(),
                variant.ident,
            ),
        );
    }
}
