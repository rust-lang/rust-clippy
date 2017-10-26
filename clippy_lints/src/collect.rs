use itertools::{repeat_n, Itertools};
use rustc::hir::*;
use rustc::lint::*;
use rustc::ty::TypeVariants;
use syntax::ast::NodeId;

use std::collections::HashSet;

use crate::utils::{match_trait_method, match_type, span_lint_and_sugg};
use crate::utils::paths;

/// **What it does:** Detects collect calls on iterators to collections
/// of either `Result<_, E>` or `Option<_>` inside functions that also
/// have such a return type.
///
/// **Why is this bad?** It is possible to short-circuit these collect
/// calls and return early whenever a `None` or `Err(E)` is encountered.
///
/// **Known problems:** It may be possible that a collection of options
/// or results is intended. This would then generate a false positive.
///
/// **Example:**
/// ```rust
/// pub fn div(a: i32, b: &[i32]) -> Result<Vec<i32>, String> {
///     let option_vec: Vec<_> = b.into_iter()
///         .cloned()
///         .map(|i| if i != 0 {
///             Ok(a / i)
///         } else {
///             Err("Division by zero!".to_owned())
///         })
///         .collect();
///     let mut int_vec = Vec::new();
///     for opt in option_vec {
///         int_vec.push(opt?);
///     }
///     Ok(int_vec)
/// }
/// ```
declare_clippy_lint! {
    pub POSSIBLE_SHORTCIRCUITING_COLLECT,
    nursery,
    "missed shortcircuit opportunity on collect"
}

#[derive(Clone)]
pub struct Pass {
    // To ensure that we do not lint the same expression more than once
    seen_expr_nodes: HashSet<NodeId>,
}

impl Pass {
    pub fn new() -> Self {
        Self { seen_expr_nodes: HashSet::new() }
    }
}

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(POSSIBLE_SHORTCIRCUITING_COLLECT)
    }
}

struct Suggestion {
    pattern: String,
    type_colloquial: &'static str,
    success_variant: &'static str,
}

fn format_suggestion_pattern<'a, 'tcx>(
    cx: &LateContext<'a, 'tcx>,
    collection_ty: TypeVariants,
    is_option: bool,
) -> String {
    let collection_pat = match collection_ty {
        TypeVariants::TyAdt(def, subs) => {
            let mut buf = cx.tcx.item_path_str(def.did);

            if !subs.is_empty() {
                buf.push('<');
                buf.push_str(&repeat_n('_', subs.len()).join(", "));
                buf.push('>');
            }

            buf
        },
        TypeVariants::TyParam(p) => p.to_string(),
        _ => "_".into(),
    };

    if is_option {
        format!("Option<{}>", collection_pat)
    } else {
        format!("Result<{}, _>", collection_pat)
    }
}

fn check_expr_for_collect<'a, 'tcx>(cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) -> Option<Suggestion> {
    if let ExprMethodCall(ref method, _, ref args) = expr.node {
        if args.len() == 1 && method.name == "collect" && match_trait_method(cx, expr, &paths::ITERATOR) {
            let collect_ty = cx.tables.expr_ty(expr);

            if match_type(cx, collect_ty, &paths::OPTION) || match_type(cx, collect_ty, &paths::RESULT) {
                // Already collecting into an Option or Result - good!
                return None;
            }

            // Get the type of the Item associated to the Iterator on which collect() is
            // called.
            let arg_ty = cx.tables.expr_ty(&args[0]);
            let method_call = cx.tables.type_dependent_defs()[args[0].hir_id];
            let trt_id = cx.tcx.trait_of_item(method_call.def_id()).unwrap();
            let assoc_item_id = cx.tcx.associated_items(trt_id).next().unwrap().def_id;
            let substitutions = cx.tcx.mk_substs_trait(arg_ty, &[]);
            let projection = cx.tcx.mk_projection(assoc_item_id, substitutions);
            let normal_ty = cx.tcx.normalize_erasing_regions(
                cx.param_env,
                projection,
            );

            return if match_type(cx, normal_ty, &paths::OPTION) {
                Some(Suggestion {
                    pattern: format_suggestion_pattern(cx, collect_ty.sty.clone(), true),
                    type_colloquial: "Option",
                    success_variant: "Some",
                })
            } else if match_type(cx, normal_ty, &paths::RESULT) {
                Some(Suggestion {
                    pattern: format_suggestion_pattern(cx, collect_ty.sty.clone(), false),
                    type_colloquial: "Result",
                    success_variant: "Ok",
                })
            } else {
                None
            };
        }
    }

    None
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for Pass {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        if self.seen_expr_nodes.contains(&expr.id) {
            return;
        }

        if let Some(suggestion) = check_expr_for_collect(cx, expr) {
            let sugg_span = if let ExprMethodCall(_, call_span, _) = expr.node {
                expr.span.between(call_span)
            } else {
                unreachable!()
            };

            span_lint_and_sugg(
                cx,
                POSSIBLE_SHORTCIRCUITING_COLLECT,
                sugg_span,
                &format!("you are creating a collection of `{}`s", suggestion.type_colloquial),
                &format!(
                    "if you are only interested in the case where all values are `{}`, try",
                    suggestion.success_variant
                ),
                format!("collect::<{}>()", suggestion.pattern),
            );
        }
    }

    fn check_stmt(&mut self, cx: &LateContext<'a, 'tcx>, stmt: &'tcx Stmt) {
        if_chain! {
            if let StmtDecl(ref decl, _) = stmt.node;
            if let DeclLocal(ref local) = decl.node;
            if let Some(ref ty) = local.ty;
            if let Some(ref expr) = local.init;
            then {
                self.seen_expr_nodes.insert(expr.id);

                if let Some(suggestion) = check_expr_for_collect(cx, expr) {
                    span_lint_and_sugg(
                        cx,
                        POSSIBLE_SHORTCIRCUITING_COLLECT,
                        ty.span,
                        &format!("you are creating a collection of `{}`s", suggestion.type_colloquial),
                        &format!(
                            "if you are only interested in the case where all values are `{}`, try",
                            suggestion.success_variant
                        ),
                        suggestion.pattern
                    );
                }
            }
        }
    }
}
