//! Checks for needless boolean results of if-else expressions
//!
//! This lint is **warn** by default

use clippy_utils::diagnostics::{span_lint, span_lint_and_sugg};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sugg::{make_eq_false, Sugg};
use clippy_utils::{
    get_parent_node, is_else_clause, is_expn_of, peel_blocks, peel_blocks_with_stmt, span_extract_comment,
};
use clippy_utils::{higher, SpanlessEq};
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Block, Expr, ExprKind, HirId, Node, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint, impl_lint_pass};
use rustc_span::source_map::Spanned;
use rustc_span::Span;

type BoolComparisonBinCb<'any> = fn(Sugg<'any>, Sugg<'any>) -> Sugg<'any>;
type BoolComparisonUnaryCb<'any> = fn(Sugg<'any>) -> Sugg<'any>;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions of the form `if c { true } else {
    /// false }` (or vice versa) and suggests using the condition directly.
    ///
    /// ### Why is this bad?
    /// Redundant code.
    ///
    /// ### Known problems
    /// Maybe false positives: Sometimes, the two branches are
    /// painstakingly documented (which we, of course, do not detect), so they *may*
    /// have some value. Even then, the documentation can be rewritten to match the
    /// shorter code.
    ///
    /// ### Example
    /// ```rust
    /// # let x = true;
    /// if x {
    ///     false
    /// } else {
    ///     true
    /// }
    /// # ;
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// # let x = true;
    /// !x
    /// # ;
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub NEEDLESS_BOOL,
    complexity,
    "if-statements with plain booleans in the then- and else-clause, e.g., `if p { true } else { false }`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions of the form `x == true`,
    /// `x != true` and order comparisons such as `x < true` (or vice versa) and
    /// suggest using the variable directly.
    ///
    /// ### Why is this bad?
    /// Unnecessary code.
    ///
    /// ### Example
    /// ```rust,ignore
    /// if x == true {}
    /// if y == false {}
    /// ```
    /// use `x` directly:
    /// ```rust,ignore
    /// if x {}
    /// if !y {}
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub BOOL_COMPARISON,
    complexity,
    "comparing a variable to a boolean, e.g., `if x == true` or `if x != true`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions of the form `if c { x = true } else { x = false }`
    /// (or vice versa) and suggest assigning the variable directly from the
    /// condition.
    ///
    /// ### Why is this bad?
    /// Redundant code.
    ///
    /// ### Example
    /// ```rust,ignore
    /// # fn must_keep(x: i32, y: i32) -> bool { x == y }
    /// # let x = 32; let y = 10;
    /// # let mut skip: bool;
    /// if must_keep(x, y) {
    ///     skip = false;
    /// } else {
    ///     skip = true;
    /// }
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// # fn must_keep(x: i32, y: i32) -> bool { x == y }
    /// # let x = 32; let y = 10;
    /// # let mut skip: bool;
    /// skip = !must_keep(x, y);
    /// ```
    #[clippy::version = "1.69.0"]
    pub NEEDLESS_BOOL_ASSIGN,
    complexity,
    "setting the same boolean variable in both branches of an if-statement"
}
declare_lint_pass!(NeedlessBool => [NEEDLESS_BOOL, NEEDLESS_BOOL_ASSIGN]);

fn condition_needs_parentheses(e: &Expr<'_>) -> bool {
    let mut inner = e;
    while let ExprKind::Binary(_, i, _)
    | ExprKind::Call(i, _)
    | ExprKind::Cast(i, _)
    | ExprKind::Type(i, _)
    | ExprKind::Index(i, _) = inner.kind
    {
        if matches!(
            i.kind,
            ExprKind::Block(..)
                | ExprKind::ConstBlock(..)
                | ExprKind::If(..)
                | ExprKind::Loop(..)
                | ExprKind::Match(..)
        ) {
            return true;
        }
        inner = i;
    }
    false
}

fn is_parent_stmt(cx: &LateContext<'_>, id: HirId) -> bool {
    matches!(
        get_parent_node(cx.tcx, id),
        Some(Node::Stmt(..) | Node::Block(Block { stmts: &[], .. }))
    )
}

