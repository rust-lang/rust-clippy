use clippy_utils::diagnostics::{span_lint_and_note, span_lint_and_then};
use clippy_utils::source::{FileRangeExt, SpanExt, StrExt};
use clippy_utils::tokenize_with_text;
use core::mem;
use rustc_ast::{BinOp, BinOpKind, Block, Expr, ExprKind, MethodCall, StmtKind};
use rustc_errors::Applicability;
use rustc_lexer::TokenKind;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::{Span, SyntaxContext};

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

declare_lint_pass!(Formatting => [
    SUSPICIOUS_ASSIGNMENT_FORMATTING,
    SUSPICIOUS_UNARY_OP_FORMATTING,
    SUSPICIOUS_ELSE_FORMATTING,
    POSSIBLE_MISSING_ELSE,
    POSSIBLE_MISSING_COMMA
]);

impl EarlyLintPass for Formatting {
    fn check_block(&mut self, cx: &EarlyContext<'_>, block: &Block) {
        if block.stmts.len() >= 2
            && let ctxt = block.span.ctxt()
            && !ctxt.in_external_macro(cx.sess().source_map())
        {
            for [s1, s2] in block.stmts.array_windows::<2>() {
                if let (StmtKind::Expr(first), StmtKind::Expr(second) | StmtKind::Semi(second)) = (&s1.kind, &s2.kind) {
                    check_missing_else(cx, ctxt, first, second);
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
                let ctxt = expr.span.ctxt();
                if !ctxt.in_external_macro(cx.sess().source_map()) {
                    for e in args {
                        check_missing_comma(cx, ctxt, e);
                    }
                }
            },
            ExprKind::Paren(ref child) => {
                let ctxt = expr.span.ctxt();
                if !ctxt.in_external_macro(cx.sess().source_map()) {
                    check_missing_comma(cx, expr.span.ctxt(), child);
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
        && let Some([lint_sp, sep_sp]) = op_data.map_range(sm, |scx, range| {
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
        && let Some([lint_sp, sugg_sp]) = bin_op_data.map_range(sm, |scx, range| {
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

fn check_missing_comma(cx: &EarlyContext<'_>, ctxt: SyntaxContext, e: &Expr) {
    if let ExprKind::Binary(op, lhs, rhs) = &e.kind
        && let e_data = e.span.data()
        && e_data.ctxt == ctxt
    {
        if matches!(
            op.node,
            BinOpKind::And | BinOpKind::Mul | BinOpKind::Sub | BinOpKind::BitAnd
        ) && let op_data = op.span.data()
            && op_data.ctxt == e_data.ctxt
            && let Some(insert_sp) = op_data.map_range(cx, |scx, range| {
                range
                    .extend_end_to(scx, e_data.hi_ctxt())
                    .filter(|range| {
                        scx.get_text(..range.start)
                            .is_some_and(|src| src.ends_with(char::is_whitespace))
                            && scx
                                .get_text(range.clone())
                                .and_then(|src| src.strip_prefix(op.node.as_str()))
                                .is_some_and(|src| src.starts_with(|c: char| !c.is_whitespace() && c != '/'))
                    })?
                    .with_leading_whitespace(scx)
                    .map(|range| range.start..range.start)
            })
            && let Some(insert_sp) = match lhs.span.walk_to_ctxt(ctxt) {
                Some(lhs_sp) => {
                    let lhs_data = lhs_sp.data();
                    // Sanity check that the lhs actually comes first.
                    (lhs_data.hi <= insert_sp.hi())
                        .then(|| Span::new(lhs_data.hi, lhs_data.hi, lhs_data.ctxt, lhs_data.parent))
                },
                None => Some(insert_sp),
            }
        {
            span_lint_and_then(
                cx,
                POSSIBLE_MISSING_COMMA,
                op.span,
                "the is formatted like a unary operator, but it's parsed as a binary operator",
                |diag| {
                    diag.span_suggestion(insert_sp, "add a comma before", ",", Applicability::MaybeIncorrect)
                        .span_suggestion(
                            Span::new(op_data.hi, op_data.hi, op_data.ctxt, op_data.parent),
                            "add a space after",
                            " ",
                            Applicability::MaybeIncorrect,
                        );
                },
            );
        }
        check_missing_comma(cx, ctxt, lhs);
        check_missing_comma(cx, ctxt, rhs);
    }
}

fn check_missing_else(cx: &EarlyContext<'_>, ctxt: SyntaxContext, first: &Expr, second: &Expr) {
    if matches!(first.kind, ExprKind::If(..))
        && matches!(second.kind, ExprKind::If(..) | ExprKind::Block(..))
        && let first_data = first.span.data()
        && let second_data = second.span.data()
        && first_data.ctxt == ctxt
        && second_data.ctxt == ctxt
        && let Some((scx, range)) = first_data.mk_edit_cx(cx)
        && scx
            .get_text(range.clone())
            .is_some_and(|src| src.starts_with("if") && src.ends_with('}'))
        && let Some(range) = range.get_range_between(&scx, second_data)
        && scx
            .get_text(range.clone())
            .is_some_and(|src| src.chars().all(|c| c != '\n' && c.is_whitespace()))
        && let Some(indent) = scx.get_line_indent_before(range.start)
    {
        let lint_sp = scx.mk_span(range);
        span_lint_and_then(
            cx,
            POSSIBLE_MISSING_ELSE,
            lint_sp,
            "this is formatted as though there should be an `else`",
            |diag| {
                diag.span_suggestion(lint_sp, "add an `else`", " else ", Applicability::MaybeIncorrect)
                    .span_suggestion(
                        lint_sp,
                        "add a line break",
                        format!("\n{indent}"),
                        Applicability::MaybeIncorrect,
                    );
            },
        );
    }
}
