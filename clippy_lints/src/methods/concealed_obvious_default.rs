use rustc_span::Symbol;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::CONCEALED_OBVIOUS_DEFAULT;

pub(super) fn check(cx: &LateContext<'_>, recv: &hir::Expr<'_>, method_name: Symbol, call_span: Span) {
    // Type of the expression which invoked the method
    let recv_ty = cx.typeck_results().expr_ty(recv);

    // Option::<bool>::unwrap_or_default()
    //                 ^^^^^^^^^^^^^^^^^^^
    // if the call comes from expansion, bail
    if call_span.from_expansion() {
        return;
    }

    // Only consider algebraic data types e.g. an `Option`.
    // Their generics are represented by `generic_args`
    if let ty::Adt(adt, generic_args) = recv_ty.kind()
        // `name_of_generic`, is e.g. a `sym::Option`
        && let Some(name_of_generic) = cx.tcx.get_diagnostic_name(adt.did())
        && let Some((message, suggestion)) = CONCEALING_METHODS.into_iter().find(|concealing| {
            name_of_generic == concealing.ty && method_name == concealing.method
        }).and_then(|concealing| {
            let ty = generic_args.type_at(concealing.generic_index);
            extract_obvious_default(cx, ty).map(|(default, ty)| {
                let method = (concealing.fmt_msg)(ty);
                (
                    format!("method {method} conceals the underlying type"),
                    (concealing.fmt_sugg)(default),
                )
            })
        })
    {
        span_lint_and_sugg(
            cx,
            CONCEALED_OBVIOUS_DEFAULT,
            call_span,
            message,
            "write the default type explicitly".to_string(),
            suggestion,
            Applicability::MachineApplicable,
        );
    }
}

/// Method which conceals an underlying type of a generic
struct ConcealingMethod {
    /// Generic which contains the concealing method, e.g. `Option<T>`
    ty: Symbol,
    /// Index of the concealed generic type, e.g. `0` for `Option<T>`
    generic_index: usize,
    /// The concealing method, e.g. `unwrap_or_default`
    method: Symbol,
    /// Format the lint's message
    ///
    /// Receives the concealed type, e.g. for `Option<bool>` receives `bool`
    fmt_msg: fn(&'static str) -> String,
    /// Format the lint's suggestion
    ///
    /// Receives the default of the concealed type, e.g. for `Option<bool>` receives `false`,
    /// as `bool::default()` is `false`
    fmt_sugg: fn(&'static str) -> String,
}

/// List of methods which use `Default` trait under the hood,
/// but they have an alternative non-`Default` method
///
/// For example, there is `Option::unwrap_or_default` which is almost the same
/// as `Option::unwrap_or`, but the type does not have to be provided and the
/// `Default` implementation is used.
const CONCEALING_METHODS: [ConcealingMethod; 4] = [
    ConcealingMethod {
        ty: sym::Result,
        // Result<T, E>
        //        ^ want
        generic_index: 0,
        method: sym::unwrap_or_default,
        fmt_msg: |ty| format!("Result::<{ty}, _>::unwrap_or_default()"),
        fmt_sugg: |val| format!("unwrap_or({val})"),
    },
    ConcealingMethod {
        ty: sym::Option,
        // Option<T>
        //        ^ want
        generic_index: 0,
        method: sym::unwrap_or_default,
        fmt_msg: |ty| format!("Option::<{ty}>::unwrap_or_default()"),
        fmt_sugg: |val| format!("unwrap_or({val})"),
    },
    ConcealingMethod {
        ty: sym::HashMapEntry,
        // Entry<'a, K, V, A = Global>
        //              ^ want
        generic_index: 2,
        method: sym::or_default,
        fmt_msg: |ty| format!("hash_map::Entry::<'_, _, {ty}>::or_default()"),
        fmt_sugg: |val| format!("or_insert({val})"),
    },
    ConcealingMethod {
        ty: sym::BTreeEntry,
        // Entry<'a, K, V, A = Global>
        //              ^ want
        generic_index: 2,
        method: sym::or_default,
        fmt_msg: |ty| format!("btree_map::Entry::<'_, _, {ty}>::or_default()"),
        fmt_sugg: |val| format!("or_insert({val})"),
    },
];

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
fn extract_obvious_default(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<(&'static str, &'static str)> {
    match ty.peel_refs().kind() {
        ty::Int(ty) => Some(("0", ty.name_str())),
        ty::Uint(ty) => Some(("0", ty.name_str())),
        ty::Float(ty) => Some(("0.0", ty.name_str())),
        ty::Char => Some((r"'\0'", "char")),
        ty::Str => Some((r#""""#, "&str")),
        ty::Bool => Some(("false", "bool")),
        ty::Tuple(tys) if tys.is_empty() => Some(("()", "()")),
        ty::Adt(def, _) if cx.tcx.get_diagnostic_name(def.did()) == Some(sym::Option) => Some(("None", "Option<_>")),
        _ => None,
    }
}
