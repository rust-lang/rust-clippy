use std::fmt::Write as _;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::implements_trait;
use clippy_utils::{is_path_diagnostic_item, sugg};
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{self as hir, Expr, ExprKind, GenericArg, QPath, TyKind};
use rustc_lint::LateContext;
use rustc_middle::ty::GenericParamDefKind;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `from_iter()` function calls on types that implement the `FromIterator`
    /// trait.
    ///
    /// ### Why is this bad?
    /// If it's needed to create a collection from the contents of an iterator, the `Iterator::collect(_)`
    /// method is preferred. However, when it's needed to specify the container type,
    /// `Vec::from_iter(_)` can be more readable than using a turbofish (e.g. `_.collect::<Vec<_>>()`). See
    /// [FromIterator documentation](https://doc.rust-lang.org/std/iter/trait.FromIterator.html)
    ///
    /// ### Example
    /// ```no_run
    /// let five_fives = std::iter::repeat(5).take(5);
    ///
    /// let v = Vec::from_iter(five_fives);
    ///
    /// assert_eq!(v, vec![5, 5, 5, 5, 5]);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let five_fives = std::iter::repeat(5).take(5);
    ///
    /// let v: Vec<i32> = five_fives.collect();
    ///
    /// assert_eq!(v, vec![5, 5, 5, 5, 5]);
    /// ```
    /// but prefer to use
    /// ```no_run
    /// let numbers: Vec<i32> = FromIterator::from_iter(1..=5);
    /// ```
    /// instead of
    /// ```no_run
    /// let numbers = (1..=5).collect::<Vec<_>>();
    /// ```
    #[clippy::version = "1.49.0"]
    pub FROM_ITER_INSTEAD_OF_COLLECT,
    pedantic,
    "use `.collect()` instead of `::from_iter()`"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, args: &[Expr<'_>], func: &Expr<'_>) {
    if is_path_diagnostic_item(cx, func, sym::from_iter_fn)
        && let arg_ty = cx.typeck_results().expr_ty(&args[0])
        && let Some(iter_id) = cx.tcx.get_diagnostic_item(sym::Iterator)
        && implements_trait(cx, arg_ty, iter_id, &[])
    {
        let mut app = Applicability::MaybeIncorrect;
        let turbofish = match func.kind {
            ExprKind::Path(QPath::TypeRelative(hir_ty, _)) => build_full_type(cx, hir_ty, &mut app),
            ExprKind::Path(QPath::Resolved(Some(self_ty), _)) => build_full_type(cx, self_ty, &mut app),
            _ => return,
        };
        let iter_expr = sugg::Sugg::hir(cx, &args[0], "..").maybe_paren();
        let sugg = format!("{iter_expr}.collect::<{turbofish}>()");
        span_lint_and_sugg(
            cx,
            FROM_ITER_INSTEAD_OF_COLLECT,
            expr.span,
            "usage of `FromIterator::from_iter`",
            "use `.collect()` instead of `::from_iter()`",
            sugg,
            app,
        );
    }
}

/// Build a type which can be used in a turbofish syntax from `hir_ty`, either by copying the
/// existing generic arguments with the exception of elided lifetimes, or by inserting placeholders
/// for types and consts without default values.
fn build_full_type(cx: &LateContext<'_>, hir_ty: &hir::Ty<'_>, app: &mut Applicability) -> String {
    if let TyKind::Path(ty_qpath) = hir_ty.kind
        && let QPath::Resolved(None, ty_path) = &ty_qpath
        && let Res::Def(_, ty_did) = ty_path.res
    {
        let mut ty_str = itertools::join(ty_path.segments.iter().map(|s| s.ident), "::");
        let mut first = true;
        let mut append = |arg: &str| {
            write!(&mut ty_str, "{}{arg}", [", ", "<"][usize::from(first)]).unwrap();
            first = false;
        };
        if let Some(args) = ty_path.segments.last().and_then(|segment| segment.args) {
            args.args
                .iter()
                .filter(|arg| !matches!(arg, GenericArg::Lifetime(lt) if lt.is_elided()))
                .for_each(|arg| append(&snippet_with_applicability(cx, arg.span().source_callsite(), "_", app)));
        } else {
            cx.tcx
                .generics_of(ty_did)
                .own_params
                .iter()
                .filter(|param| {
                    matches!(
                        param.kind,
                        GenericParamDefKind::Type { has_default: false, .. }
                            | GenericParamDefKind::Const { has_default: false, .. }
                    )
                })
                .for_each(|_| append("_"));
        }
        ty_str.push_str([">", ""][usize::from(first)]);
        ty_str
    } else {
        snippet_with_applicability(cx, hir_ty.span.source_callsite(), "_", app).into()
    }
}
