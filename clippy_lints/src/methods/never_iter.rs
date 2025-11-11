use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use clippy_utils::visitors::{Descend, for_each_expr_without_closures};
use rustc_errors::Applicability;
use rustc_hir::{Block, Destination, Expr, ExprKind, HirId, InlineAsm, InlineAsmOperand, Node, Stmt, StmtKind};
use rustc_lint::LateContext;
use rustc_span::{BytePos, Span, sym};
use std::iter::once;
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for iterator methods with closures that always diverge.
    ///
    /// ### Why is this bad?
    /// The iterator method will only process the first element before diverging,
    /// which is often not the intended behavior.
    ///
    /// ### Example
    /// ```no_run
    /// fn diverge() -> ! {
    ///     panic!();
    /// }
    ///
    /// [0, 1].into_iter().for_each(|x| diverge()); // Only calls diverge once
    /// ```
    #[clippy::version = "1.75.0"]
    pub NEVER_ITER,
    correctness,
    "iterator methods with closures that always diverge"
}

// Copy the necessary logic from never_loop to analyze closures
#[derive(Clone, Debug)]
enum NeverLoopResult {
    /// A continue may occur for the main loop.
    MayContinueMainLoop,
    /// We have not encountered any main loop continue,
    /// but we are diverging (subsequent control flow is not reachable)
    Diverging {
        break_spans: Vec<Span>,
        never_spans: Vec<Span>,
    },
    /// We have not encountered any main loop continue,
    /// and subsequent control flow is (possibly) reachable
    Normal,
}

fn never_loop_block<'tcx>(
    cx: &LateContext<'tcx>,
    block: &Block<'tcx>,
    local_labels: &mut Vec<(HirId, bool)>,
    main_loop_id: HirId,
) -> NeverLoopResult {
    let iter = block
        .stmts
        .iter()
        .filter_map(stmt_to_expr)
        .chain(block.expr.map(|expr| (expr, None)));
    combine_seq_many(iter.map(|(e, els)| {
        let e = never_loop_expr(cx, e, local_labels, main_loop_id);
        // els is an else block in a let...else binding
        els.map_or(e.clone(), |els| {
            combine_seq(e, || match never_loop_block(cx, els, local_labels, main_loop_id) {
                // Returning MayContinueMainLoop here means that
                // we will not evaluate the rest of the body
                NeverLoopResult::MayContinueMainLoop => NeverLoopResult::MayContinueMainLoop,
                // An else block always diverges, so the Normal case should not happen,
                // but the analysis is approximate so it might return Normal anyway.
                // Returning Normal here says that nothing more happens on the main path
                NeverLoopResult::Diverging { .. } | NeverLoopResult::Normal => NeverLoopResult::Normal,
            })
        })
    }))
}

fn stmt_to_expr<'tcx>(stmt: &Stmt<'tcx>) -> Option<(&'tcx Expr<'tcx>, Option<&'tcx Block<'tcx>>)> {
    match stmt.kind {
        StmtKind::Semi(e) | StmtKind::Expr(e) => Some((e, None)),
        // add the let...else expression (if present)
        StmtKind::Let(local) => local.init.map(|init| (init, local.els)),
        StmtKind::Item(..) => None,
    }
}

fn stmt_source_span(stmt: &Stmt<'_>) -> Span {
    let call_span = stmt.span.source_callsite();
    // if it is a macro call, the span will be missing the trailing semicolon
    if stmt.span == call_span {
        return call_span;
    }

    // An expression without a trailing semi-colon (must have unit type).
    if let StmtKind::Expr(..) = stmt.kind {
        return call_span;
    }

    call_span.with_hi(call_span.hi() + BytePos(1))
}

/// Returns a Vec of all the individual spans after the highlighted expression in a block
fn all_spans_after_expr(cx: &LateContext<'_>, expr: &Expr<'_>) -> Vec<Span> {
    if let Node::Stmt(stmt) = cx.tcx.parent_hir_node(expr.hir_id) {
        if let Node::Block(block) = cx.tcx.parent_hir_node(stmt.hir_id) {
            return block
                .stmts
                .iter()
                .skip_while(|inner| inner.hir_id != stmt.hir_id)
                .map(stmt_source_span)
                .chain(block.expr.map(|e| e.span))
                .collect();
        }

        return vec![stmt.span];
    } else if let Node::Block(_) = cx.tcx.parent_hir_node(expr.hir_id) {
        return vec![expr.span];
    }

    vec![]
}

