use std::mem;
use std::ops::ControlFlow;

use clippy_utils::comparisons::{Rel, normalize_comparison};
use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::{If, Range};
use clippy_utils::macros::{find_assert_eq_args, first_node_macro_backtrace, root_macro_call};
use clippy_utils::source::{snippet, snippet_with_applicability};
use clippy_utils::visitors::for_each_expr_without_closures;
use clippy_utils::{eq_expr_value, hash_expr};
use rustc_ast::{BinOpKind, LitKind, RangeLimits};
use rustc_data_structures::packed::Pu128;
use rustc_data_structures::unhash::UnindexMap;
use rustc_errors::{Applicability, Diag};
use rustc_hir::{Block, Body, Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::source_map::Spanned;
use rustc_span::{Span, Symbol, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for repeated slice indexing without asserting beforehand that the length
    /// is greater than the largest index used to index into the slice.
    ///
    /// ### Why restrict this?
    /// In the general case where the compiler does not have a lot of information
    /// about the length of a slice, indexing it repeatedly will generate a bounds check
    /// for every single index.
    ///
    /// Asserting that the length of the slice is at least as large as the largest value
    /// to index beforehand gives the compiler enough information to elide the bounds checks,
    /// effectively reducing the number of bounds checks from however many times
    /// the slice was indexed to just one (the assert).
    ///
    /// ### Drawbacks
    /// False positives. It is, in general, very difficult to predict how well
    /// the optimizer will be able to elide bounds checks and it very much depends on
    /// the surrounding code. For example, indexing into the slice yielded by the
    /// [`slice::chunks_exact`](https://doc.rust-lang.org/stable/std/primitive.slice.html#method.chunks_exact)
    /// iterator will likely have all of the bounds checks elided even without an assert
    /// if the `chunk_size` is a constant.
    ///
    /// Asserts are not tracked across function calls. Asserting the length of a slice
    /// in a different function likely gives the optimizer enough information
    /// about the length of a slice, but this lint will not detect that.
    ///
    /// ### Example
    /// ```no_run
    /// fn sum(v: &[u8]) -> u8 {
    ///     // 4 bounds checks
    ///     v[0] + v[1] + v[2] + v[3]
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn sum(v: &[u8]) -> u8 {
    ///     assert!(v.len() > 3);
    ///     // no bounds checks
    ///     v[0] + v[1] + v[2] + v[3]
    /// }
    /// ```
    #[clippy::version = "1.74.0"]
    pub MISSING_ASSERTS_FOR_INDEXING,
    restriction,
    "indexing into a slice multiple times without an `assert`"
}
declare_lint_pass!(MissingAssertsForIndexing => [MISSING_ASSERTS_FOR_INDEXING]);

fn report_lint<F>(cx: &LateContext<'_>, index_spans: Vec<Span>, msg: &'static str, f: F)
where
    F: FnOnce(&mut Diag<'_, ()>),
{
    span_lint_and_then(cx, MISSING_ASSERTS_FOR_INDEXING, index_spans, msg, |diag| {
        f(diag);
        diag.note_once("asserting the length before indexing will elide bounds checks");
    });
}

#[derive(Copy, Clone, Debug)]
enum LengthComparison {
    /// `v.len() < 5`
    LengthLessThanInt,
    /// `5 < v.len()`
    IntLessThanLength,
    /// `v.len() <= 5`
    LengthLessThanOrEqualInt,
    /// `5 <= v.len()`
    IntLessThanOrEqualLength,
    /// `5 == v.len()`
    /// `v.len() == 5`
    LengthEqualInt,
}
#[derive(Copy, Clone, Debug)]
struct EvaluatedIntExpr<'hir> {
    expr: &'hir Expr<'hir>,
    value: usize,
}
#[derive(Copy, Clone, Debug)]
enum AssertionSide<'hir> {
    /// `v.len()` in `v.len() > 5`
    SliceLen {
        /// `v` in `v.len()`
        slice: &'hir Expr<'hir>,
    },
    /// `5` in `v.len() > 5`
    AssertedLen(EvaluatedIntExpr<'hir>),
}
impl<'hir> AssertionSide<'hir> {
    pub fn from_expr(cx: &LateContext<'_>, expr: &'hir Expr<'hir>) -> Option<Self> {
        Self::asserted_len_from_int_lit_expr(expr)
            .or_else(|| Self::slice_len_from_expr(cx, expr))
            .or_else(|| Self::asserted_len_from_possibly_const_expr(cx, expr))
    }
    pub fn slice_len_from_expr(cx: &LateContext<'_>, expr: &'hir Expr<'hir>) -> Option<Self> {
        if let ExprKind::MethodCall(method, recv, [], _) = expr.kind
            // checking method name first rather than receiver's type could improve performance
            && method.ident.name == sym::len
            && cx.typeck_results().expr_ty_adjusted(recv).peel_refs().is_slice()
        {
            Some(Self::SliceLen { slice: recv })
        } else {
            None
        }
    }
    pub fn asserted_len_from_expr(cx: &LateContext<'_>, expr: &'hir Expr<'hir>) -> Option<Self> {
        Self::asserted_len_from_int_lit_expr(expr).or_else(|| Self::asserted_len_from_possibly_const_expr(cx, expr))
    }
    pub fn asserted_len_from_int_lit_expr(expr: &'hir Expr<'hir>) -> Option<Self> {
        if let ExprKind::Lit(Spanned {
            node: LitKind::Int(Pu128(x), _),
            ..
        }) = expr.kind
        {
            Some(Self::AssertedLen(EvaluatedIntExpr {
                expr,
                value: x as usize,
            }))
        } else {
            None
        }
    }
    pub fn asserted_len_from_possibly_const_expr(cx: &LateContext<'_>, expr: &'hir Expr<'hir>) -> Option<Self> {
        if let Some(Constant::Int(x)) = ConstEvalCtxt::new(cx).eval(expr) {
            Some(Self::AssertedLen(EvaluatedIntExpr {
                expr,
                value: x as usize,
            }))
        } else {
            None
        }
    }
}

/// Extracts parts out of a length comparison expression.
///
/// E.g. for `v.len() > 5` this returns `Some((LengthComparison::IntLessThanLength, 5, v.len()))`
fn len_comparison<'hir>(
    cx: &LateContext<'_>,
    bin_op: BinOpKind,
    left_expr: &'hir Expr<'hir>,
    right_expr: &'hir Expr<'hir>,
) -> Option<(LengthComparison, EvaluatedIntExpr<'hir>, &'hir Expr<'hir>)> {
    fn sniff_operands<'hir>(
        cx: &LateContext<'_>,
        left: &'hir Expr<'hir>,
        right: &'hir Expr<'hir>,
    ) -> Option<(AssertionSide<'hir>, AssertionSide<'hir>)> {
        // sniff as cheap as possible
        if let Some(left) = AssertionSide::asserted_len_from_int_lit_expr(left) {
            Some((left, AssertionSide::from_expr(cx, right)?))
        } else if let Some(left) = AssertionSide::slice_len_from_expr(cx, left) {
            Some((left, AssertionSide::asserted_len_from_expr(cx, right)?))
        } else {
            Some((
                AssertionSide::asserted_len_from_possibly_const_expr(cx, left)?,
                AssertionSide::slice_len_from_expr(cx, right)?,
            ))
        }
    }

    type Side<'hir> = AssertionSide<'hir>;

    // normalize comparison, `v.len() > 4` becomes `4 < v.len()`
    // this simplifies the logic a bit
    let (op, left_expr, right_expr) = normalize_comparison(bin_op, left_expr, right_expr)?;

    let (left, right) = sniff_operands(cx, left_expr, right_expr)?;
    let (swapped, asserted_len, slice) = match (left, right) {
        // `A > B` (e.g. `5 > 4`)
        | (Side::AssertedLen(_), Side::AssertedLen(_))
        // `v.len() > w.len()`
        | (Side::SliceLen { .. }, Side::SliceLen { .. }) => return None,
        (Side::AssertedLen(asserted_len), Side::SliceLen { slice }) => {
            (false, asserted_len, slice)
        },
        (Side::SliceLen { slice }, Side::AssertedLen(asserted_len)) => {
            (true, asserted_len, slice)
        },
    };

    match op {
        Rel::Lt => {
            let cmp = if swapped {
                LengthComparison::LengthLessThanInt
            } else {
                LengthComparison::IntLessThanLength
            };
            Some((cmp, asserted_len, slice))
        },
        Rel::Le => {
            let cmp = if swapped {
                LengthComparison::LengthLessThanOrEqualInt
            } else {
                LengthComparison::IntLessThanOrEqualLength
            };
            Some((cmp, asserted_len, slice))
        },
        Rel::Eq => Some((LengthComparison::LengthEqualInt, asserted_len, slice)),
        Rel::Ne => None,
    }
}

