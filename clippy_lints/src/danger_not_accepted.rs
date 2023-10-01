use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_attr;
use rustc_ast::{ast, token, tokenstream};
use rustc_data_structures::fx::{FxHashMap, StdEntry};
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Symbol;

// TODO: Ensure that our attributes are being used properly
// TODO: Improve lint messages

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.74.0"]
    pub DANGER_NOT_ACCEPTED,
    nursery,
    "default lint description"
}

#[derive(Default)]
pub struct DangerNotAccepted {
    accepted_dangers: FxHashMap<Symbol, u64>,
}

impl_lint_pass!(DangerNotAccepted => [DANGER_NOT_ACCEPTED]);

impl LateLintPass<'_> for DangerNotAccepted {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        // If we're calling a method...
        if let ExprKind::MethodCall(path, _, _self_arg, ..) = &expr.kind
            && let def::Res::Def(def::DefKind::Fn, fn_id) = path.res
            // And that function is dangerous to us...
            && let Some(dangers) = self.get_unaccepted_dangers(cx, fn_id)
        {
            // Raise a lint
            emit_dangerous_call_lint(cx, expr, &dangers);
            return;
        }

        // If we're referencing a function...
        if let ExprKind::Path(path) = &expr.kind
            && let QPath::Resolved(_, path) = path
            && let def::Res::Def(def::DefKind::Fn, fn_id) = path.res
            // And that function is dangerous to us...
            && let Some(dangers) = self.get_unaccepted_dangers(cx, fn_id)
        {
            // Raise a lint
            emit_dangerous_call_lint(cx, expr, &dangers);
            return;
        }
    }

    fn enter_lint_attrs(&mut self, cx: &LateContext<'_>, attrs: &'_ [rustc_ast::Attribute]) {
        for attr in get_attr(cx.sess(), attrs, "accept_danger") {
            let dangers = parse_dangers_attr(cx, attr);
            for danger in dangers {
                *self.accepted_dangers.entry(danger).or_default() += 1;
            }
        }
    }

    fn exit_lint_attrs(&mut self, cx: &LateContext<'_>, attrs: &'_ [rustc_ast::Attribute]) {
        for attr in get_attr(cx.sess(), attrs, "accept_danger") {
            let dangers = parse_dangers_attr(cx, attr);
            for danger in dangers {
                match self.accepted_dangers.entry(danger) {
                    StdEntry::Occupied(mut entry) => {
                        *entry.get_mut() -= 1;
                        if *entry.get() == 0 {
                            entry.remove();
                        }
                    },
                    StdEntry::Vacant(_) => unreachable!(),
                }
            }
        }
    }
}

impl DangerNotAccepted {
    fn get_unaccepted_dangers(&self, cx: &LateContext<'_>, item_id: def_id::DefId) -> Option<Vec<Symbol>> {
        let mut unaccepted_dangers = Vec::new();
        let mut item_iter = Some(item_id);

        while let Some(item_id) = item_iter {
            for attr in cx.tcx.get_attrs_by_path(item_id, &[sym!(clippy), sym!(dangerous)]) {
                for danger in parse_dangers_attr(cx, attr) {
                    if self.accepted_dangers.contains_key(&danger) {
                        continue;
                    }

                    unaccepted_dangers.push(danger);
                }
            }

            item_iter = cx.tcx.opt_parent(item_id);
        }

        (!unaccepted_dangers.is_empty()).then_some(unaccepted_dangers)
    }
}

fn emit_dangerous_call_lint(cx: &LateContext<'_>, expr: &'_ Expr<'_>, dangers: &[Symbol]) {
    span_lint_and_help(
        cx,
        DANGER_NOT_ACCEPTED,
        expr.span,
        "Called a method marked with `#[clippy::dangerous]` without blessing the calling module with `#![clippy::accept_danger]`.",
        None,
        format!("This method poses the following unaccepted dangers: {dangers:?}").as_str(),
    );
}

fn parse_dangers_attr(cx: &LateContext<'_>, attr: &ast::Attribute) -> Vec<Symbol> {
    const EXPECTATION: &str = "Expected a delimited attribute with a list of danger identifiers.";

    let span = attr.span;

    let rustc_ast::AttrKind::Normal(attr) = &attr.kind else {
        cx.sess().span_err(span, EXPECTATION);
        return Vec::new();
    };

    let ast::AttrArgs::Delimited(attr) = &attr.item.args else {
        cx.sess().span_err(span, EXPECTATION);
        return Vec::new();
    };

    if attr.delim != token::Delimiter::Parenthesis {
        cx.sess().span_err(span, EXPECTATION);
        return Vec::new();
    }

    let mut stream = attr.tokens.trees();
    let mut dangers = Vec::new();

    loop {
        let sym = match stream.next() {
            Some(tokenstream::TokenTree::Token(sym, _)) => sym,
            None => break,
            _ => {
                cx.sess().span_err(span, EXPECTATION);
                return Vec::new();
            },
        };

        let Some((sym, _)) = sym.ident() else {
            cx.sess().span_err(span, EXPECTATION);
            return Vec::new();
        };

        dangers.push(sym.name);

        match stream.next() {
            Some(tokenstream::TokenTree::Token(sym, _)) if sym.kind == token::TokenKind::Comma => {
                continue;
            },
            None => {
                break;
            },
            _ => {
                cx.sess().span_err(span, EXPECTATION);
                return Vec::new();
            },
        }
    }

    dangers
}
