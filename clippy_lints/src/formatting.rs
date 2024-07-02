use clippy_utils::diagnostics::{span_lint_and_note, span_lint_and_then};
use clippy_utils::source::{walk_span_to_context, IntoSpan, SpanRangeExt};
use clippy_utils::{is_span_if, tokenize_with_text};
use core::mem;
use rustc_ast::{BinOp, BinOpKind, Block, Expr, ExprKind, MethodCall, StmtKind};
use rustc_errors::Applicability;
use rustc_lexer::TokenKind;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::declare_lint_pass;
use rustc_span::{BytePos, Pos, Span, SyntaxContext};

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
        && (op_data.lo..assign_data.hi).check_source_text(cx, |src| {
            if let Some(src) = src.strip_prefix('=')
                && let Some(src) = src.strip_prefix(op.as_str())
            {
                src.starts_with(|c: char| c.is_whitespace())
            } else {
                false
            }
        })
        && !in_external_macro(cx.sess(), assign.span)
    {
        let lint_range = op_data.lo..op_data.lo + BytePos(2);
        let lint_sp = lint_range.clone().with_ctxt(assign_data.ctxt);
        span_lint_and_then(
            cx,
            SUSPICIOUS_ASSIGNMENT_FORMATTING,
            lint_sp,
            "this looks similar to a compound assignment operator",
            |diag| {
                let op = op.as_str();
                diag.span_suggestion(
                    lint_sp,
                    "reverse the characters",
                    format!("{op}="),
                    Applicability::MaybeIncorrect,
                )
                .span_suggestion(
                    lint_range.with_trailing_whitespace(cx).with_ctxt(assign_data.ctxt),
                    "separate the characters",
                    format!("= {op}"),
                    Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

/// Implementation of the `SUSPICIOUS_UNARY_OP_FORMATTING` lint.
fn check_un_op(cx: &EarlyContext<'_>, bin_expr: &Expr, bin_op: &BinOp, rhs: &Expr) {
    if let ExprKind::Unary(un_op, _) = rhs.kind
        && let ctxt = bin_expr.span.ctxt()
        && let bin_op_data = bin_op.span.data()
        && bin_op_data.ctxt == ctxt
        && let rhs_data = rhs.span.data()
        && rhs_data.ctxt == ctxt
        && let bin_op_str = bin_op.node.as_str()
        && let un_op_str = un_op.as_str()
        && (bin_op_data.lo..rhs_data.hi).check_source_text(cx, |src| {
            if let Some(src) = src.strip_prefix(bin_op_str)
                && let Some(src) = src.strip_prefix(un_op_str)
            {
                src.starts_with(|c: char| c.is_whitespace())
            } else {
                false
            }
        })
        && !in_external_macro(cx.sess(), bin_expr.span)
    {
        let range = bin_op_data.lo
            ..bin_op_data.lo + BytePos::from_usize(bin_op_str.len()) + BytePos::from_usize(un_op_str.len());
        span_lint_and_then(
            cx,
            SUSPICIOUS_UNARY_OP_FORMATTING,
            range.clone().with_ctxt(ctxt),
            "this formatting makes the binary and unary operators look like a single operator",
            |diag| {
                diag.span_suggestion(
                    range.with_trailing_whitespace(cx).with_ctxt(ctxt),
                    "add a space between",
                    format!("{bin_op_str} {un_op_str}"),
                    if ctxt.is_root() {
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
    let ctxt = expr.span.ctxt();
    if then.span.ctxt() == ctxt
        && else_.span.ctxt() == ctxt
        && let is_block = matches!(else_.kind, ExprKind::Block(..))
        && let else_range = (then.span.hi()..else_.span.lo())
        && else_range.clone().with_source_text(cx, |src| {
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
                    Some((TokenKind::Whitespace, text)) => match text.bytes().filter(|&c| c == b'\n').count() {
                        0 => {},
                        x => lf_count += x - usize::from(mem::replace(&mut skip_lf, false)),
                    },
                    Some((TokenKind::LineComment { .. }, _)) => skip_lf = lf_count != 0,
                    Some((TokenKind::BlockComment { .. }, text)) => {
                        if lf_count == 0 {
                            lf_count = usize::from(text.contains('\n'));
                        }
                        skip_lf = lf_count != 0;
                    },
                    Some((TokenKind::Ident, "else")) if skip_lf || lf_count > 1 => return true,
                    Some((TokenKind::Ident, "else")) => break,
                    _ => return false,
                }
            }
            let mut allow_lf = is_block && lf_count != 0;
            skip_lf = false;
            lf_count = 0;
            for (kind, text) in tokens {
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
        }) == Some(true)
        && !in_external_macro(cx.sess(), expr.span)
    {
        let else_desc = if is_block { "{..}" } else { "if" };
        span_lint_and_note(
            cx,
            SUSPICIOUS_ELSE_FORMATTING,
            else_range.with_ctxt(ctxt),
            format!("this is an `else {else_desc}` but the formatting might hide it"),
            None,
            format!(
                "to remove this lint, remove the `else` or remove the new line between \
                 `else` and `{else_desc}`",
            ),
        );
    }
}

fn check_missing_comma(cx: &EarlyContext<'_>, ctxt: SyntaxContext, e: &Expr) {
    if let ExprKind::Binary(op, lhs, rhs) = &e.kind
        && e.span.ctxt() == ctxt
    {
        if matches!(
            op.node,
            BinOpKind::And | BinOpKind::Mul | BinOpKind::Sub | BinOpKind::BitAnd
        ) && let op_data = op.span.data()
            && op_data.ctxt == ctxt
            && (op_data.lo..e.span.hi()).check_source_text_with_range(cx, |src, range| {
                if let Some(src) = src.get(..range.end)
                    && let Some((pre, src)) = src.split_at_checked(range.start)
                    && let Some(stripped) = src.strip_prefix(op.node.as_str())
                {
                    stripped.starts_with(|c: char| !c.is_whitespace() && c != '/')
                        && pre.ends_with(|c: char| c.is_whitespace())
                } else {
                    false
                }
            })
            && let Some(lhs_sp) = walk_span_to_context(lhs.span, ctxt)
            && !in_external_macro(cx.sess(), e.span)
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
                        Span::new(op_data.hi, op_data.hi, ctxt, None),
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
        && first.span.ctxt() == ctxt
        && second.span.ctxt() == ctxt
        && is_span_if(cx, first.span)
        && let else_range = (first.span.hi()..second.span.lo())
        && else_range.clone().with_source_text(cx, |src| {
            // Only lint when the end of the first expression and the start of the
            // second are on the same line without anything in between.
            src.chars().all(|c| c != '\n' && c.is_whitespace())
        }) == Some(true)
    {
        let sp = else_range.with_ctxt(first.span.ctxt());
        span_lint_and_then(
            cx,
            SUSPICIOUS_ELSE_FORMATTING,
            sp,
            "this is formatted as though there should be an `else`",
            |diag| {
                diag.span_suggestion(sp, "add an `else`", " else ", Applicability::MaybeIncorrect)
                    .span_suggestion(
                        sp,
                        "add a line break",
                        first.span.with_line_indent(cx, |indent| format!("\n{indent}")),
                        Applicability::MaybeIncorrect,
                    );
            },
        );
    }
}