/// Attempts to extract parts out of an `assert!`-like expression
/// in the form `assert!(some_slice.len() > 5)`.
///
/// `assert!` has expanded to an if expression at the HIR, so this
/// actually works not just with `assert!` specifically, but anything
/// that has a never type expression in the `then` block (e.g. `panic!`).
fn assert_len_expr<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
) -> Option<(LengthComparison, EvaluatedIntExpr<'hir>, &'hir Expr<'hir>, Symbol)> {
    let ((cmp, asserted_len, slice), macro_call) = if let Some(If { cond, then, .. }) = If::hir(expr)
        && let ExprKind::Unary(UnOp::Not, condition) = &cond.kind
        && let ExprKind::Binary(bin_op, left, right) = &condition.kind
        // check if `then` block has a never type expression
        && let ExprKind::Block(Block { expr: Some(then_expr), .. }, _) = then.kind
        && cx.typeck_results().expr_ty(then_expr).is_never()
    {
        (len_comparison(cx, bin_op.node, left, right)?, sym::assert_macro)
    } else if let Some((macro_call, bin_op)) = first_node_macro_backtrace(cx, expr).find_map(|macro_call| {
        match cx.tcx.get_diagnostic_name(macro_call.def_id) {
            Some(sym::assert_eq_macro) => Some((macro_call, BinOpKind::Eq)),
            Some(sym::assert_ne_macro) => Some((macro_call, BinOpKind::Ne)),
            _ => None,
        }
    }) && let Some((left, right, _)) = find_assert_eq_args(cx, expr, macro_call.expn)
    {
        (
            len_comparison(cx, bin_op, left, right)?,
            root_macro_call(expr.span)
                .and_then(|macro_call| cx.tcx.get_diagnostic_name(macro_call.def_id))
                .unwrap_or(sym::assert_macro),
        )
    } else {
        return None;
    };

    Some((cmp, asserted_len, slice, macro_call))
}

