use clippy_utils::diagnostics::{span_lint_and_note, span_lint_and_then};
use clippy_utils::source::{FileRangeExt, SpanEditCx, SpanExt, StrExt};
use clippy_utils::tokenize_with_text;
use core::mem;
use rustc_ast::{BinOp, BinOpKind, Block, Expr, ExprKind, MethodCall, StmtKind};
use rustc_errors::Applicability;
use rustc_lexer::TokenKind;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::{Span, SpanData};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for possible missing comma in an array. It lints if
    /// an array element is a binary operator expression and it lies on two lines.
    ///
    /// ### Why is this bad?
    /// This could lead to unexpected results.
    ///
    /// ### Example
    /// ```rust,ignore
    /// let a = &[
    ///     -1, -2, -3 // <= no comma here
    ///     -4, -5, -6
    /// ];
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub POSSIBLE_MISSING_COMMA,
    correctness,
    "possible missing comma in array"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for an `if` expression followed by either a block or another `if` that
    /// looks like it should have an `else` between them.
    ///
    /// ### Why is this bad?
    /// This is probably some refactoring remnant, even if the code is correct, it
    /// might look confusing.
    ///
    /// ### Example
    /// ```rust,ignore
    /// if foo {
    /// } { // looks like an `else` is missing here
    /// }
    ///
    /// if foo {
    /// } if bar { // looks like an `else` is missing here
    /// }
    /// ```
    #[clippy::version = "1.91.0"]
    pub POSSIBLE_MISSING_ELSE,
    suspicious,
    "possibly missing `else`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of the non-existent `=*`, `=!` and `=-`
    /// operators.
    ///
    /// ### Why is this bad?
    /// This is either a typo of `*=`, `!=` or `-=` or
    /// confusing.
    ///
    /// ### Example
    /// ```rust,ignore
    /// a =- 42; // confusing, should it be `a -= 42` or `a = -42`?
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub SUSPICIOUS_ASSIGNMENT_FORMATTING,
    suspicious,
    "suspicious formatting of `*=`, `-=` or `!=`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for formatting of `else`. It lints if the `else`
    /// is followed immediately by a newline or the `else` seems to be missing.
    ///
    /// ### Why is this bad?
    /// This is probably some refactoring remnant, even if the
    /// code is correct, it might look confusing.
    ///
    /// ### Example
    /// ```rust,ignore
    /// if foo {
    /// } { // looks like an `else` is missing here
    /// }
    ///
    /// if foo {
    /// } if bar { // looks like an `else` is missing here
    /// }
    ///
    /// if foo {
    /// } else
    ///
    /// { // this is the `else` block of the previous `if`, but should it be?
    /// }
    ///
    /// if foo {
    /// } else
    ///
    /// if bar { // this is the `else` block of the previous `if`, but should it be?
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub SUSPICIOUS_ELSE_FORMATTING,
    suspicious,
    "suspicious formatting of `else`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks the formatting of a unary operator on the right hand side
    /// of a binary operator. It lints if there is no space between the binary and unary operators,
    /// but there is a space between the unary and its operand.
    ///
    /// ### Why is this bad?
    /// This is either a typo in the binary operator or confusing.
    ///
    /// ### Example
    /// ```no_run
    /// # let foo = true;
    /// # let bar = false;
    /// // &&! looks like a different operator
    /// if foo &&! bar {}
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let foo = true;
    /// # let bar = false;
    /// if foo && !bar {}
    /// ```
    #[clippy::version = "1.40.0"]
    pub SUSPICIOUS_UNARY_OP_FORMATTING,
    suspicious,
    "suspicious formatting of unary `-` or `!` on the RHS of a BinOp"
}

declare_lint_pass!(Formatting => [
    POSSIBLE_MISSING_COMMA,
    POSSIBLE_MISSING_ELSE,
    SUSPICIOUS_ASSIGNMENT_FORMATTING,
    SUSPICIOUS_ELSE_FORMATTING,
    SUSPICIOUS_UNARY_OP_FORMATTING,
]);

