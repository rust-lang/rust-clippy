use clippy_utils::{
    diagnostics::span_lint_and_sugg, peel_blocks, peel_blocks_with_stmt, sugg::Sugg, ty::is_type_diagnostic_item,
};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `option.map(_).unwrap_or(_)` or `option.map(_).unwrap_or_else(_)` or
    /// `result.map(_).unwrap_or_else(_)`.
    /// ### Why is this bad?
    /// Readability, these can be written more concisely (resp.) as
    /// `option.map_or(_, _)`, `option.map_or_else(_, _)` and `result.map_or_else(_, _)`.
    ///
    /// ### Example
    /// ```rust, ignore
    /// # let option = Some(1);
    /// # let result: Result<usize, ()> = Ok(1);
    /// # fn some_function(foo: ()) -> usize { 1 }
    /// option.map(|a| a + 1).unwrap_or(0);
    /// option.map(|a| a > 5).unwrap_or(false);
    /// result.map(|a| a + 1).unwrap_or_else(some_function);
    /// ```
    /// Use instead:
    /// ```rust, ignore
    /// # let option = Some(1);
    /// # let result: Result<usize, ()> = Ok(1);
    /// # fn some_function(foo: ()) -> usize { 1 }
    /// option.map_or(0, |a| a + 1);
    /// option.is_some_and(|a| a > 5);
    /// result.map_or_else(some_function, |a| a + 1);
    /// ```
    #[clippy::version = "1.71.0"]
    pub OPTION_IS_SOME_WITH,
    complexity,
    "default lint description"
}
declare_lint_pass!(OptionIsSomeWith => [OPTION_IS_SOME_WITH]);

struct OptionMapOccurence {
    variable: String,
    ty_variable: String,
    suggest: String,
    declare_expr: String,
    some_expr: String,
    method_call: String,
}

impl LateLintPass<'_> for OptionIsSomeWith {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        let detection = detect_option_map_and_unwrap_or(cx, expr);
        if let Some(det) = detection {
            span_lint_and_sugg(
                cx,
                OPTION_IS_SOME_WITH,
                expr.span,
                format!(
                    "called `map(<f>).{}(<g>)` on an `{}` value. This can be done more directly by calling `{}({}<f>)` instead",
                    det.method_call, det.ty_variable, det.suggest,
                    format!("{}", if (det.suggest == "and_then") || (det.suggest == "is_some_and") {""} else {"<g>, "}),
            ).as_str(),
                "try",
                format!(
                    "{}.{}({}{})",
                    det.variable, det.suggest, if (det.suggest == "is_some_and") || (det.suggest == "and_then") { "".to_owned() } else { det.declare_expr + ", " }, 
                    det.some_expr
                ),
                Applicability::MaybeIncorrect,
            );
        }
    }
}

fn detect_option_map_and_unwrap_or<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Option<OptionMapOccurence> {
    if let Some((name, recv, args, _, _)) = method_call(expr) {
        match (name, args) {
            ("unwrap_or" | "unwrap_or_else", [unwrap_arg]) => match method_call(recv) {
                Some(("map", map_recv, [map_arg], _, _)) => {
                    if_chain! {
                        let ident = get_inner_pat(map_recv)?;
                        let is_option = is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv), sym::Option);

                        then {
                            let mut app = Applicability::Unspecified;
                            let method_sugg = if name == "unwrap_or_else" {
                                "map_or_else"
                            } else {
                                if let Some(ty) = get_ty(peel_blocks(unwrap_arg)) {
                                    ty
                                } else {"and_then"}
                            };
                            return Some(OptionMapOccurence {
                                variable: ident.to_string(),
                                ty_variable: if is_option {"Option".to_owned()} else {"Result".to_owned()},
                                suggest: method_sugg.to_string(),
                                some_expr: format!(
                                    "{}",
                                    Sugg::hir_with_context(cx, peel_blocks_with_stmt(map_arg), expr.span.ctxt(), "..", &mut app)
                                ),
                                declare_expr: format!(
                                    "{}",
                                    Sugg::hir_with_context(cx, peel_blocks_with_stmt(unwrap_arg), expr.span.ctxt(), "..", &mut app)
                                ),
                                method_call: name.to_owned(),
                            });
                        }
                    }
                },
                _ => return None,
            },
            _ => return None,
        }
    }

    None
}

fn get_ty<'a>(arg: &Expr<'_>) -> Option<&'a str> {
    if let ExprKind::Lit(Lit {node, ..}) = arg.kind && !arg.span.from_expansion() {
        return match node {
            LitKind::Bool(_) => Some("is_some_and"),
            LitKind::Err => None,
            _ => Some("map_or"),
        }
    } else {
        None
    }
}

fn method_call<'tcx>(recv: &'tcx Expr<'tcx>) -> Option<(&'tcx str, &'tcx Expr<'tcx>, &'tcx [Expr<'tcx>], Span, Span)> {
    if let ExprKind::MethodCall(path, receiver, args, call_span) = recv.kind {
        if !args.iter().any(|e| e.span.from_expansion()) && !receiver.span.from_expansion() {
            let name = path.ident.name.as_str();
            return Some((name, receiver, args, path.ident.span, call_span));
        }
    }
    None
}

fn get_inner_pat<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx rustc_span::symbol::Ident> {
    if let ExprKind::Path(QPath::Resolved(
        _,
        Path {
            segments: [PathSegment { ident, .. }],
            ..
        },
    )) = &expr.kind
    {
        return Some(ident);
    }

    None
}
