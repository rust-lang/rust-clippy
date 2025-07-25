use clippy_utils::diagnostics::{span_lint_and_note, span_lint_and_then};
use clippy_utils::source::{SpanExt, walk_span_to_context};
use clippy_utils::tokenize_with_text;
use core::mem;
use rustc_ast::{BinOp, BinOpKind, Block, Expr, ExprKind, MethodCall, StmtKind};
use rustc_errors::Applicability;
use rustc_lexer::TokenKind;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::{Pos, Span, SyntaxContext};

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
    #[clippy::version = "1.90.0"]
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
        let ctxt = block.span.ctxt();
        for [s1, s2] in block.stmts.array_windows::<2>() {
            if let (StmtKind::Expr(first), StmtKind::Expr(second) | StmtKind::Semi(second)) = (&s1.kind, &s2.kind) {
                check_missing_else(cx, ctxt, first, second);
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
                for e in args {
                    check_missing_comma(cx, ctxt, e);
                }
            },
            ExprKind::Paren(ref child) => check_missing_comma(cx, expr.span.ctxt(), child),
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
        && let Some(mut check_range) = op_data.get_source_range(cx)
        && let Some(check_range) = check_range.set_end_if_after(assign_data.hi)
        && let Some(check_range) = check_range.edit_range(|src, range| {
            if let Some(src) = src.get(range.clone())
                && let Some(src) = src.strip_prefix('=')
                && let Some(src) = src.strip_prefix(op_str)
                && src.starts_with(|c: char| c.is_whitespace())
            {
                Some(range.start..range.start + 2)
            } else {
                None
            }
        })
        && let lint_range = check_range.source_range()
        && let Some(sep_range) = check_range.add_trailing_whitespace()
        && !assign_data.ctxt.in_external_macro(cx.sess().source_map())
    {
        let sep_range = sep_range.source_range();
        let lint_sp = Span::new(lint_range.start, lint_range.end, assign_data.ctxt, assign_data.parent);
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
                    Span::new(sep_range.start, sep_range.end, assign_data.ctxt, assign_data.parent),
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
        && let Some(mut check_range) = bin_op_data.get_source_range(cx)
        && let Some(check_range) = check_range.set_end_if_after(rhs_data.hi)
        && let Some(check_range) = check_range.edit_range(|src, range| {
            if let Some(src) = src.get(range.clone())
                && let Some(src) = src.strip_prefix(bin_op_str)
                && let Some(src) = src.strip_prefix(un_op_str)
                && src.starts_with(|c: char| c.is_whitespace())
            {
                Some(range.start..range.start + bin_op_str.len() + un_op_str.len())
            } else {
                None
            }
        })
        && let lint_range = check_range.source_range()
        && let Some(sugg_range) = check_range.add_trailing_whitespace()
        && !bin_op_data.ctxt.in_external_macro(cx.sess().source_map())
    {
        span_lint_and_then(
            cx,
            SUSPICIOUS_UNARY_OP_FORMATTING,
            Span::new(lint_range.start, lint_range.end, bin_op_data.ctxt, bin_op_data.parent),
            "this formatting makes the binary and unary operators look like a single operator",
            |diag| {
                let sugg_range = sugg_range.source_range();
                diag.span_suggestion(
                    Span::new(sugg_range.start, sugg_range.end, bin_op_data.ctxt, bin_op_data.parent),
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
        && let Some(mut check_range) = then_data.get_source_range(cx)
        && let Some(check_range) = check_range.set_range_between_other(else_data)
        && let is_else_block = matches!(else_.kind, ExprKind::Block(..))
        && check_range
            .current_text()
            .is_some_and(|src| check_else_formatting(src, is_else_block))
        && !then_data.ctxt.in_external_macro(cx.sess().source_map())
    {
        let else_desc = if is_else_block { "{..}" } else { "if" };
        let range = check_range.source_range();
        span_lint_and_note(
            cx,
            SUSPICIOUS_ELSE_FORMATTING,
            Span::new(range.start, range.end, then_data.ctxt, then_data.parent),
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
            && let Some(mut check_range) = op_data.get_source_range(cx)
            && let Some(check_range) = check_range.set_end_if_after(e_data.hi)
            && let Some(src) = check_range.file_text().get(..check_range.range().end.to_usize())
            && let Some((pre_src, src)) = src.split_at_checked(check_range.range().start.to_usize())
            && let Some(src) = src.strip_prefix(op.node.as_str())
            && src.starts_with(|c: char| !c.is_whitespace() && c != '/')
            && pre_src.ends_with(|c: char| c.is_whitespace())
            && let Some(lhs_sp) = walk_span_to_context(lhs.span, ctxt)
            && !ctxt.in_external_macro(cx.sess().source_map())
        {
            span_lint_and_then(
                cx,
                POSSIBLE_MISSING_COMMA,
                op.span,
                "the is formatted like a unary operator, but it's parsed as a binary operator",
                |diag| {
                    diag.span_suggestion(
                        lhs_sp.shrink_to_hi(),
                        "add a comma before",
                        ",",
                        Applicability::MaybeIncorrect,
                    )
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
        && let Some(mut check_range) = first_data.get_source_range(cx)
        && check_range.current_text().is_some_and(|src| src.starts_with("if") && src.ends_with('}'))
        && let Some(check_range) = check_range.set_range_between_other(second_data)
        // Only lint when the end of the first expression and the start of the
        // second are on the same line without anything in between.
        && check_range.current_text().is_some_and(|src| src.chars().all(|c| c != '\n' && c.is_whitespace()))
    {
        let range = check_range.source_range();
        let sp = Span::new(range.start, range.end, first_data.ctxt, first_data.parent);
        span_lint_and_then(
            cx,
            POSSIBLE_MISSING_ELSE,
            sp,
            "this is formatted as though there should be an `else`",
            |diag| {
                diag.span_suggestion(sp, "add an `else`", " else ", Applicability::MaybeIncorrect)
                    .span_suggestion(
                        sp,
                        "add a line break",
                        format!("\n{}", check_range.get_line_indent()),
                        Applicability::MaybeIncorrect,
                    );
            },
        );
    }
}
