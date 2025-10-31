use rustc_ast::{Block, Expr, ExprKind, MethodCall, StmtKind};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;

mod possible_missing_comma;
mod possible_missing_else;
mod suspicious_assignment_formatting;
mod suspicious_else_formatting;
mod suspicious_unary_op_formatting;

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
                    possible_missing_else::check(cx, ctxt, first, second);
                }
            }
        }
    }

    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        match expr.kind {
            ExprKind::If(_, ref then, Some(ref else_)) => suspicious_else_formatting::check(cx, expr, then, else_),
            ExprKind::Assign(_, ref rhs, sp) => suspicious_assignment_formatting::check(cx, expr, rhs, sp),
            ExprKind::Binary(ref bin_op, _, ref rhs) => suspicious_unary_op_formatting::check(cx, expr, bin_op, rhs),
            ExprKind::Array(ref args)
            | ExprKind::Tup(ref args)
            | ExprKind::Call(_, ref args)
            | ExprKind::MethodCall(box MethodCall { ref args, .. }) => {
                let ctxt = expr.span.ctxt();
                if !ctxt.in_external_macro(cx.sess().source_map()) {
                    for e in args {
                        possible_missing_comma::check(cx, ctxt, e);
                    }
                }
            },
            ExprKind::Paren(ref child) => {
                let ctxt = expr.span.ctxt();
                if !ctxt.in_external_macro(cx.sess().source_map()) {
                    possible_missing_comma::check(cx, expr.span.ctxt(), child);
                }
            },
            _ => {},
        }
    }
}
