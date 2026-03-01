use clippy_config::Conf;
use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::{span_lint, span_lint_and_then};
use clippy_utils::ty::{deref_chain, get_adt_inherent_method};
use clippy_utils::{higher, is_from_proc_macro, is_in_test, sym};
use rustc_ast::ast::RangeLimits;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;

const NOTE: &str = "the suggestion might not be applicable in constant blocks";
const RANGE_HELP_MSG_BOUNDED: &str = "consider using `.get(n..m)` or `.get_mut(n..m)` instead";
const RANGE_HELP_MSG_FROM: &str = "consider using `.get(n..)` or `.get_mut(n..)` instead";
const RANGE_HELP_MSG_TO: &str = "consider using `.get(..n)` or `.get_mut(..n)` instead";

declare_clippy_lint! {
    /// ### What it does
    /// Checks for out of bounds array indexing with a constant
    /// index.
    ///
    /// ### Why is this bad?
    /// This will always panic at runtime.
    ///
    /// ### Example
    /// ```rust,no_run
    /// let x = [1, 2, 3, 4];
    ///
    /// x[9];
    /// &x[2..9];
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let x = [1, 2, 3, 4];
    /// // Index within bounds
    ///
    /// x[0];
    /// x[3];
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub OUT_OF_BOUNDS_INDEXING,
    correctness,
    "out of bounds constant indexing"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of indexing or slicing that may panic at runtime.
    ///
    /// This lint does not report on indexing or slicing operations
    /// that always panic, [out_of_bounds_indexing](#out_of_bounds_indexing) already
    /// handles those cases.
    ///
    /// ### Why restrict this?
    /// To avoid implicit panics from indexing and slicing.
    ///
    /// There are “checked” alternatives which do not panic, and can be used with `unwrap()` to make
    /// an explicit panic when it is desired.
    ///
    /// ### Limitations
    /// This lint does not check for the usage of indexing or slicing on strings. These are covered
    /// by the more specific `string_slice` lint.
    ///
    /// ### Example
    /// ```rust,no_run
    /// // Vector
    /// let x = vec![0, 1, 2, 3];
    ///
    /// x[2];
    /// x[100];
    /// &x[2..100];
    ///
    /// // Array
    /// let y = [0, 1, 2, 3];
    ///
    /// let i = 10; // Could be a runtime value
    /// let j = 20;
    /// &y[i..j];
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let x = vec![0, 1, 2, 3];
    /// x.get(2);
    /// x.get(100);
    /// x.get(2..100);
    ///
    /// # let y = [0, 1, 2, 3];
    /// let i = 10;
    /// let j = 20;
    /// y.get(i..j);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub INDEXING_SLICING,
    restriction,
    "indexing/slicing usage"
}

impl_lint_pass!(IndexingSlicing => [INDEXING_SLICING, OUT_OF_BOUNDS_INDEXING]);

pub struct IndexingSlicing {
    allow_indexing_slicing_in_tests: bool,
    suppress_restriction_lint_in_const: bool,
}

impl IndexingSlicing {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            allow_indexing_slicing_in_tests: conf.allow_indexing_slicing_in_tests,
            suppress_restriction_lint_in_const: conf.suppress_restriction_lint_in_const,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for IndexingSlicing {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Index(array, index, _) = &expr.kind
            && (!self.suppress_restriction_lint_in_const || !cx.tcx.hir_is_inside_const_context(expr.hir_id))
            && let expr_ty = cx.typeck_results().expr_ty(array)
            && let mut deref = deref_chain(cx, expr_ty)
            && deref.any(|l| ty_has_applicable_get_function(cx, l.peel_refs(), expr_ty, index))
            && !is_from_proc_macro(cx, expr)
        {
            let ty = cx.typeck_results().expr_ty(array).peel_refs();
            let allowed_in_tests = self.allow_indexing_slicing_in_tests && is_in_test(cx.tcx, expr.hir_id);

            if check_range(cx, expr, index, ty, allowed_in_tests) {
                return;
            }

            // Catchall non-range index, i.e., [n] or [n << m]
            if let ty::Array(..) = ty.kind() {
                // Index is a const block.
                if let ExprKind::ConstBlock(..) = index.kind {
                    return;
                }
                // Index is a constant uint.
                if let Some(constant) = ConstEvalCtxt::new(cx).eval(index) {
                    // only `usize` index is legal in rust array index
                    // leave other type to rustc
                    if let Constant::Int(off) = constant
                        && off <= usize::MAX as u128
                        && let ty::Uint(utype) = cx.typeck_results().expr_ty(index).kind()
                        && *utype == ty::UintTy::Usize
                        && let ty::Array(_, s) = ty.kind()
                        && let Some(size) = s.try_to_target_usize(cx.tcx)
                    {
                        // get constant offset and check whether it is in bounds
                        let off = usize::try_from(off).unwrap();
                        let size = usize::try_from(size).unwrap();

                        if off >= size {
                            span_lint(cx, OUT_OF_BOUNDS_INDEXING, expr.span, "index is out of bounds");
                        }
                    }
                    // Let rustc's `const_err` lint handle constant `usize` indexing on arrays.
                    return;
                }
            }

            if allowed_in_tests {
                return;
            }

            span_lint_and_then(cx, INDEXING_SLICING, expr.span, "indexing may panic", |diag| {
                diag.help("consider using `.get(n)` or `.get_mut(n)` instead");

                if cx.tcx.hir_is_inside_const_context(expr.hir_id) {
                    diag.note(NOTE);
                }
            });
        }
    }
}

