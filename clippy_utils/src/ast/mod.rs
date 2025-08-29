//! Utilities for manipulating and extracting information from `rustc_ast::ast`.

#![allow(clippy::wildcard_imports, clippy::enum_glob_use)]

use rustc_ast::BinOpKind;

pub mod ident_iter;
pub use ident_iter::IdentIter;

mod spanless;
pub use self::spanless::EqCtxt;

pub fn is_useless_with_eq_exprs(kind: BinOpKind) -> bool {
    use BinOpKind::*;
    matches!(
        kind,
        Sub | Div | Eq | Lt | Le | Gt | Ge | Ne | And | Or | BitXor | BitAnd | BitOr
    )
}

/// Checks if each element in the first slice is contained within the latter as per `eq_fn`.
pub fn unordered_over<X, Y>(left: &[X], right: &[Y], mut eq_fn: impl FnMut(&X, &Y) -> bool) -> bool {
    left.len() == right.len() && left.iter().all(|l| right.iter().any(|r| eq_fn(l, r)))
}

/// Checks if two AST nodes are semantically equivalent. Small syntax differences,
/// spans and node IDs are ignored.
#[inline]
pub fn spanless_eq<T: spanless::AstNode>(l: &T, r: &T) -> bool {
    EqCtxt::default().eq(l, r)
}