#[expect(clippy::too_many_lines)]
fn never_loop_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    local_labels: &mut Vec<(HirId, bool)>,
    main_loop_id: HirId,
) -> NeverLoopResult {
    let result = match expr.kind {
        ExprKind::Unary(_, e)
        | ExprKind::Cast(e, _)
        | ExprKind::Type(e, _)
        | ExprKind::Field(e, _)
        | ExprKind::AddrOf(_, _, e)
        | ExprKind::Repeat(e, _)
        | ExprKind::DropTemps(e)
        | ExprKind::UnsafeBinderCast(_, e, _) => never_loop_expr(cx, e, local_labels, main_loop_id),
        ExprKind::Let(let_expr) => never_loop_expr(cx, let_expr.init, local_labels, main_loop_id),
        ExprKind::Array(es) | ExprKind::Tup(es) => never_loop_expr_all(cx, es.iter(), local_labels, main_loop_id),
        ExprKind::Use(expr, _) => never_loop_expr(cx, expr, local_labels, main_loop_id),
        ExprKind::MethodCall(_, receiver, es, _) => {
            never_loop_expr_all(cx, once(receiver).chain(es.iter()), local_labels, main_loop_id)
        },
        ExprKind::Struct(_, fields, base) => {
            let fields = never_loop_expr_all(cx, fields.iter().map(|f| f.expr), local_labels, main_loop_id);
            if let rustc_hir::StructTailExpr::Base(base) = base {
                combine_seq(fields, || never_loop_expr(cx, base, local_labels, main_loop_id))
            } else {
                fields
            }
        },
        ExprKind::Call(e, es) => never_loop_expr_all(cx, once(e).chain(es.iter()), local_labels, main_loop_id),
        ExprKind::Binary(_, e1, e2)
        | ExprKind::Assign(e1, e2, _)
        | ExprKind::AssignOp(_, e1, e2)
        | ExprKind::Index(e1, e2, _) => never_loop_expr_all(cx, [e1, e2].iter().copied(), local_labels, main_loop_id),
        ExprKind::Loop(b, _, _, _) => {
            // We don't attempt to track reachability after a loop,
            // just assume there may have been a break somewhere
            absorb_break(&never_loop_block(cx, b, local_labels, main_loop_id))
        },
        ExprKind::If(e, e2, e3) => {
            let e1 = never_loop_expr(cx, e, local_labels, main_loop_id);
            combine_seq(e1, || {
                let e2 = never_loop_expr(cx, e2, local_labels, main_loop_id);
                let e3 = e3.as_ref().map_or(NeverLoopResult::Normal, |e| {
                    never_loop_expr(cx, e, local_labels, main_loop_id)
                });
                combine_branches(e2, e3)
            })
        },
        ExprKind::Match(e, arms, _) => {
            let e = never_loop_expr(cx, e, local_labels, main_loop_id);
            combine_seq(e, || {
                arms.iter().fold(
                    NeverLoopResult::Diverging {
                        break_spans: vec![],
                        never_spans: vec![],
                    },
                    |a, b| combine_branches(a, never_loop_expr(cx, b.body, local_labels, main_loop_id)),
                )
            })
        },
        ExprKind::Block(b, _) => {
            if b.targeted_by_break {
                local_labels.push((b.hir_id, false));
            }
            let ret = never_loop_block(cx, b, local_labels, main_loop_id);
            let jumped_to = b.targeted_by_break && local_labels.pop().unwrap().1;
            match ret {
                NeverLoopResult::Diverging { .. } if jumped_to => NeverLoopResult::Normal,
                _ => ret,
            }
        },
        ExprKind::Continue(d) => {
            let id = d
                .target_id
                .expect("target ID can only be missing in the presence of compilation errors");
            if id == main_loop_id {
                NeverLoopResult::MayContinueMainLoop
            } else {
                NeverLoopResult::Diverging {
                    break_spans: all_spans_after_expr(cx, expr),
                    never_spans: vec![],
                }
            }
        },
        ExprKind::Ret(e) => {
            let first = e.as_ref().map_or(NeverLoopResult::Normal, |e| {
                never_loop_expr(cx, e, local_labels, main_loop_id)
            });
            combine_seq(first, || {
                NeverLoopResult::Diverging {
                    break_spans: vec![],
                    never_spans: vec![],
                }
            })
        },
        ExprKind::Break(dest, e) => {
            let first = e.as_ref().map_or(NeverLoopResult::Normal, |e| {
                never_loop_expr(cx, e, local_labels, main_loop_id)
            });
            combine_seq(first, || {
                NeverLoopResult::Diverging {
                    break_spans: vec![],
                    never_spans: vec![],
                }
            })
        },
        ExprKind::Become(e) => combine_seq(never_loop_expr(cx, e, local_labels, main_loop_id), || {
            NeverLoopResult::Diverging {
                break_spans: vec![],
                never_spans: vec![],
            }
        }),
        ExprKind::InlineAsm(asm) => combine_seq_many(asm.operands.iter().map(|(o, _)| match o {
            InlineAsmOperand::In { expr, .. } | InlineAsmOperand::InOut { expr, .. } => {
                never_loop_expr(cx, expr, local_labels, main_loop_id)
            },
            InlineAsmOperand::Out { expr, .. } => {
                never_loop_expr_all(cx, expr.iter().copied(), local_labels, main_loop_id)
            },
            InlineAsmOperand::SplitInOut { in_expr, out_expr, .. } => never_loop_expr_all(
                cx,
                once(*in_expr).chain(out_expr.iter().copied()),
                local_labels,
                main_loop_id,
            ),
            InlineAsmOperand::Const { .. } | InlineAsmOperand::SymFn { .. } | InlineAsmOperand::SymStatic { .. } => {
                NeverLoopResult::Normal
            },
            InlineAsmOperand::Label { block } =>
            // We do not know whether the label will be executed or not, so `Diverging` must be
            // downgraded to `Normal`.
            {
                match never_loop_block(cx, block, local_labels, main_loop_id) {
                    NeverLoopResult::Diverging { .. } => NeverLoopResult::Normal,
                    result => result,
                }
            },
        })),
        ExprKind::OffsetOf(_, _)
        | ExprKind::Yield(_, _)
        | ExprKind::Closure { .. }
        | ExprKind::Path(_)
        | ExprKind::ConstBlock(_)
        | ExprKind::Lit(_)
        | ExprKind::Err(_) => NeverLoopResult::Normal,
    };
    let result = combine_seq(result, || {
        if cx.typeck_results().expr_ty(expr).is_never() {
            NeverLoopResult::Diverging {
                break_spans: vec![],
                never_spans: all_spans_after_expr(cx, expr),
            }
        } else {
            NeverLoopResult::Normal
        }
    });
    result
}

