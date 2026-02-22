use super::{EXPLICIT_COUNTER_LOOP, IncrementVisitor, InitializeVisitor, make_iterator_snippet};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{get_enclosing_block, is_integer_const};
use rustc_ast::Label;
use rustc_errors::Applicability;
use rustc_hir::intravisit::{walk_block, walk_expr};
use rustc_hir::{Expr, Pat};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty, UintTy};

// To trigger the EXPLICIT_COUNTER_LOOP lint, a variable must be
// incremented exactly once in the loop body, and initialized to zero
// at the start of the loop.
// ... (imports and prelude)

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'_>,
    arg: &'tcx Expr<'_>,
    body: &'tcx Expr<'_>,
    expr: &'tcx Expr<'_>,
    label: Option<Label>,
) {
    let mut increment_visitor = IncrementVisitor::new(cx);
    walk_expr(&mut increment_visitor, body);

    if let Some(block) = get_enclosing_block(cx, expr.hir_id) {
        for (id, increment_span) in increment_visitor.into_results() {
            let mut initialize_visitor = InitializeVisitor::new(cx, expr, id);
            walk_block(&mut initialize_visitor, block);

            if let Some((name, ty, initializer, init_span)) = initialize_visitor.get_result()
                && is_integer_const(cx, initializer, 0)
            {
                let mut applicability = Applicability::MaybeIncorrect;
                let span = expr.span.with_hi(arg.span.hi());
                let loop_label = label.map_or(String::new(), |l| format!("{}: ", l.ident.name));

                // Determine the suggestion and optional note based on the variable's type
                let suggestion_info = match ty.map(Ty::kind) {
                    Some(ty::Uint(UintTy::Usize)) | None => {
                        // usize or inferred (uses enumerate)
                        Some((
                            format!(
                                "{loop_label}for ({name}, {}) in {}.enumerate()",
                                snippet_with_applicability(cx, pat.span, "item", &mut applicability),
                                make_iterator_snippet(cx, arg, &mut applicability),
                            ),
                            None,
                        ))
                    },
                    Some(ty::Int(int_ty)) => {
                        // Signed integer types (uses (0_type..).zip)
                        let int_name = int_ty.name_str();
                        Some((
                            format!(
                                "{loop_label}for ({name}, {}) in (0_{int_name}..).zip({})",
                                snippet_with_applicability(cx, pat.span, "item", &mut applicability),
                                make_iterator_snippet(cx, arg, &mut applicability),
                            ),
                            Some(format!(
                                "`{name}` is of type `{int_name}`, making it ineligible for `Iterator::enumerate`"
                            )),
                        ))
                    },
                    Some(ty::Uint(uint_ty)) => {
                        // Other unsigned integer types (uses (0_type..).zip)
                        let uint_name = uint_ty.name_str();
                        Some((
                            format!(
                                "{loop_label}for ({name}, {}) in (0_{uint_name}..).zip({})",
                                snippet_with_applicability(cx, pat.span, "item", &mut applicability),
                                make_iterator_snippet(cx, arg, &mut applicability),
                            ),
                            Some(format!(
                                "`{name}` is of type `{uint_name}`, making it ineligible for `Iterator::enumerate`"
                            )),
                        ))
                    },
                    // Anything else (e.g., f32, struct) is ineligible
                    _ => None,
                };

                // If ineligible, stop processing this counter variable
                let Some((suggestion, note)) = suggestion_info else {
                    continue;
                };

                span_lint_and_then(
                    cx,
                    EXPLICIT_COUNTER_LOOP,
                    expr.span,
                    format!("the variable `{name}` is used as a loop counter"),
                    |diag| {
                        diag.multipart_suggestion(
                            "consider rewriting the loop to use an iterator adapter",
                            vec![
                                (span, suggestion),
                                (init_span, String::new()),
                                (increment_span, String::new()),
                            ],
                            applicability,
                        );

                        if let Some(note_text) = note {
                            diag.note(note_text);
                        }
                    },
                );
            }
        }
    }
}