#[derive(Debug)]
enum IndexEntry<'hir> {
    /// `assert!` without any indexing (so far)
    StrayAssert {
        asserted_len: EvaluatedIntExpr<'hir>,
        comparison: LengthComparison,
        assert_span: Span,
        slice: &'hir Expr<'hir>,
        macro_call: Symbol,
    },
    /// `assert!` with indexing
    ///
    /// We also store the highest index to be able to check
    /// if the `assert!` asserts the right length.
    AssertWithIndex {
        highest_index: usize,
        is_first_highest: bool,
        asserted_len: EvaluatedIntExpr<'hir>,
        assert_span: Span,
        slice: &'hir Expr<'hir>,
        indexes: Vec<Span>,
        comparison: LengthComparison,
        macro_call: Symbol,
    },
    /// Indexing without an `assert!`
    IndexWithoutAssert {
        highest_index: usize,
        is_first_highest: bool,
        indexes: Vec<Span>,
        slice: &'hir Expr<'hir>,
    },
}

impl<'hir> IndexEntry<'hir> {
    pub fn slice(&self) -> &'hir Expr<'hir> {
        match self {
            IndexEntry::StrayAssert { slice, .. }
            | IndexEntry::AssertWithIndex { slice, .. }
            | IndexEntry::IndexWithoutAssert { slice, .. } => slice,
        }
    }
}