fn check_range<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    index: &'tcx Expr<'_>,
    ty: Ty<'tcx>,
    allowed_in_tests: bool,
) -> bool {
    if let Some(range) = higher::Range::hir(cx, index) {
        // Ranged indexes, i.e., &x[n..m], &x[n..], &x[..n] and &x[..]
        if let ty::Array(_, s) = ty.kind() {
            let size: u128 = if let Some(size) = s.try_to_target_usize(cx.tcx) {
                size.into()
            } else {
                return true;
            };

            let const_range = to_const_range(cx, range, size);

            if let (Some(start), _) = const_range
                && start > size
            {
                span_lint(
                    cx,
                    OUT_OF_BOUNDS_INDEXING,
                    range.start.map_or(expr.span, |start| start.span),
                    "range is out of bounds",
                );
                return true;
            }

            if let (_, Some(end)) = const_range
                && end > size
            {
                span_lint(
                    cx,
                    OUT_OF_BOUNDS_INDEXING,
                    range.end.map_or(expr.span, |end| end.span),
                    "range is out of bounds",
                );
                return true;
            }

            if let (Some(_), Some(_)) = const_range {
                // early return because both start and end are constants
                // and we have proven above that they are in bounds
                return true;
            }
        }

        let help_msg = match (range.start, range.end) {
            (None, Some(_)) => RANGE_HELP_MSG_TO,
            (Some(_), None) => RANGE_HELP_MSG_FROM,
            (Some(_), Some(_)) => RANGE_HELP_MSG_BOUNDED,
            (None, None) => return true, // [..] is ok
        };

        if allowed_in_tests {
            return true;
        }

        span_lint_and_then(cx, INDEXING_SLICING, expr.span, "slicing may panic", |diag| {
            diag.help(help_msg);

            if cx.tcx.hir_is_inside_const_context(expr.hir_id) {
                diag.note(NOTE);
            }
        });

        return true;
    }

    if let Some(help_msg) =
        slice_range_kind(cx, cx.typeck_results().expr_ty(index).peel_refs()).and_then(|kind| match kind {
            LangItem::Range | LangItem::RangeInclusiveStruct => Some(RANGE_HELP_MSG_BOUNDED),
            LangItem::RangeFrom => Some(RANGE_HELP_MSG_FROM),
            LangItem::RangeTo | LangItem::RangeToInclusive => Some(RANGE_HELP_MSG_TO),
            _ => None,
        })
    {
        // Range variable (not a literal), e.g., `let r = 0..5; slice[r]`
        if allowed_in_tests {
            return true;
        }

        span_lint_and_then(cx, INDEXING_SLICING, expr.span, "slicing may panic", |diag| {
            diag.help(help_msg);

            if cx.tcx.hir_is_inside_const_context(expr.hir_id) {
                diag.note(NOTE);
            }
        });

        return true;
    }

    false
}

/// Returns a tuple of options with the start and end (exclusive) values of
/// the range. If the start or end is not constant, None is returned.
fn to_const_range(cx: &LateContext<'_>, range: higher::Range<'_>, array_size: u128) -> (Option<u128>, Option<u128>) {
    let ecx = ConstEvalCtxt::new(cx);
    let s = range.start.map(|expr| ecx.eval(expr));
    let start = match s {
        Some(Some(Constant::Int(x))) => Some(x),
        Some(_) => None,
        None => Some(0),
    };

    let e = range.end.map(|expr| ecx.eval(expr));
    let end = match e {
        Some(Some(Constant::Int(x))) => {
            if range.limits == RangeLimits::Closed {
                Some(x + 1)
            } else {
                Some(x)
            }
        },
        Some(_) => None,
        None => Some(array_size),
    };

    (start, end)
}

/// Checks if the type has an applicable `get` method that accepts the given index type.
fn ty_has_applicable_get_function<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    array_ty: Ty<'tcx>,
    index_expr: &Expr<'_>,
) -> bool {
    let index_ty = cx.typeck_results().expr_ty(index_expr).peel_refs();

    if ty.is_slice() || ty.is_array() {
        return matches!(index_ty.kind(), ty::Uint(ty::UintTy::Usize)) || slice_range_kind(cx, index_ty).is_some();
    }

    // For ADTs, check if `get` method exists and accepts the index type
    if let ty::Adt(_, _) = array_ty.kind()
        && let Some(get_method) = get_adt_inherent_method(cx, ty, sym::get)
        && let fn_sig = cx.tcx.fn_sig(get_method.def_id).skip_binder().skip_binder()
        && let get_output_ty = fn_sig.output()
        && let ty::Adt(def, _) = get_output_ty.kind()
        && cx.tcx.is_diagnostic_item(sym::Option, def.0.did)
        // Check that get's input parameter matches the index type
        && fn_sig.inputs().len() >= 2
    {
        let get_input_ty = fn_sig.inputs()[1].peel_refs();
        // FIXME: ideally this would handle type params and projections properly
        return get_input_ty == index_ty || matches!(get_input_ty.kind(), ty::Param(_) | ty::Alias(_, _));
    }

    false
}

/// Checks if `ty` is a `Range*<usize>` type valid for slice indexing.
fn slice_range_kind(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<LangItem> {
    if let ty::Adt(adt_def, args) = ty.kind() {
        let did = adt_def.did();
        let lang_items = cx.tcx.lang_items();

        if lang_items.get(LangItem::RangeFull) == Some(did) {
            return Some(LangItem::RangeFull);
        }

        let is_usize = args
            .first()
            .and_then(|arg| arg.as_type())
            .is_some_and(|t| matches!(t.kind(), ty::Uint(ty::UintTy::Usize)));

        if !is_usize {
            return None;
        }

        for item in [
            LangItem::Range,
            LangItem::RangeFrom,
            LangItem::RangeTo,
            LangItem::RangeInclusiveStruct,
            LangItem::RangeToInclusive,
        ] {
            if lang_items.get(item) == Some(did) {
                return Some(item);
            }
        }
    }

    None
}
