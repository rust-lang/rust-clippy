#![expect(unused)]
use clippy_utils::diagnostics::span_lint;
use clippy_utils::ty::is_c_void;
use clippy_utils::{get_parent_expr, is_hir_ty_cfg_dependant, sym};
use rustc_hir::{Expr, ExprKind, GenericArg};
use rustc_lint::LateContext;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{self, Ty};

use super::CAST_PTR_ALIGNMENT;

pub(super) fn check<'tcx>(_: &LateContext<'tcx>, _: &Expr<'_>, _: Ty<'tcx>, _: Ty<'tcx>) {}

pub(super) fn check_cast_method(_: &LateContext<'_>, _: &Expr<'_>) {}
