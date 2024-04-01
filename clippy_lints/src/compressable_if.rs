use clippy_utils::diagnostics::{multispan_sugg_with_applicability, span_lint_and_then};
use clippy_utils::source::snippet;
use clippy_utils::SpanlessEq;
use rustc_errors::MultiSpan;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, Expr, ExprKind, FnDecl, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for two `if` blocks with the same content
    /// and suggests merging them.
    /// ### Why is this bad?
    /// This make the code more complex and harder to read.
    /// ### Example
    /// ```no_run
    /// if a == 1 {
    ///    println!("odd");
    /// }
    /// if a == 3 {
    ///   println!("odd");
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// if a == 1 || a == 3 {
    ///   println!("odd");
    /// }
    /// ```
    #[clippy::version = "1.79.0"]
    pub COMPRESSABLE_IF,
    complexity,
    "if two `if` blocks have the same content, they can be merged"
}

declare_lint_pass!(CompressableIf => [COMPRESSABLE_IF]);

impl<'tcx> LateLintPass<'tcx> for CompressableIf {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        _fn_span: Span,
        _: LocalDefId,
    ) {
        // DEBUG
        // println!("\n\n\n");

        let mut compressables = function_has_compressable_if(cx, body);
        while let Some((first, second)) = compressables.pop() {
            // span_lint_and_note(
            //     cx,
            //     COMPRESSABLE_IF,
            //     fn_span,
            //     "these if blocks have the same content and no side effects",
            //     None,
            //     "consider merging them into a single if block with a combined condition",
            // );

            // Construct multi-span lint
            let spans = MultiSpan::from_spans(vec![first.span, second.span]);

            // Deconstruct the pair into cond and then
            let ExprKind::If(first_cond, first_then, _) = first.kind else {
                unreachable!()
            };

            let ExprKind::If(second_cond, _, _) = second.kind else {
                unreachable!()
            };

            span_lint_and_then(
                cx,
                COMPRESSABLE_IF,
                spans,
                "these if blocks have the same content and no side effects",
                |diag| {
                    diag.help("consider merging them into a single if block with a combined condition");
                    // diag.span_suggestion(
                    //     first_cond.span,
                    //     "merge these if blocks",
                    //     format!("if {} || {}", snippet(cx, first_cond.span, ".."), snippet(cx, second_cond.span,
                    // "..")),     rustc_errors::Applicability::MaybeIncorrect,
                    // );
                    // diag.span_suggestion(
                    //     second.span,
                    //     "remove this if block",
                    //     String::new(),
                    //     rustc_errors::Applicability::MaybeIncorrect,
                    // );
                    // Multispan instead???
                    multispan_sugg_with_applicability(
                        diag,
                        "merge these if blocks",
                        rustc_errors::Applicability::Unspecified,
                        // vec<(span, replace)>
                        vec![(
                            first.span,
                            format!(
                                "if {} || {} {} ",
                                snippet(cx, first_cond.span, ".."),
                                snippet(cx, second_cond.span, ".."),
                                snippet(cx, first_then.span, "..")
                            ),
                        )]
                        .into_iter()
                        .chain(vec![(second.span, String::new())]),
                    );
                },
            );
        }
    }
}

fn function_has_compressable_if<'b>(cx: &LateContext<'_>, body: &Body<'b>) -> Vec<(&'b Expr<'b>, &'b Expr<'b>)> {
    // The body is an Expr, how is it structured?
    // It's a block, which contains a list of statements.

    let to_debug = body.value.kind;
    if let ExprKind::Block(to_debug, _) = to_debug {
        let statements = to_debug.stmts; // Does not include the final expression

        // DEBUG
        // for statement in statements.iter() {
        //     println!("{:?}\n", statement);
        // }

        // Now we have a list of statements.
        // We want to find a pair of if statements that
        // 1. Have the same content (In progress)
        // 2. Have no side effects between them (Not implemented)

        let mut found_if: Vec<&Expr<'_>> = Vec::new(); // The list of found if statements
        let mut found_compress: Vec<(&Expr<'_>, &Expr<'_>)> = Vec::new(); // The list of found compressable if statements 

        let mut spannless_eq = SpanlessEq::new(cx); // for comparing expressions

        for stmt in statements {
            if let StmtKind::Expr(expr) = stmt.kind {
                if let ExprKind::If(_, _, _) = expr.kind {
                    // before pushing, check if the then is the same as any of the previous thens

                    for prev in &found_if {
                        if let ExprKind::If(_, prev_then, _) = prev.kind
                            && let ExprKind::If(_, expr_then, _) = expr.kind
                            && spannless_eq.eq_expr(expr_then, prev_then)
                        {
                            // The thens are the same, we have a compressable if
                            // found_compress.push((prev_cond, then));
                            found_compress.push((prev, expr));
                        }
                    }

                    found_if.push(expr);
                } else {
                    // If the next expression is not an if, we can't be sure that the ifs can be merged
                    found_if.clear();
                }
            } else {
                // If the next statement is not an expression, it can't be an if, so the same thing applies
                found_if.clear();
            }
        }

        return found_compress;
    }

    Vec::new()
}