/// Extracts the upper index of a slice indexing expression.
///
/// E.g. for `5` this returns `Some(5)`, for `..5` this returns `Some(4)`,
/// for `..=5` this returns `Some(5)`
fn upper_index_expr(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<usize> {
    if let ExprKind::Lit(lit) = &expr.kind
        && let LitKind::Int(Pu128(index), _) = lit.node
    {
        Some(index as usize)
    } else if let Some(Range {
        end: Some(end), limits, ..
    }) = Range::hir(cx, expr)
        && let ExprKind::Lit(lit) = &end.kind
        && let LitKind::Int(Pu128(index @ 1..), _) = lit.node
    {
        match limits {
            RangeLimits::HalfOpen => Some(index as usize - 1),
            RangeLimits::Closed => Some(index as usize),
        }
    } else {
        None
    }
}

/// Checks if the expression is an index into a slice and adds it to `indexes`
fn check_index<'hir>(cx: &LateContext<'_>, expr: &'hir Expr<'hir>, map: &mut UnindexMap<u64, Vec<IndexEntry<'hir>>>) {
    if let ExprKind::Index(slice, index_lit, _) = expr.kind
        && cx.typeck_results().expr_ty_adjusted(slice).peel_refs().is_slice()
        && let Some(index) = upper_index_expr(cx, index_lit)
    {
        let hash = hash_expr(cx, slice);

        let indexes = map.entry(hash).or_default();
        let entry = indexes.iter_mut().find(|entry| eq_expr_value(cx, entry.slice(), slice));

        if let Some(entry) = entry {
            match entry {
                IndexEntry::StrayAssert {
                    asserted_len,
                    comparison,
                    assert_span,
                    slice,
                    macro_call,
                } => {
                    if slice.span.lo() > assert_span.lo() {
                        *entry = IndexEntry::AssertWithIndex {
                            highest_index: index,
                            is_first_highest: true,
                            asserted_len: *asserted_len,
                            assert_span: *assert_span,
                            slice,
                            indexes: vec![expr.span],
                            comparison: *comparison,
                            macro_call: *macro_call,
                        };
                    }
                },
                IndexEntry::IndexWithoutAssert {
                    highest_index,
                    indexes,
                    is_first_highest,
                    ..
                }
                | IndexEntry::AssertWithIndex {
                    highest_index,
                    indexes,
                    is_first_highest,
                    ..
                } => {
                    indexes.push(expr.span);
                    if *is_first_highest {
                        (*is_first_highest) = *highest_index >= index;
                    }
                    *highest_index = (*highest_index).max(index);
                },
            }
        } else {
            indexes.push(IndexEntry::IndexWithoutAssert {
                highest_index: index,
                is_first_highest: true,
                indexes: vec![expr.span],
                slice,
            });
        }
    }
}

/// Checks if the expression is an `assert!` expression and adds it to `asserts`
fn check_assert<'hir>(cx: &LateContext<'_>, expr: &'hir Expr<'hir>, map: &mut UnindexMap<u64, Vec<IndexEntry<'hir>>>) {
    if let Some((comparison, asserted_len, slice, macro_call)) = assert_len_expr(cx, expr) {
        let hash = hash_expr(cx, slice);
        let indexes = map.entry(hash).or_default();

        let entry = indexes.iter_mut().find(|entry| eq_expr_value(cx, entry.slice(), slice));

        if let Some(entry) = entry {
            if let IndexEntry::IndexWithoutAssert {
                highest_index,
                is_first_highest,
                indexes,
                slice,
            } = entry
                && expr.span.lo() <= slice.span.lo()
            {
                *entry = IndexEntry::AssertWithIndex {
                    highest_index: *highest_index,
                    indexes: mem::take(indexes),
                    is_first_highest: *is_first_highest,
                    slice,
                    assert_span: expr.span.source_callsite(),
                    comparison,
                    asserted_len,
                    macro_call,
                };
            }
        } else {
            indexes.push(IndexEntry::StrayAssert {
                asserted_len,
                comparison,
                assert_span: expr.span.source_callsite(),
                slice,
                macro_call,
            });
        }
    }
}