impl EarlyLintPass for Formatting {
    fn check_block(&mut self, cx: &EarlyContext<'_>, block: &Block) {
        if block.stmts.len() >= 2
            && let mut ccx = CheckFmtCx::new(cx, block.span)
            && !ccx.span.ctxt.in_external_macro(cx.sess().source_map())
        {
            for [s1, s2] in block.stmts.array_windows::<2>() {
                if let (StmtKind::Expr(first), StmtKind::Expr(second) | StmtKind::Semi(second)) = (&s1.kind, &s2.kind) {
                    ccx.check_missing_else(first, second);
                }
            }
        }
    }

    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        match expr.kind {
            ExprKind::If(_, ref then, Some(ref else_)) => check_else(cx, expr, then, else_),
            ExprKind::Assign(_, ref rhs, sp) => check_assign(cx, expr, rhs, sp),
            ExprKind::Binary(ref bin_op, _, ref rhs) => check_un_op(cx, expr, bin_op, rhs),
            ExprKind::Array(ref args)
            | ExprKind::Tup(ref args)
            | ExprKind::Call(_, ref args)
            | ExprKind::MethodCall(box MethodCall { ref args, .. }) => {
                let mut ccx = CheckFmtCx::new(cx, expr.span);
                if !ccx.span.ctxt.in_external_macro(ccx.cx.sess().source_map()) {
                    for e in args {
                        ccx.check_missing_comma(e);
                    }
                }
            },
            ExprKind::Paren(ref child) => {
                let mut ccx = CheckFmtCx::new(cx, expr.span);
                if !ccx.span.ctxt.in_external_macro(ccx.cx.sess().source_map()) {
                    ccx.check_missing_comma(child);
                }
            },
            _ => {},
        }
    }
}