fn never_loop_expr_all<'tcx, T: Iterator<Item = &'tcx Expr<'tcx>>>(
    cx: &LateContext<'tcx>,
    es: T,
    local_labels: &mut Vec<(HirId, bool)>,
    main_loop_id: HirId,
) -> NeverLoopResult {
    combine_seq_many(es.map(|e| never_loop_expr(cx, e, local_labels, main_loop_id)))
}

// Combine two results for parts that are called in order.
#[must_use]
fn combine_seq(first: NeverLoopResult, second: impl FnOnce() -> NeverLoopResult) -> NeverLoopResult {
    match first {
        NeverLoopResult::Diverging { .. } | NeverLoopResult::MayContinueMainLoop => first,
        NeverLoopResult::Normal => second(),
    }
}

// Combine an iterator of results for parts that are called in order.
#[must_use]
fn combine_seq_many(iter: impl IntoIterator<Item = NeverLoopResult>) -> NeverLoopResult {
    for e in iter {
        if let NeverLoopResult::Diverging { .. } | NeverLoopResult::MayContinueMainLoop = e {
            return e;
        }
    }
    NeverLoopResult::Normal
}

// Combine two results where only one of the part may have been executed.
#[must_use]
fn combine_branches(b1: NeverLoopResult, b2: NeverLoopResult) -> NeverLoopResult {
    match (b1, b2) {
        (NeverLoopResult::MayContinueMainLoop, _) | (_, NeverLoopResult::MayContinueMainLoop) => {
            NeverLoopResult::MayContinueMainLoop
        },
        (NeverLoopResult::Normal, _) | (_, NeverLoopResult::Normal) => NeverLoopResult::Normal,
        (
            NeverLoopResult::Diverging {
                break_spans: mut break_spans1,
                never_spans: mut never_spans1,
            },
            NeverLoopResult::Diverging {
                break_spans: mut break_spans2,
                never_spans: mut never_spans2,
            },
        ) => {
            break_spans1.append(&mut break_spans2);
            never_spans1.append(&mut never_spans2);
            NeverLoopResult::Diverging {
                break_spans: break_spans1,
                never_spans: never_spans1,
            }
        },
    }
}

#[must_use]
fn absorb_break(arg: &NeverLoopResult) -> NeverLoopResult {
    match arg {
        NeverLoopResult::Diverging { .. } | NeverLoopResult::Normal => NeverLoopResult::Normal,
        NeverLoopResult::MayContinueMainLoop => NeverLoopResult::MayContinueMainLoop,
    }
}

pub fn check_iterator_diverge<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
    if let ExprKind::MethodCall(method_name, _receiver, args, _) = expr.kind {
        if is_iterator_reduction_method(method_name.ident.name) {
            if let [arg] = args {
                if let ExprKind::Closure(closure) = arg.kind {
                    let mut local_labels = Vec::new();
                    let closure_body = cx.tcx.hir().body(closure.body);
                    let diverges_in_closure = never_loop_block(cx, closure_body.value, &mut local_labels, expr.hir_id);
                    
                    if let NeverLoopResult::Diverging { .. } = diverges_in_closure {
                        span_lint_and_then(
                            cx,
                            NEVER_ITER,
                            expr.span,
                            "this iterator method never processes more than the first element",
                            |diag| {
                                diag.span_help(
                                    expr.span,
                                    "this method will only process the first element due to divergence in the closure"
                                );
                            }
                        );
                    }
                }
            }
        }
    }
}

fn is_iterator_reduction_method(method_name: rustc_span::Symbol) -> bool {
    matches!(
        method_name,
        sym::for_each | sym::try_for_each | sym::fold | sym::reduce |
        sym::all | sym::any | sym::find | sym::find_map | sym::position | sym::rposition
    )
}