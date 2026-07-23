use super::CONST_SIZE_WINDOWS;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::Msrv;
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::{deref_chain, implements_trait};
use clippy_utils::visitors::is_const_evaluatable;
use clippy_utils::{contains_name, msrvs, sym};
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind, ImplItemKind, Item, ItemKind, Node, QPath, TraitFn, TraitItemKind, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::{AssocTag, EarlyBinder, Ty, Unnormalized};
use rustc_span::{Span, Symbol};
use std::ops::Not as _;

const ARRAY_WINDOWS: Symbol = sym::array_windows;
const WINDOWS: Symbol = sym::windows;
const SLICE: Symbol = sym::slice;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Suggestion {
    span_location: SpanLocation,
    code: String,
    destructuring_example: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SpanLocation {
    MethodCall,
    EntireExpression,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DestructuredArray {
    array_snippet: String,
    extent: DestructuringExtent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DestructuringExtent {
    Full,
    Partial,
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    recv: &'tcx Expr<'tcx>,
    size_arg: &'tcx Expr<'tcx>,
    call_span: Span,
    msrv: Msrv,
) {
    // Ignore cases where entire expression originates from a macro.
    if expr.span.from_expansion() {
        return;
    }

    // Ignore Rust versions where `slice::array_windows` has not stabilized
    if !msrv.meets(cx, msrvs::ARRAY_WINDOWS) {
        return;
    }

    if is_const_size_window(cx, expr, size_arg) {
        span_lint_and_then(
            cx,
            CONST_SIZE_WINDOWS,
            call_span,
            format!("using `{SLICE}::{WINDOWS}` with a constant size instead of `{SLICE}::{ARRAY_WINDOWS}`"),
            |diag| {
                if let Some(Suggestion {
                    span_location,
                    code,
                    destructuring_example,
                }) = compute_suggestion(cx, recv, size_arg)
                {
                    let span = match span_location {
                        SpanLocation::MethodCall => call_span,
                        SpanLocation::EntireExpression => expr.span,
                    };
                    diag.span_suggestion(span, "use", code, Applicability::MachineApplicable);

                    if let Some(example) = destructuring_example {
                        diag.note(format!("you may also consider array destructuring: `{example}`"));
                    }
                } else {
                    diag.span_help(call_span, format!("consider using `{SLICE}::{ARRAY_WINDOWS}` here`"));
                }
            },
        );
    }
}

fn is_const_size_window<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>, size_arg: &'tcx Expr<'_>) -> bool {
    let is_slice_method_call = cx
        .ty_based_def(expr)
        .opt_parent(cx)
        .opt_impl_ty(cx)
        .map(EarlyBinder::instantiate_identity)
        .map(Unnormalized::skip_normalization)
        .is_some_and(Ty::is_slice);

    is_slice_method_call && is_const_evaluatable(cx.tcx, cx.typeck_results(), size_arg)
}

fn compute_suggestion<'tcx>(
    cx: &LateContext<'tcx>,
    recv: &'tcx Expr<'tcx>,
    size_arg: &'tcx Expr<'tcx>,
) -> Option<Suggestion> {
    let (recv_snippet, size_arg_snippet) = Option::zip(
        snippet_opt(cx, recv.span.source_callsite()),
        snippet_opt(cx, size_arg.span.source_callsite()),
    )?;

    let size_generic_const_arg = if expr_needs_braces_in_const_arg(size_arg) {
        format!("{{ {size_arg_snippet} }}")
    } else {
        size_arg_snippet
    };

    let is_ucfs_required = is_array_windows_method_call_ambiguous(cx, recv);

    let (span_location, code) = if is_ucfs_required {
        (
            SpanLocation::EntireExpression,
            format!(
                "<[_]>::{ARRAY_WINDOWS}::<{size_generic_const_arg}>({})",
                normalize_expr_to_single_ref(cx, recv)?
            ),
        )
    } else {
        (
            SpanLocation::MethodCall,
            format!("{ARRAY_WINDOWS}::<{size_generic_const_arg}>()"),
        )
    };

    let destructuring_example =
        build_destructured_array(cx, size_arg).and_then(|DestructuredArray { array_snippet, extent }| {
            let iterator_expr_snippet = match extent {
                DestructuringExtent::Full if is_ucfs_required => {
                    format!("<[_]>::{ARRAY_WINDOWS}({})", normalize_expr_to_single_ref(cx, recv)?)
                },
                DestructuringExtent::Full => format!("{recv_snippet}.{ARRAY_WINDOWS}()"),
                DestructuringExtent::Partial if is_ucfs_required => format!(
                    "<[_]>::{ARRAY_WINDOWS}::<{size_generic_const_arg}>({})",
                    normalize_expr_to_single_ref(cx, recv)?
                ),
                DestructuringExtent::Partial => format!("{recv_snippet}.{ARRAY_WINDOWS}::<{size_generic_const_arg}>()"),
            };

            Some(format!("for {array_snippet} in {iterator_expr_snippet}"))
        });

    Some(Suggestion {
        span_location,
        code,
        destructuring_example,
    })
}

fn build_destructured_array(cx: &LateContext<'_>, size_arg: &Expr<'_>) -> Option<DestructuredArray> {
    const MAX_HINTED_DESTRUCTURED_ARRAY_LENGTH: u128 = 5;
    let containing_fn = get_containing_fn(cx, size_arg)?;
    let is_unused_name = |name: &&str| !contains_name(Symbol::intern(name), containing_fn, cx);

    let destructured_array = if let ExprKind::Lit(lit) = size_arg.kind
        && let LitKind::Int(Pu128(length), _) = lit.node
        && length <= MAX_HINTED_DESTRUCTURED_ARRAY_LENGTH
        && let length = length as usize
        && !size_arg.span.from_expansion()
    {
        const EXACT_LEN_POTENTIAL_NAMES: &[&[&str]] = &[&["left", "right"], &["x", "y", "z"]];

        const ANY_LEN_POTENTIAL_NAMES: &[&[&str]] = &[
            &["item_1", "item_2", "item_3", "item_4", "item_5"],
            &["el_1", "el_2", "el_3", "el_4", "el_5"],
            &["x1", "x2", "x3", "x4", "x5"],
        ];

        let comma_separated_variables = EXACT_LEN_POTENTIAL_NAMES
            .iter()
            .filter(|names| names.len() == length)
            .chain(ANY_LEN_POTENTIAL_NAMES.iter().filter(|names| names.len() >= length))
            .map(|names| &names[..length])
            .find(|names| names.iter().all(is_unused_name))?
            .join(", ");

        DestructuredArray {
            array_snippet: format!("[{comma_separated_variables}]"),
            extent: DestructuringExtent::Full,
        }
    } else {
        const POTENTIAL_NAMES: &[&str] = &["item", "element", "el", "a", "x"];
        let name = POTENTIAL_NAMES.iter().find(|name| is_unused_name(name))?;
        DestructuredArray {
            array_snippet: format!("[{name}, ..]"),
            extent: DestructuringExtent::Partial,
        }
    };

    Some(destructured_array)
}

fn expr_needs_braces_in_const_arg(expr: &Expr<'_>) -> bool {
    expr.span.from_expansion()
        || match &expr.kind {
            ExprKind::Lit(_) => false,
            ExprKind::Path(QPath::Resolved(None, path)) => {
                path.segments.len() != 1 || path.segments[0].args.is_none().not()
            },
            _ => true,
        }
}

/// We are overly careful with this check: we allow FP because our goal is never allowing FN.
/// The worst-case scenario of being overly careful is that UCFS is suggested when it isn't strictly
/// necessary.
fn is_array_windows_method_call_ambiguous(cx: &LateContext<'_>, recv: &Expr<'_>) -> bool {
    let typeck_results = cx.typeck_results();
    let candidate_recv_types: Vec<_> = [typeck_results.expr_ty(recv), typeck_results.expr_ty_adjusted(recv)]
        .into_iter()
        .flat_map(|ty| deref_chain(cx, ty))
        .collect();

    let recv_may_implements_trait = |trait_def_id: DefId| {
        candidate_recv_types
            .iter()
            .any(|&ty| implements_trait(cx, ty, trait_def_id, &[]))
    };

    cx.tcx
        .visible_traits()
        .filter(|&trait_def_id| trait_declares_array_windows(cx, trait_def_id))
        .any(recv_may_implements_trait)
}

fn trait_declares_array_windows(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    cx.tcx
        .associated_items(trait_def_id)
        .filter_by_name_unhygienic(ARRAY_WINDOWS)
        .any(|item| item.tag() == AssocTag::Fn)
}

fn get_containing_fn<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>) -> Option<&'tcx Expr<'tcx>> {
    let parent_item = cx.tcx.hir_get_parent_item(expr.hir_id);

    let body_id = match cx.tcx.hir_node_by_def_id(parent_item.def_id) {
        Node::Item(Item {
            kind: ItemKind::Fn { body, .. },
            ..
        })
        | Node::ImplItem(rustc_hir::ImplItem {
            kind: ImplItemKind::Fn(_, body),
            ..
        })
        | Node::TraitItem(rustc_hir::TraitItem {
            kind: TraitItemKind::Fn(_, TraitFn::Provided(body)),
            ..
        }) => Some(*body),
        _ => None,
    };

    Some(cx.tcx.hir_body(body_id?).value)
}

/// Attempt to produce a snippet of type &T by removing/adding `&` and `*` operators.
/// If this is not possible, return `None`.
/// Stops at macro expansion.
///
/// Examples:
/// if `expr` is of type `T` -> `&expr`
/// if `expr` is of type `&T` -> `expr`
/// if `expr` is of type `&&T` -> `*expr`
fn normalize_expr_to_single_ref(cx: &LateContext<'_>, recv: &Expr<'_>) -> Option<String> {
    // Peel & and * until we reach macro expansion or core referent
    let mut referent = recv;
    while !referent.span.from_expansion()
        && let ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(UnOp::Deref, inner) = referent.kind
    {
        referent = inner;
    }

    let referent_snippet = snippet_opt(cx, referent.span.source_callsite())?;
    let snippet = if cx.typeck_results().expr_ty(referent).is_ref() {
        referent_snippet
    } else {
        format!("&{referent_snippet}")
    };

    Some(snippet)
}