impl<'tcx> LateLintPass<'tcx> for NeedlessBool {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        use self::Expression::{Bool, RetBool};
        if e.span.from_expansion() || !span_extract_comment(cx.tcx.sess.source_map(), e.span).is_empty() {
            return;
        }
        if let Some(higher::If {
            cond,
            then,
            r#else: Some(r#else),
        }) = higher::If::hir(e)
        {
            let reduce = |ret, not| {
                let mut applicability = Applicability::MachineApplicable;
                let snip = Sugg::hir_with_applicability(cx, cond, "<predicate>", &mut applicability);
                let mut snip = if not { !snip } else { snip };

                if ret {
                    snip = snip.make_return();
                }

                if is_else_clause(cx.tcx, e) {
                    snip = snip.blockify();
                }

                if condition_needs_parentheses(cond) && is_parent_stmt(cx, e.hir_id) {
                    snip = snip.maybe_par();
                }

                span_lint_and_sugg(
                    cx,
                    NEEDLESS_BOOL,
                    e.span,
                    "this if-then-else expression returns a bool literal",
                    "you can reduce it to",
                    snip.to_string(),
                    applicability,
                );
            };
            if let Some((a, b)) = fetch_bool_block(then).and_then(|a| Some((a, fetch_bool_block(r#else)?))) {
                match (a, b) {
                    (RetBool(true), RetBool(true)) | (Bool(true), Bool(true)) => {
                        span_lint(
                            cx,
                            NEEDLESS_BOOL,
                            e.span,
                            "this if-then-else expression will always return true",
                        );
                    },
                    (RetBool(false), RetBool(false)) | (Bool(false), Bool(false)) => {
                        span_lint(
                            cx,
                            NEEDLESS_BOOL,
                            e.span,
                            "this if-then-else expression will always return false",
                        );
                    },
                    (RetBool(true), RetBool(false)) => reduce(true, false),
                    (Bool(true), Bool(false)) => reduce(false, false),
                    (RetBool(false), RetBool(true)) => reduce(true, true),
                    (Bool(false), Bool(true)) => reduce(false, true),
                    _ => (),
                }
            }
            if let Some((lhs_a, a)) = fetch_assign(then) &&
                let Some((lhs_b, b)) = fetch_assign(r#else) &&
                SpanlessEq::new(cx).eq_expr(lhs_a, lhs_b)
            {
                let mut applicability = Applicability::MachineApplicable;
                let cond = Sugg::hir_with_applicability(cx, cond, "..", &mut applicability);
                let lhs = snippet_with_applicability(cx, lhs_a.span, "..", &mut applicability);
                let sugg = if a == b {
                    format!("{cond}; {lhs} = {a:?};")
                } else {
                    format!("{lhs} = {};", if a { cond } else { !cond })
                };
                span_lint_and_sugg(
                    cx,
                    NEEDLESS_BOOL_ASSIGN,
                    e.span,
                    "this if-then-else expression assigns a bool literal",
                    "you can reduce it to",
                    sugg,
                    applicability
                );
            }
        }
    }
}

#[derive(Debug)]
pub struct BoolComparison {
    bcft: BoolComparisonFalseTy,
}

impl_lint_pass!(BoolComparison => [BOOL_COMPARISON]);

impl BoolComparison {
    pub fn new(bcft: Option<&str>) -> Self {
        Self {
            bcft: bcft.map_or(BoolComparisonFalseTy::ExclamationMark, BoolComparisonFalseTy::from),
        }
    }

    fn manage_bcft_bin<'any, 's>(
        bcft: BoolComparisonFalseTy,
        equal_cb: BoolComparisonBinCb<'any>,
        not_cb: BoolComparisonBinCb<'any>,
    ) -> (BoolComparisonBinCb<'any>, &'s str) {
        let msg = "order comparisons between booleans can be simplified";
        match bcft {
            BoolComparisonFalseTy::Equals => (equal_cb, msg),
            BoolComparisonFalseTy::ExclamationMark => (not_cb, msg),
        }
    }

    fn manage_bcft_single<'any, 's>(
        bcft: BoolComparisonFalseTy,
        equal_s: Option<&'s str>,
        not_s: &'s str,
    ) -> Option<(BoolComparisonUnaryCb<'any>, &'s str)> {
        match bcft {
            BoolComparisonFalseTy::Equals => equal_s.map(|elem| {
                let type_infer: BoolComparisonUnaryCb<'any> = make_eq_false;
                (type_infer, elem)
            }),
            BoolComparisonFalseTy::ExclamationMark => Some((|h| !h, not_s)),
        }
    }
}

macro_rules! base_gt_str {
    () => {
        "greater than checks against false are unnecessary"
    };
}
macro_rules! base_lt_str {
    () => {
        "less than comparison against true can be replaced "
    };
}
macro_rules! base_ne_str {
    () => {
        "inequality checks against true can be replaced "
    };
}

impl<'tcx> LateLintPass<'tcx> for BoolComparison {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if e.span.from_expansion() {
            return;
        }
        match e.kind {
            ExprKind::Binary(Spanned { node, .. }, ..) => {
                let ignore_case = None::<(BoolComparisonUnaryCb<'_>, &str)>;
                let ignore_no_literal = None::<(fn(_, _) -> _, &str)>;
                match node {
                    BinOpKind::Eq => {
                        let true_case = Some((|h| h, "equality checks against true are unnecessary"));
                        let false_case = Self::manage_bcft_single(
                            self.bcft,
                            None,
                            "equality checks against false can be replaced by a negation",
                        );
                        check_comparison(cx, e, true_case, false_case, true_case, false_case, ignore_no_literal);
                    },
                    BinOpKind::Ne => {
                        let true_case = Self::manage_bcft_single(
                            self.bcft,
                            Some(concat!(base_ne_str!(), "with a comparison against `false`")),
                            concat!(base_ne_str!(), "by a negation"),
                        );
                        let false_case = Some((|h| h, "inequality checks against false are unnecessary"));
                        check_comparison(cx, e, true_case, false_case, true_case, false_case, ignore_no_literal);
                    },
                    BinOpKind::Lt => check_comparison(
                        cx,
                        e,
                        ignore_case,
                        Some((|h| h, base_gt_str!())),
                        Self::manage_bcft_single(
                            self.bcft,
                            Some(concat!(base_lt_str!(), "with a comparison against `false`")),
                            concat!(base_lt_str!(), "by a negation"),
                        ),
                        ignore_case,
                        Some(Self::manage_bcft_bin(
                            self.bcft,
                            |l, r| make_eq_false(l).bit_and(&r),
                            |l, r| (!l).bit_and(&r),
                        )),
                    ),
                    BinOpKind::Gt => check_comparison(
                        cx,
                        e,
                        Self::manage_bcft_single(
                            self.bcft,
                            Some(concat!(base_lt_str!(), "with a comparison against `false`")),
                            concat!(base_lt_str!(), "by a negation"),
                        ),
                        ignore_case,
                        ignore_case,
                        Some((|h| h, base_gt_str!())),
                        Some(Self::manage_bcft_bin(
                            self.bcft,
                            |l, r| l.bit_and(&make_eq_false(r)),
                            |l, r| l.bit_and(&(!r)),
                        )),
                    ),
                    _ => {},
                }
            },
            ExprKind::Unary(UnOp::Not, local_expr) => {
                if self.bcft != BoolComparisonFalseTy::Equals {
                    return;
                }
                if !cx.typeck_results().expr_ty(local_expr).is_bool() {
                    return;
                }
                let mut applicability = Applicability::MachineApplicable;
                span_lint_and_sugg(
                    cx,
                    BOOL_COMPARISON,
                    e.span,
                    "negation (`!`) is hard to see and can lead to possible misunderstandings",
                    "try replacing it with a comparison against `false`",
                    format!(
                        "{} == false",
                        snippet_with_applicability(cx, local_expr.span, "..", &mut applicability),
                    ),
                    applicability,
                );
            },
            _ => {},
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BoolComparisonFalseTy {
    Equals,
    ExclamationMark,
}

impl From<&str> for BoolComparisonFalseTy {
    fn from(s: &str) -> Self {
        match s {
            "equals" => Self::Equals,
            "exclamation-mark" => Self::ExclamationMark,
            _ => panic!("Unknown configuration attribute"),
        }
    }
}

struct ExpressionInfoWithSpan {
    one_side_is_unary_not: bool,
    left_span: Span,
    right_span: Span,
}

fn is_unary_not(e: &Expr<'_>) -> (bool, Span) {
    if let ExprKind::Unary(UnOp::Not, operand) = e.kind {
        return (true, operand.span);
    }
    (false, e.span)
}

fn one_side_is_unary_not<'tcx>(left_side: &'tcx Expr<'_>, right_side: &'tcx Expr<'_>) -> ExpressionInfoWithSpan {
    let left = is_unary_not(left_side);
    let right = is_unary_not(right_side);

    ExpressionInfoWithSpan {
        one_side_is_unary_not: left.0 != right.0,
        left_span: left.1,
        right_span: right.1,
    }
}

fn check_comparison<'a, 'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    left_true: Option<(impl FnOnce(Sugg<'a>) -> Sugg<'a>, &str)>,
    left_false: Option<(impl FnOnce(Sugg<'a>) -> Sugg<'a>, &str)>,
    right_true: Option<(impl FnOnce(Sugg<'a>) -> Sugg<'a>, &str)>,
    right_false: Option<(impl FnOnce(Sugg<'a>) -> Sugg<'a>, &str)>,
    no_literal: Option<(impl FnOnce(Sugg<'a>, Sugg<'a>) -> Sugg<'a>, &str)>,
) {
    if let ExprKind::Binary(op, left_side, right_side) = e.kind {
        let (l_ty, r_ty) = (
            cx.typeck_results().expr_ty(left_side),
            cx.typeck_results().expr_ty(right_side),
        );
        if is_expn_of(left_side.span, "cfg").is_some() || is_expn_of(right_side.span, "cfg").is_some() {
            return;
        }
        if l_ty.is_bool() && r_ty.is_bool() {
            let mut applicability = Applicability::MachineApplicable;

            if op.node == BinOpKind::Eq {
                let expression_info = one_side_is_unary_not(left_side, right_side);
                if expression_info.one_side_is_unary_not {
                    span_lint_and_sugg(
                        cx,
                        BOOL_COMPARISON,
                        e.span,
                        "this comparison might be written more concisely",
                        "try simplifying it as shown",
                        format!(
                            "{} != {}",
                            snippet_with_applicability(cx, expression_info.left_span, "..", &mut applicability),
                            snippet_with_applicability(cx, expression_info.right_span, "..", &mut applicability)
                        ),
                        applicability,
                    );
                }
            }

            match (fetch_bool_expr(left_side), fetch_bool_expr(right_side)) {
                (Some(true), None) => left_true.map_or((), |(h, m)| {
                    suggest_bool_comparison(cx, e, right_side, applicability, m, h);
                }),
                (None, Some(true)) => right_true.map_or((), |(h, m)| {
                    suggest_bool_comparison(cx, e, left_side, applicability, m, h);
                }),
                (Some(false), None) => left_false.map_or((), |(h, m)| {
                    suggest_bool_comparison(cx, e, right_side, applicability, m, h);
                }),
                (None, Some(false)) => right_false.map_or((), |(h, m)| {
                    suggest_bool_comparison(cx, e, left_side, applicability, m, h);
                }),
                (None, None) => no_literal.map_or((), |(h, m)| {
                    let left_side = Sugg::hir_with_applicability(cx, left_side, "..", &mut applicability);
                    let right_side = Sugg::hir_with_applicability(cx, right_side, "..", &mut applicability);
                    span_lint_and_sugg(
                        cx,
                        BOOL_COMPARISON,
                        e.span,
                        m,
                        "try simplifying it as shown",
                        h(left_side, right_side).to_string(),
                        applicability,
                    );
                }),
                _ => (),
            }
        }
    }
}

fn suggest_bool_comparison<'a, 'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    expr: &Expr<'_>,
    mut app: Applicability,
    message: &str,
    conv_hint: impl FnOnce(Sugg<'a>) -> Sugg<'a>,
) {
    let hint = Sugg::hir_with_context(cx, expr, e.span.ctxt(), "..", &mut app);
    span_lint_and_sugg(
        cx,
        BOOL_COMPARISON,
        e.span,
        message,
        "try simplifying it as shown",
        conv_hint(hint).to_string(),
        app,
    );
}

enum Expression {
    Bool(bool),
    RetBool(bool),
}

fn fetch_bool_block(expr: &Expr<'_>) -> Option<Expression> {
    match peel_blocks_with_stmt(expr).kind {
        ExprKind::Ret(Some(ret)) => Some(Expression::RetBool(fetch_bool_expr(ret)?)),
        _ => Some(Expression::Bool(fetch_bool_expr(expr)?)),
    }
}

fn fetch_bool_expr(expr: &Expr<'_>) -> Option<bool> {
    if let ExprKind::Lit(lit_ptr) = peel_blocks(expr).kind {
        if let LitKind::Bool(value) = lit_ptr.node {
            return Some(value);
        }
    }
    None
}

fn fetch_assign<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, bool)> {
    if let ExprKind::Assign(lhs, rhs, _) = peel_blocks_with_stmt(expr).kind {
        fetch_bool_expr(rhs).map(|b| (lhs, b))
    } else {
        None
    }
}