/// Inspects indexes and reports lints.
///
/// Called at the end of this lint after all indexing and `assert!` expressions have been collected.
fn report_indexes(cx: &LateContext<'_>, map: UnindexMap<u64, Vec<IndexEntry<'_>>>) {
    for bucket in map.into_values() {
        for entry in bucket {
            match entry {
                IndexEntry::AssertWithIndex {
                    highest_index,
                    is_first_highest,
                    asserted_len,
                    indexes,
                    comparison,
                    assert_span,
                    slice,
                    macro_call,
                } if indexes.len() > 1 && !is_first_highest => {
                    let mut app = Applicability::MachineApplicable;
                    let slice_str = snippet_with_applicability(cx, slice.span, "_", &mut app);
                    // if we have found an `assert!`, let's also check that it's actually right
                    // and if it covers the highest index and if not, suggest the correct length
                    let sugg = match comparison {
                        // `v.len() < 5` and `v.len() <= 5` does nothing in terms of bounds checks.
                        // The user probably meant `v.len() > 5`
                        LengthComparison::LengthLessThanInt | LengthComparison::LengthLessThanOrEqualInt => {
                            Some(format!("assert!({slice_str}.len() > {highest_index})",))
                        },
                        // `5 < v.len()` == `v.len() > 5`
                        LengthComparison::IntLessThanLength if asserted_len.value < highest_index => {
                            Some(format!("assert!({slice_str}.len() > {highest_index})",))
                        },
                        // `5 <= v.len() == `v.len() >= 5`
                        LengthComparison::IntLessThanOrEqualLength if asserted_len.value == highest_index => {
                            let asserted_len_str =
                                snippet_with_applicability(cx, asserted_len.expr.span, "_", &mut app);
                            Some(format!("assert!({slice_str}.len() > {asserted_len_str})",))
                        },
                        LengthComparison::IntLessThanOrEqualLength if asserted_len.value < highest_index => {
                            Some(format!("assert!({slice_str}.len() >= {})", highest_index + 1))
                        },
                        // `highest_index` here is rather a length, so we need to add 1 to it
                        LengthComparison::LengthEqualInt if asserted_len.value < highest_index + 1 => {
                            match macro_call {
                                sym::assert_eq_macro => {
                                    Some(format!("assert_eq!({slice_str}.len(), {})", highest_index + 1))
                                },
                                sym::debug_assert_eq_macro => {
                                    Some(format!("debug_assert_eq!({slice_str}.len(), {})", highest_index + 1))
                                },
                                _ => Some(format!("assert!({slice_str}.len() == {})", highest_index + 1)),
                            }
                        },
                        _ => None,
                    };

                    if let Some(sugg) = sugg {
                        report_lint(
                            cx,
                            indexes,
                            "indexing into a slice multiple times with an `assert` that does not cover the highest index",
                            |diag| {
                                diag.span_suggestion_verbose(
                                    assert_span,
                                    "provide the highest index that is indexed with",
                                    sugg,
                                    app,
                                );
                            },
                        );
                    }
                },
                IndexEntry::IndexWithoutAssert {
                    indexes,
                    highest_index,
                    is_first_highest,
                    slice,
                } if indexes.len() > 1 && !is_first_highest => {
                    // if there was no `assert!` but more than one index, suggest
                    // adding an `assert!` that covers the highest index
                    report_lint(
                        cx,
                        indexes,
                        "indexing into a slice multiple times without an `assert`",
                        |diag| {
                            diag.help(format!(
                                "consider asserting the length before indexing: `assert!({}.len() > {highest_index});`",
                                snippet(cx, slice.span, "..")
                            ));
                        },
                    );
                },
                _ => {},
            }
        }
    }
}

impl LateLintPass<'_> for MissingAssertsForIndexing {
    fn check_body(&mut self, cx: &LateContext<'_>, body: &Body<'_>) {
        let mut map = UnindexMap::default();

        for_each_expr_without_closures(body.value, |expr| {
            check_index(cx, expr, &mut map);
            check_assert(cx, expr, &mut map);
            ControlFlow::<!, ()>::Continue(())
        });

        report_indexes(cx, map);
    }
}