/// Implementation of the `SUSPICIOUS_ASSIGNMENT_FORMATTING` lint.
fn check_assign(cx: &EarlyContext<'_>, assign: &Expr, rhs: &Expr, op_sp: Span) {
    if let ExprKind::Unary(op, _) = rhs.kind
        && let assign_data = assign.span.data()
        && rhs.span.ctxt() == assign_data.ctxt
        && let op_data = op_sp.data()
        && op_data.ctxt == assign_data.ctxt
        && let op_str = op.as_str()
        && let sm = cx.sess().source_map()
        && !assign_data.ctxt.in_external_macro(sm)
        && let Some([lint_sp, sep_sp]) = op_data.map_split_range(sm, |scx, range| {
            let lint_range = range
                .extend_end_to(scx, assign_data.hi_ctxt())?
                .map_range_text(scx, |src| {
                    src.split_multipart_prefix(["=", op_str])
                        .and_then(|[s, rest]| rest.starts_with(char::is_whitespace).then_some(s))
                })?;
            lint_range
                .clone()
                .with_trailing_whitespace(scx)
                .map(|sep_range| [lint_range, sep_range])
        })
    {
        span_lint_and_then(
            cx,
            SUSPICIOUS_ASSIGNMENT_FORMATTING,
            lint_sp,
            "this looks similar to a compound assignment operator",
            |diag| {
                diag.span_suggestion(
                    lint_sp,
                    "reverse the characters",
                    format!("{op_str}="),
                    Applicability::MaybeIncorrect,
                )
                .span_suggestion(
                    sep_sp,
                    "separate the characters",
                    format!("= {op_str}"),
                    Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

/// Implementation of the `SUSPICIOUS_UNARY_OP_FORMATTING` lint.
fn check_un_op(cx: &EarlyContext<'_>, bin_expr: &Expr, bin_op: &BinOp, rhs: &Expr) {
    if let ExprKind::Unary(un_op, _) = rhs.kind
        && let bin_op_data = bin_op.span.data()
        && bin_op_data.ctxt == bin_expr.span.ctxt()
        && let rhs_data = rhs.span.data()
        && rhs_data.ctxt == bin_op_data.ctxt
        && let bin_op_str = bin_op.node.as_str()
        && let un_op_str = un_op.as_str()
        && let sm = cx.sess().source_map()
        && !bin_op_data.ctxt.in_external_macro(sm)
        && let Some([lint_sp, sugg_sp]) = bin_op_data.map_split_range(sm, |scx, range| {
            let lint_range = range
                .extend_end_to(scx, rhs_data.hi_ctxt())?
                .map_range_text(scx, |src| {
                    src.split_multipart_prefix([bin_op_str, un_op_str])
                        .and_then(|[s, rest]| rest.starts_with(char::is_whitespace).then_some(s))
                })?;
            lint_range
                .clone()
                .with_trailing_whitespace(scx)
                .map(|sugg_range| [lint_range, sugg_range])
        })
    {
        span_lint_and_then(
            cx,
            SUSPICIOUS_UNARY_OP_FORMATTING,
            lint_sp,
            "this formatting makes the binary and unary operators look like a single operator",
            |diag| {
                diag.span_suggestion(
                    sugg_sp,
                    "add a space between",
                    format!("{bin_op_str} {un_op_str}"),
                    if bin_op_data.ctxt.is_root() {
                        Applicability::MachineApplicable
                    } else {
                        Applicability::MaybeIncorrect
                    },
                );
            },
        );
    }
}

/// Implementation of the `SUSPICIOUS_ELSE_FORMATTING` lint for weird `else`.
fn check_else(cx: &EarlyContext<'_>, expr: &Expr, then: &Block, else_: &Expr) {
    let then_data = then.span.data();
    if then_data.ctxt == expr.span.ctxt()
        && let else_data = else_.span.data()
        && then_data.ctxt == else_data.ctxt
        && let sm = cx.sess().source_map()
        && !then_data.ctxt.in_external_macro(sm)
        && let is_else_block = matches!(else_.kind, ExprKind::Block(..))
        && let Some(lint_sp) = then_data.map_range(sm, |scx, range| {
            range.get_range_between(scx, else_data).filter(|range| {
                scx.get_text(range.clone())
                    .is_some_and(|src| check_else_formatting(src, is_else_block))
            })
        })
    {
        let else_desc = if is_else_block { "{..}" } else { "if" };
        span_lint_and_note(
            cx,
            SUSPICIOUS_ELSE_FORMATTING,
            lint_sp,
            format!("this is an `else {else_desc}` but the formatting might hide it"),
            None,
            format!(
                "to remove this lint, remove the `else` or remove the new line between \
                 `else` and `{else_desc}`",
            ),
        );
    }
}

fn check_else_formatting(src: &str, is_else_block: bool) -> bool {
    // Check for any of the following:
    // * A blank line between the end of the previous block and the `else`.
    // * A blank line between the `else` and the start of it's block.
    // * A block comment preceding the `else`, `if` or block if it's the first thing on the line.
    // * The `else` and `if` are on separate lines unless separated by multiple lines with every
    //   intervening line containing only block comments. This is due to rustfmt splitting
    //   `else/*comment*/if` into three lines.
    // * The `else` and it's block are on separate lines unless every intervening line containing only
    //   block comments. There must be one such line unless the `else` and the preceding block are on
    //   separate lines.
    let mut tokens = tokenize_with_text(src);
    let mut lf_count = 0;
    let mut skip_lf = false;
    loop {
        match tokens.next() {
            Some((TokenKind::Whitespace, text, _)) => match text.bytes().filter(|&c| c == b'\n').count() {
                0 => {},
                x => lf_count += x - usize::from(mem::replace(&mut skip_lf, false)),
            },
            Some((TokenKind::LineComment { .. }, _, _)) => skip_lf = lf_count != 0,
            Some((TokenKind::BlockComment { .. }, text, _)) => {
                if lf_count == 0 {
                    lf_count = usize::from(text.contains('\n'));
                }
                skip_lf = lf_count != 0;
            },
            Some((TokenKind::Ident, "else", _)) if skip_lf || lf_count > 1 => return true,
            Some((TokenKind::Ident, "else", _)) => break,
            _ => return false,
        }
    }
    let mut allow_lf = is_else_block && lf_count != 0;
    skip_lf = false;
    lf_count = 0;
    for (kind, text, _) in tokens {
        match kind {
            TokenKind::Whitespace => match text.bytes().filter(|&c| c == b'\n').count() {
                0 => {},
                x => lf_count += x - usize::from(mem::replace(&mut skip_lf, false)),
            },
            TokenKind::BlockComment { .. } => {
                skip_lf = lf_count != 0;
                allow_lf |= skip_lf;
            },
            TokenKind::LineComment { .. } => return true,
            _ => return false,
        }
    }
    skip_lf || lf_count > usize::from(allow_lf)
}

struct CheckFmtCx<'cx> {
    cx: &'cx EarlyContext<'cx>,
    // Delay and cache the construction of this. It isn't cheap and it's frequently unused,
    // but it can also be used a large number of times.
    scx: Option<SpanEditCx<'cx>>,
    span: SpanData,
}
impl<'cx> CheckFmtCx<'cx> {
    fn new(cx: &'cx EarlyContext<'cx>, sp: Span) -> Self {
        Self {
            cx,
            scx: None,
            span: sp.data(),
        }
    }

    fn check_missing_comma(&mut self, e: &Expr) {
        if let ExprKind::Binary(op, lhs, rhs) = &e.kind
            && let e_sp = e.span.data()
            && e_sp.ctxt == self.span.ctxt
            && [self.span.lo, e_sp.lo, e_sp.hi, self.span.hi].is_sorted()
        {
            if matches!(
                op.node,
                BinOpKind::And | BinOpKind::Mul | BinOpKind::Sub | BinOpKind::BitAnd
            ) && let op_sp = op.span.data()
                && op_sp.ctxt == e_sp.ctxt
                && [e_sp.lo, op_sp.lo, op_sp.hi, e_sp.hi].is_sorted()
                && let scx = match &mut self.scx {
                    Some(scx) => scx,
                    None if let Some((x, _)) = self.span.mk_edit_cx(self.cx) => self.scx.insert(x),
                    _ => return,
                }
                && let op_range = scx.span_to_file_range(op_sp)
                && let e_range = scx.span_to_file_range(e_sp)
                && scx
                    .get_text(..op_range.start)
                    .is_some_and(|src| src.ends_with(char::is_whitespace))
                && scx.get_text(op_range.clone()) == Some(op.node.as_str())
                && scx
                    .get_text(op_range.end..e_range.end)
                    .is_some_and(|src| src.starts_with(|c: char| !c.is_whitespace() && c != '/'))
                && let Some(op_range) = op_range.with_leading_whitespace(scx)
                && let op_range = scx.mk_source_range(op_range, None)
                && let Some(insert_sp) = match lhs.span.walk_into_other(&e_sp) {
                    Some(lhs_sp) => {
                        // Sanity check that the lhs actually comes first.
                        (lhs_sp.hi <= op_sp.lo).then(|| Span::new(lhs_sp.hi, lhs_sp.hi, lhs_sp.ctxt, lhs_sp.parent))
                    },
                    None => Some(Span::new(op_range.start, op_range.start, op_sp.ctxt, op_sp.parent)),
                }
            {
                span_lint_and_then(
                    self.cx,
                    POSSIBLE_MISSING_COMMA,
                    op.span,
                    "the is formatted like a unary operator, but it's parsed as a binary operator",
                    |diag| {
                        diag.span_suggestion(insert_sp, "add a comma before", ",", Applicability::MaybeIncorrect)
                            .span_suggestion(
                                Span::new(op_sp.hi, op_sp.hi, op_sp.ctxt, op_sp.parent),
                                "add a space after",
                                " ",
                                Applicability::MaybeIncorrect,
                            );
                    },
                );
            }
            self.check_missing_comma(lhs);
            self.check_missing_comma(rhs);
        }
    }

    fn check_missing_else(&mut self, first: &Expr, second: &Expr) {
        if matches!(first.kind, ExprKind::If(..))
            && let second_pat = match second.kind {
                ExprKind::If(..) => "if",
                ExprKind::Block(..) => "{",
                _ => return,
            }
            && let first_sp = first.span.data()
            && first_sp.ctxt == self.span.ctxt
            && let second_sp = second.span.data()
            && second_sp.ctxt == self.span.ctxt
            && [
                self.span.lo,
                first_sp.lo,
                first_sp.hi,
                second_sp.lo,
                second_sp.hi,
                self.span.hi,
            ]
            .is_sorted()
            && let scx = match &mut self.scx {
                Some(scx) => scx,
                None if let Some((x, _)) = self.span.mk_edit_cx(self.cx) => self.scx.insert(x),
                _ => return,
            }
            && let first_range = scx.span_to_file_range(first_sp)
            && let second_range = scx.span_to_file_range(second_sp)
            && scx
                .get_text(first_range.clone())
                .is_some_and(|src| src.starts_with("if") && src.ends_with('}'))
            && scx
                .get_text(first_range.end..second_range.start)
                .is_some_and(|s| s.chars().all(|c| c.is_whitespace() && c != '\n'))
            && let Some(lint_range) = second_range
                .clone()
                .map_range_text(scx, |s| s.split_prefix(second_pat).map(|[x, _]| x))
            && let Some(indent) = scx.get_line_indent_before(first_range.end)
        {
            span_lint_and_then(
                self.cx,
                POSSIBLE_MISSING_ELSE,
                scx.mk_span(lint_range, Some(second_range.clone())),
                "this is formatted as though there should be an `else`",
                |diag| {
                    let sugg_sp = scx.mk_span(first_range.end..second_range.start, None);
                    diag.span_suggestion(sugg_sp, "either add an `else`", " else ", Applicability::MaybeIncorrect)
                        .span_suggestion(
                            sugg_sp,
                            "or add a line break",
                            format!("\n{indent}"),
                            Applicability::MaybeIncorrect,
                        );
                },
            );
        }
    }
}
