use std::fmt;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::get_attr;
use rustc_ast::{ast, token, tokenstream};
use rustc_data_structures::fx::{FxHashMap, FxHashSet, StdEntry};
use rustc_hir::{def, def_id, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{Span, Symbol};

// Future improvements:
//
// - Allow users to override modules as *not* having a specific danger.
// - Allow users to specify additional dangerous items in the clippy config.
// - Devise a scheme (maybe path compression?) to reduce the amount of ancestry tracing we have to
//   do to determine the dangers posed by a method.
// - Implement a way to forbid `accept_danger` in a given module.
// - Allow `accept_danger` and `dangerous` to be used as inner attributes on stable Rust.
//

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for uses of functions, inherent methods, and trait methods which have been marked as
    /// dangerous with the `#[clippy::dangerous(...)]` attribute and whose dangers have not been
    /// explicitly accepted.
    ///
    /// Each `#[clippy::dangerous(reason_1, reason_2, ...)]` attribute specifies a list of dangers
    /// that the user must accept using the `#[clippy::accept_danger(reason_1, reason_2, ...)]`
    /// attribute before using the dangerous item to avoid triggering this lint.
    ///
    /// ### Why is this bad?
    ///
    /// Some functionality in a project may be dangerous to use without giving it the appropriate
    /// caution, even if its misuse does not cause undefined behavior—for example, the method could
    /// be the source of tricky logic bugs. Other functionality may be dangerous in some contexts
    /// but not others. This lint helps ensure that users do not unknowingly call into these
    /// dangerous functions while still allowing users who know what they're doing to call these
    /// functions without issue.
    ///
    /// ### Example
    /// ```rust
    /// # fn main() {}  // Ensures that doc-test doesn't automatically wrap us in a main function.
    /// #[clippy::dangerous(use_of_lib_1_dangerous_module)]
    /// pub mod dangerous_module {
    ///     # fn break_the_program() {}
    ///     #[clippy::dangerous(may_break_program)]
    ///     pub fn do_something_innocuous_looking() {
    ///         break_the_program();
    ///     }
    /// }
    ///
    /// pub mod unsuspecting_module {
    ///    fn do_something() {
    ///        // This function call causes clippy to issue a warning
    ///        crate::dangerous_module::do_something_innocuous_looking();
    ///    }
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// # fn main() {}  // Ensures that doc-test doesn't automatically wrap us in a main function.
    /// #[clippy::dangerous(use_of_lib_1_dangerous_module)]
    /// pub mod dangerous_module {
    ///     # fn break_the_program() {}
    ///     #[clippy::dangerous(may_break_program)]
    ///     pub fn do_something_innocuous_looking() {
    ///         break_the_program();
    ///     }
    /// }
    ///
    /// // This entire module can use functions with the danger `use_of_lib_1_dangerous_module`.
    /// #[clippy::accept_danger(use_of_lib_1_dangerous_module)]
    /// pub mod unsuspecting_module {
    ///    fn do_something() {
    ///        // Only this statement can call functions with the danger `may_break_program`.
    ///        #[clippy::accept_danger(may_break_program)]
    ///        crate::dangerous_module::do_something_innocuous_looking();
    ///    }
    /// }
    /// ```
    #[clippy::version = "1.74.0"]
    pub DANGER_NOT_ACCEPTED,
    nursery,
    "checks for use of functions marked as dangerous"
}

#[derive(Default)]
pub struct DangerNotAccepted {
    accepted_dangers: FxHashMap<Symbol, u64>,
}

impl_lint_pass!(DangerNotAccepted => [DANGER_NOT_ACCEPTED]);

impl LateLintPass<'_> for DangerNotAccepted {
    #[allow(clippy::needless_return, reason = "unified syntax improves readability")]
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ Expr<'_>) {
        // If we're calling a method...
        if let ExprKind::MethodCall(_path, _, _self_arg, ..) = &expr.kind
            && let Some(fn_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
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
        // Both `accept_danger` and `dangerous` contribute to the accepted danger map.
        for attr_name in ["accept_danger", "dangerous"] {
            for attr in get_attr(cx.sess(), attrs, attr_name) {
                let dangers = parse_dangers_attr(cx, attr);
                for (_span, danger) in dangers {
                    *self.accepted_dangers.entry(danger).or_default() += 1;
                }
            }
        }
    }

    fn exit_lint_attrs(&mut self, cx: &LateContext<'_>, attrs: &'_ [rustc_ast::Attribute]) {
        // Both `accept_danger` and `dangerous` contribute to the accepted danger map.
        for attr_name in ["accept_danger", "dangerous"] {
            for attr in get_attr(cx.sess(), attrs, attr_name) {
                let dangers = parse_dangers_attr(cx, attr);
                for (_span, danger) in dangers {
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
}

impl DangerNotAccepted {
    fn get_unaccepted_dangers(&self, cx: &LateContext<'_>, item_id: def_id::DefId) -> Option<Vec<(Span, Symbol)>> {
        let mut unaccepted_dangers = Vec::new();
        let mut item_iter = Some(item_id);

        while let Some(item_id) = item_iter {
            item_iter = cx.tcx.opt_parent(item_id);

            // HACK: Ensure that this is not an intrinsic because calling `get_attrs_unchecked` on
            //  a foreign module breaks everything.
            if cx.tcx.def_kind(item_id) == def::DefKind::ForeignMod {
                continue;
            }

            for attr in get_attr(cx.sess(), cx.tcx.get_attrs_unchecked(item_id), "dangerous") {
                for danger in parse_dangers_attr(cx, attr) {
                    if self.accepted_dangers.contains_key(&danger.1) {
                        continue;
                    }

                    unaccepted_dangers.push(danger);
                }
            }
        }

        (!unaccepted_dangers.is_empty()).then_some(unaccepted_dangers)
    }
}

fn parse_dangers_attr(cx: &LateContext<'_>, attr: &ast::Attribute) -> Vec<(Span, Symbol)> {
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

        dangers.push((sym.span, sym.name));

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

fn emit_dangerous_call_lint(cx: &LateContext<'_>, expr: &'_ Expr<'_>, unaccepted_dangers: &[(Span, Symbol)]) {
    // Define formatting helpers
    struct FmtInline<F>(F);

    impl<F> fmt::Display for FmtInline<F>
    where
        F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.0(f)
        }
    }

    fn fmt_inline<F>(f: F) -> FmtInline<F>
    where
        F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
    {
        FmtInline(f)
    }

    // Collect all unique dangers
    let unique_dangers = unaccepted_dangers.iter().map(|(_, sym)| sym).collect::<FxHashSet<_>>();

    // Create a lint
    span_lint_and_then(
        cx,
        DANGER_NOT_ACCEPTED,
        expr.span,
        &format!(
            "called a function marked with `#[clippy::dangerous(...)]` without blessing the calling \
             module with `#![clippy::accept_danger({})]`",
            fmt_inline(|f| {
                let mut is_subsequent = false;
                for danger in &unique_dangers {
                    if is_subsequent {
                        f.write_str(", ")?;
                    }
                    is_subsequent = true;
                    f.write_str(danger.as_str())?;
                }
                Ok(())
            }),
        ),
        |diag| {
            for (danger_span, danger_name) in unaccepted_dangers {
                diag.span_note(*danger_span, format!("danger `{danger_name}` declared here"));
            }
        },
    );
}
