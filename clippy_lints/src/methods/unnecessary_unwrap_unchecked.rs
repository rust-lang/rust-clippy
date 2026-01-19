use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeQPath;
use clippy_utils::{is_from_proc_macro, last_path_segment};
use rustc_hir::def::{DefKind, Namespace, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind};
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
                    && let parent = cx.tcx.parent(path.res(cx).def_id())
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
                    && fn_ret_ty(cx, unchecked_def_id) == expected_ret_ty =>
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
                    && let parent = cx.tcx.parent(path.res(cx).def_id())
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
                    && fn_ret_ty(cx, unchecked.def_id) == expected_ret_ty =>
            {
                Some(Self {
                    variant: Variant::Assoc(AssocKind::new(has_self)),
                    ident: unchecked_ident,
                })
            },
            // ... And now the latter ^^
            ExprKind::MethodCall(segment, _, _, _)
                if let Some(def_id) = cx.typeck_results().type_dependent_def_id(recv.hir_id)
                    && let parent = cx.tcx.parent(def_id)
                    // Don't use `parent_impl`. We only want to lint if its first parent is an `Impl`
                    && matches!(cx.tcx.def_kind(parent), DefKind::Impl { .. })
                    && let Some(unchecked_ident) = unchecked_ident(segment.ident)
                    && let Some(unchecked) = cx.tcx.associated_items(parent).find_by_ident_and_namespace(
                        cx.tcx,
                        unchecked_ident,
                        Namespace::ValueNS,
                        parent,
                    )
                    && fn_ret_ty(cx, unchecked.def_id) == expected_ret_ty =>
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

fn fn_ret_ty<'tcx>(cx: &LateContext<'tcx>, def_id: DefId) -> Ty<'tcx> {
    cx.tcx.fn_sig(def_id).skip_binder().output().skip_binder()
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
