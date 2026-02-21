use std::fmt::Display;
use std::str::FromStr;

use clippy_utils::source::{SpanRangeExt as _, str_literal_to_char_literal};
use clippy_utils::{get_parent_expr, sym};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Path, QPath};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::Symbol;

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    lit: Symbol,
    ty: Ty<'_>,
    strlit: &Expr<'_>,
    explicit_type: Option<QPath<'_>>,
) {
    macro_rules! number {
        ($kind:ident, $expr:expr, $msg:expr, [$($subkind:ident => $ty:ident),*$(,)?]$(,)?) => {{
            match $expr {
                // If the right type has been found, return the string to substitute the parsing
                // call with, which will be the literal followed by a suffix if initially the
                // parse call was qualified with the return type. Note that we use a canonical
                // suffix, whereas the parse call might have been qualified with a type alias,
                // because type aliases can't be used as suffixes.
                $(ty::$kind::$subkind =>
                  (
                      try_parse::<$ty>(
                          cx,
                          lit,
                          Some(sym::$ty),
                          explicit_type,
                      ),
                      $msg
                  ),)*
                #[allow(unreachable_patterns)]
                _ => return,
            }
        }};
    }

    if let (Some(mut subst), entity) = match ty.kind() {
        ty::Int(int_ty) => number!(IntTy, int_ty, "a signed integer",
                                     [Isize => isize, I8 => i8, I16 => i16, I32 => i32, I64 => i64, I128 => i128]),
        ty::Uint(uint_ty) => number!(UintTy, uint_ty, "an unsigned integer",
                                        [Usize => usize, U8 => u8, U16 => u16, U32 => u32, U64 => u64, U128 => u128]),
        // FIXME: ignore `f128` for now as they cannot use the default parser or formatter
        ty::Float(float_ty) if is_regular_float(lit.as_str()) => number!(FloatTy, float_ty, "a real number",
                                                                         [F16 => f16, F32 => f32, F64 => f64]),
        ty::Float(_) if explicit_type.is_some() => (
            special_float(cx, lit.as_str(), explicit_type.unwrap()),
            "a real number special value",
        ),
        ty::Bool => (try_parse::<bool>(cx, lit, None, None), "a boolean"),
        ty::Char => {
            let mut app = Applicability::MachineApplicable;
            let literal = str_literal_to_char_literal(cx, strlit, &mut app, false);
            if app != Applicability::MachineApplicable {
                return;
            }
            (literal, "a single character")
        },
        _ => return,
    } {
        let contains_cast = subst.contains(" as ");
        if subst.starts_with('+') {
            subst.remove(0);
        }
        if (contains_cast || subst.starts_with('-'))
            && let Some(parent_expr) = get_parent_expr(cx, expr)
        {
            match parent_expr.kind {
                // Unary negation and cast must be parenthesized if they are receivers of a method call
                ExprKind::MethodCall(_, recv, _, _) if expr.hir_id == recv.hir_id => {
                    subst = format!("({subst})");
                },
                // Cast must be parenthesized if it is the argument of a unary operator
                ExprKind::Unary(_, arg) if contains_cast && expr.hir_id == arg.hir_id => {
                    subst = format!("({subst})");
                },
                _ => {},
            }
        }
        super::maybe_emit_lint(cx, expr, false, entity, subst);
    }
}

fn try_parse<T: FromStr + Display>(
    cx: &LateContext<'_>,
    lit: Symbol,
    suffix: Option<Symbol>,
    explicit_type: Option<QPath<'_>>,
) -> Option<String> {
    lit.as_str().parse::<T>().ok().and_then(|_| match explicit_type {
        Some(QPath::Resolved(
            None,
            Path {
                segments: [segment], ..
            },
        )) if Some(segment.ident.name) == suffix => Some(format!("{lit}_{}", segment.ident.name)),
        Some(qpath) => qpath.span().with_source_text(cx, |ty| format!("{lit} as {ty}")),
        None => Some(format!("{lit}")),
    })
}

fn is_regular_float(lit: &str) -> bool {
    lit.chars()
        .all(|c| c.is_ascii_digit() || matches!(c, 'E' | 'e' | '.' | '+' | '-'))
}

fn special_float(cx: &LateContext<'_>, lit: &str, qpath: QPath<'_>) -> Option<String> {
    match &*lit.to_ascii_lowercase() {
        "inf" | "+inf" => qpath.span().with_source_text(cx, |ty| format!("{ty}::INFINITY")),
        "-inf" => qpath.span().with_source_text(cx, |ty| format!("{ty}::NEG_INFINITY")),
        "nan" | "+nan" => qpath.span().with_source_text(cx, |ty| format!("{ty}::NAN")),
        "-nan" => qpath.span().with_source_text(cx, |ty| format!("-{ty}::NAN")),
        _ => None,
    }
}
