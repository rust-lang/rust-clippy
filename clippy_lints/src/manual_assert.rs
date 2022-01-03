use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher::PanicExpn;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{is_expn_of, sugg};
use rustc_errors::Applicability;
use rustc_hir::{Block, Expr, ExprKind, StmtKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Detects `if`-then-`panic!` that can be replaced with `assert!`.
    ///
    /// ### Why is this bad?
    /// `assert!` is simpler than `if`-then-`panic!`.
    ///
    /// ### Example
    /// ```rust
    /// let sad_people: Vec<&str> = vec![];
    /// if !sad_people.is_empty() {
    ///     panic!("there are sad people: {:?}", sad_people);
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// let sad_people: Vec<&str> = vec![];
    /// assert!(sad_people.is_empty(), "there are sad people: {:?}", sad_people);
    /// ```
    #[clippy::version = "1.57.0"]
    pub MANUAL_ASSERT,
    pedantic,
    "`panic!` and only a `panic!` in `if`-then statement"
}

declare_lint_pass!(ManualAssert => [MANUAL_ASSERT]);

impl LateLintPass<'_> for ManualAssert {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if_chain! {
            if let Expr {
                kind: ExprKind:: If(cond, Expr {
                    kind: ExprKind::Block(
                        Block {
                            stmts: [stmt],
                            ..
                        },
                        _),
                    ..
                }, None),
                ..
            } = &expr;
            if is_expn_of(stmt.span, "panic").is_some();
            if !matches!(cond.kind, ExprKind::Let(_));
            if let StmtKind::Semi(semi) = stmt.kind;
            if !cx.tcx.sess.source_map().is_multiline(cond.span);

            then {
                let call = if_chain! {
                    if let ExprKind::Block(block, _) = semi.kind;
                    if let Some(init) = block.expr;
                    then {
                        init
                    } else {
                        semi
                    }
                };
                let span = if let Some(panic_expn) = PanicExpn::parse(cx, call) {
                    match *panic_expn.format_args.value_args {
                        [] => panic_expn.format_args.format_string_span,
                        [.., last] => panic_expn.format_args.format_string_span.to(last.span),
                    }
                } else if let ExprKind::Call(_, [format_args]) = call.kind {
                    format_args.span
                } else {
                    return
                };
                let mut applicability = Applicability::MachineApplicable;
                let sugg = snippet_with_applicability(cx, span, "..", &mut applicability);
                let cond_sugg = if let ExprKind::DropTemps(e, ..) = cond.kind {
                    if let Expr{kind: ExprKind::Unary(UnOp::Not, not_expr), ..} = e {
                         sugg::Sugg::hir_with_applicability(cx, not_expr, "..", &mut applicability).maybe_par().to_string()
                    } else {
                       format!("!{}", sugg::Sugg::hir_with_applicability(cx, e, "..", &mut applicability).maybe_par())
                    }
                } else {
                   format!("!{}", sugg::Sugg::hir_with_applicability(cx, cond, "..", &mut applicability).maybe_par())
                };

                span_lint_and_sugg(
                    cx,
                    MANUAL_ASSERT,
                    expr.span,
                    "only a `panic!` in `if`-then statement",
                    "try",
                    format!("assert!({}, {});", cond_sugg, sugg),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}
