use rustc_span::Symbol;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::{LateContext, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::CONCEALED_OBVIOUS_DEFAULT;

pub(super) fn check(
    cx: &LateContext<'_>,
    recv: &hir::Expr<'_>,
    method_name: Symbol,
    call_span: Span,
    method_args: &[rustc_hir::Expr<'_>],
) {
    // Type of the expression which invoked the method
    let recv_ty = cx.typeck_results().expr_ty(recv);

    // Option::<bool>::unwrap_or_default()
    //                 ^^^^^^^^^^^^^^^^^^^
    // if the call comes from expansion, bail
    if call_span.in_external_macro(cx.sess().source_map()) {
        return;
    }

    // Only consider algebraic data types e.g. an `Option`.
    // Their generics are represented by `generic_args`
    if let ty::Adt(recv_adt, recv_generics) = recv_ty.kind()
        // `name_of_generic`, is e.g. a `sym::Option`
        && let Some(recv_symbol) = cx.tcx.get_diagnostic_name(recv_adt.did())
        && let Some((suggestion, ty, applicability)) = resolve_obvious_default(cx, recv_symbol,
            recv_generics.as_slice(), method_name, method_args)
    {
        let message = format!("method `{ty}` conceals the underlying type");

        span_lint_and_sugg(
            cx,
            CONCEALED_OBVIOUS_DEFAULT,
            call_span,
            message,
            "write the default value explicitly".to_string(),
            suggestion,
            applicability,
        );
    }
}

/// Resolve obvious default on the given method call.
///
/// Returns (suggestion, type, applicability)
fn resolve_obvious_default(
    cx: &LateContext<'_>,
    symbol: Symbol,
    generics: &[ty::GenericArg<'_>],
    method: Symbol,
    args: &[rustc_hir::Expr<'_>],
) -> Option<(String, String, Applicability)> {
    Some(match (symbol, generics, method, args) {
        (sym::Result, [ok, _err], sym::unwrap_or_default, []) => {
            let ObviousDefaultReplacement {
                applicability,
                insert,
                ty,
            } = extract_obvious_default(cx, ok.expect_ty())?;

            (
                format!("unwrap_or({insert})"),
                format!("Result::<{ty}, _>::unwrap_or_default"),
                applicability,
            )
        },
        (sym::Result, [ok, _err], sym::map_or_default, [f]) => {
            let ObviousDefaultReplacement {
                mut applicability,
                insert,
                ty,
            } = extract_obvious_default(cx, ok.expect_ty())?;

            let f = clippy_utils::source::snippet_with_applicability(cx.tcx.sess, f.span, "..", &mut applicability);

            (
                format!("map_or({insert}, {f})"),
                format!("Result::<{ty}, _>::map_or_default"),
                applicability,
            )
        },
        (sym::Option, [some], sym::unwrap_or_default, []) => {
            let ObviousDefaultReplacement {
                applicability,
                insert,
                ty,
            } = extract_obvious_default(cx, some.expect_ty())?;

            (
                format!("unwrap_or({insert})"),
                format!("Option::<{ty}>::unwrap_or_default"),
                applicability,
            )
        },
        (sym::Option, [some], sym::map_or_default, [f]) => {
            let ObviousDefaultReplacement {
                mut applicability,
                insert,
                ty,
            } = extract_obvious_default(cx, some.expect_ty())?;

            let f = clippy_utils::source::snippet_with_applicability(cx.tcx.sess, f.span, "..", &mut applicability);

            (
                format!("map_or({insert}, {f})"),
                format!("Option::<{ty}>::map_or_default"),
                applicability,
            )
        },
        (sym::HashMapEntry, [_lifetime, _key, value, _allocator], sym::or_default, []) => {
            let ObviousDefaultReplacement {
                applicability,
                insert,
                ty,
            } = extract_obvious_default(cx, value.expect_ty())?;

            (
                format!("or_insert({insert})"),
                format!("hash_map::Entry::<'_, _, {ty}>::or_default"),
                applicability,
            )
        },
        (sym::BTreeEntry, [_lifetime, _key, value, _allocator], sym::or_default, []) => {
            let ObviousDefaultReplacement {
                applicability,
                insert,
                ty,
            } = extract_obvious_default(cx, value.expect_ty())?;

            (
                format!("or_insert({insert})"),
                format!("btree_map::Entry::<'_, _, {ty}>::or_default"),
                applicability,
            )
        },
        _ => return None,
    })
}

/// An "obvious default"
struct ObviousDefaultReplacement {
    applicability: Applicability,
    /// What to replace this default with
    insert: &'static str,
    /// Type of this default
    ty: &'static str,
}

/// Get default value of a type with an obvious default.
///
/// # Returns
///
/// If the type has an obvious default:
///
/// - Default for the type
/// - The type as it should be displayed in the lint message
///
/// If the type is not considered to have an obvious default, return `None`.
fn extract_obvious_default(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<ObviousDefaultReplacement> {
    match ty.peel_refs().kind() {
        ty::Int(ty) => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: "0",
            ty: ty.name_str(),
        }),
        ty::Uint(ty) => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: "0",
            ty: ty.name_str(),
        }),
        ty::Float(ty) => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: "0.0",
            ty: ty.name_str(),
        }),
        ty::Char => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: r"'\0'",
            ty: "char",
        }),
        ty::Str => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: r#""""#,
            ty: "&str",
        }),
        ty::Bool => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: "false",
            ty: "bool",
        }),
        ty::Tuple(tys) if tys.is_empty() => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: "()",
            ty: "()",
        }),
        ty::Array(_, len)
            if matches!(len.kind(), ty::ConstKind::Value(value) if
            value.to_leaf().size() == rustc_abi::Size::ZERO) =>
        {
            Some(ObviousDefaultReplacement {
                applicability: Applicability::MachineApplicable,
                insert: "[]",
                ty: "[_; 0]",
            })
        },
        // &[u8]
        ty::Ref(_, ty, _)
            if matches!(ty.peel_refs().kind(), ty::Slice(ty) if matches!(ty.peel_refs().kind(),
            ty::Uint(rustc_ast::UintTy::U8))) =>
        {
            Some(ObviousDefaultReplacement {
                applicability: Applicability::MachineApplicable,
                insert: r#"b"""#,
                ty: "&[u8]",
            })
        },
        // &[_]
        ty::Ref(_, ty, _) if matches!(ty.peel_refs().kind(), ty::Slice(_)) => Some(ObviousDefaultReplacement {
            applicability: Applicability::MachineApplicable,
            insert: "&[]",
            ty: "&[_]",
        }),
        ty::Adt(def, _) if cx.tcx.get_diagnostic_name(def.did()) == Some(sym::Vec) => Some(ObviousDefaultReplacement {
            // `vec![]` might be shadowed
            applicability: Applicability::MaybeIncorrect,
            insert: "vec![]",
            ty: "Vec<_>",
        }),
        ty::Adt(def, _) if cx.tcx.get_diagnostic_name(def.did()) == Some(sym::CStr) => {
            Some(ObviousDefaultReplacement {
                applicability: Applicability::MachineApplicable,
                insert: r#"c"""#,
                ty: "&core::ffi::CStr",
            })
        },
        ty::Adt(def, _) if cx.tcx.get_diagnostic_name(def.did()) == Some(sym::Option) => {
            Some(ObviousDefaultReplacement {
                // `None` might be shadowed
                applicability: Applicability::MaybeIncorrect,
                insert: "None",
                ty: "Option<_>",
            })
        },
        _ => None,
    }
}
