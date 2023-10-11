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
// - Allow users to override modules as *not* posing a specific danger.
// - Allow users to specify additional dangerous items in the clippy config.
// - Devise a scheme (maybe path compression?) to reduce the amount of ancestry tracing we have to
//   do to determine the dangers posed by a method.
// - Implement a way to forbid additional `accept_danger` calls in a given module.
// - Allow `accept_danger` and `dangerous` to be used as inner attributes on stable Rust. As
//   discussed, this would likely involve granting the clippy attribute namespace special status,
//   similar to `#[allow(...)]`.
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
                // We don't really care about why a user might have accepted a danger or marked a
                // section as dangerous.
                let (dangers, _ignored_reason) = parse_dangers_attr(cx, attr);

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
                let (dangers, _ignored_reason) = parse_dangers_attr(cx, attr);
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

struct UnacceptedDanger {
    span: Span,
    id: Symbol,
    reason: Option<token::Lit>,
}

impl DangerNotAccepted {
    fn get_unaccepted_dangers(&self, cx: &LateContext<'_>, item_id: def_id::DefId) -> Option<Vec<UnacceptedDanger>> {
        let mut unaccepted_dangers = Vec::new();
        let mut item_iter = Some(item_id);

        while let Some(item_id) = item_iter {
            item_iter = cx.tcx.opt_parent(item_id);

            // HACK: Ensure that this is not a foreign module because calling `get_attrs_unchecked` on
            //  an intrinsic foreign module breaks everything.
            if cx.tcx.def_kind(item_id) == def::DefKind::ForeignMod {
                continue;
            }

            for attr in get_attr(cx.sess(), cx.tcx.get_attrs_unchecked(item_id), "dangerous") {
                let (dangers, danger_reason) = parse_dangers_attr(cx, attr);
                for (danger_span, danger_id) in dangers {
                    if self.accepted_dangers.contains_key(&danger_id) {
                        continue;
                    }

                    unaccepted_dangers.push(UnacceptedDanger {
                        span: danger_span,
                        id: danger_id,
                        reason: danger_reason,
                    });
                }
            }
        }

        (!unaccepted_dangers.is_empty()).then_some(unaccepted_dangers)
    }
}

fn parse_dangers_attr(cx: &LateContext<'_>, attr: &ast::Attribute) -> (Vec<(Span, Symbol)>, Option<token::Lit>) {
    const EXPECTATION: &str = "expected a delimited attribute with a list of danger identifiers";
    const NOTHING_AFTER_ERR: &str = "nothing should come after the reason attribute besides an optional comma";

    let span = attr.span;
    let reason_sym = sym!(reason);

    // Expect a normal non doc-comment attribute.
    let rustc_ast::AttrKind::Normal(attr) = &attr.kind else {
        cx.sess().span_err(span, EXPECTATION);
        return (Vec::new(), None);
    };

    // Expect it to be a delimited attribute of the form #[attr(...)] and not #[attr {...}]
    let ast::AttrArgs::Delimited(attr) = &attr.item.args else {
        cx.sess().span_err(span, EXPECTATION);
        return (Vec::new(), None);
    };

    if attr.delim != token::Delimiter::Parenthesis {
        cx.sess().span_err(span, EXPECTATION);
        return (Vec::new(), None);
    }

    // Parse the attribute arguments
    let mut stream = attr.tokens.trees();
    let mut dangers = Vec::new();
    let mut specified_reason = None;

    loop {
        // Expect an identifier
        let sym = match stream.next() {
            Some(tokenstream::TokenTree::Token(sym, _)) if let Some((sym, _)) = sym.ident() => sym,
            // An EOS is also valid here.
            None => break,

            // Otherwise, raise an error.
            Some(tokenstream::TokenTree::Token(sym, _)) => {
                cx.sess().span_err(sym.span, format!("{EXPECTATION}; this was not an identifier"));

                return (Vec::new(), None);
            },
            _ => {
                cx.sess().span_err(span, EXPECTATION);
                return (Vec::new(), None);
            },
        };

        // If the identifier is not "reason", add it as a danger
        #[allow(clippy::if_not_else, reason = "it is clearer to put the common path first")]
        if sym.name != reason_sym {
            // Push it to the danger list
            dangers.push((sym.span, sym.name));

            // If we find a comma, continue. If we find an EOS for the inner stream, break.
            match stream.next() {
                // If we find an equality sign, continue.
                Some(tokenstream::TokenTree::Token(sym, _)) if sym.kind == token::TokenKind::Comma => {
                    continue;
                },
                None => break,
                // Otherwise, raise an error.
                Some(tokenstream::TokenTree::Token(sym, _)) => {
                    cx.sess()
                        .span_err(sym.span, format!("{EXPECTATION}; this was not a comma delimiter"));
                },
                _ => {
                    cx.sess().span_err(span, EXPECTATION);
                    return (Vec::new(), None);
                },
            }
        } else {
            // If the identifier was "reason", expect an equality sign.
            let eq_tok = match stream.next() {
                // If we find a comma, continue.
                Some(tokenstream::TokenTree::Token(tok, _)) if tok.kind == token::TokenKind::Eq => tok,
                // Otherwise, raise an error.
                Some(tokenstream::TokenTree::Token(tok, _)) => {
                    cx.sess().span_err(tok.span, "expected = after a reason attribute");
                    return (Vec::new(), None);
                },
                _ => {
                    cx.sess().span_err(sym.span, "expected = after a reason attribute");
                    return (Vec::new(), None);
                },
            };

            // ...then, expect a string literal.
            let (reason_tok, reason_lit) = match stream.next() {
                // If we find a string literal, continue.
                Some(tokenstream::TokenTree::Token(tok, _))
                if
                    let token::TokenKind::Literal(lit) = tok.kind &&
                    // We don't admit byte string literals because they wouldn't be `&str`s in regular
                    // scenarios so why treat them as strings here?
                    let token::LitKind::Str | token::LitKind::StrRaw(_) |
                        token::LitKind::CStr | token::LitKind::CStrRaw(_) = lit.kind
                => (tok, lit),
                // Otherwise, raise an error.
                Some(tokenstream::TokenTree::Token(tok, _)) => {
                    cx.sess().span_err(tok.span, "expected a string literal after a reason attribute");
                    return (Vec::new(), None);
                },
                _ => {
                    cx.sess().span_err(eq_tok.span, "expected a string literal after a reason attribute");
                    return (Vec::new(), None);
                },
            };

            specified_reason = Some(reason_lit);

            // Finally, because the reason must be specified as the last attribute, expect either an
            // optional comma or an EOS and break.
            match stream.next() {
                Some(tokenstream::TokenTree::Token(tok, _)) if tok.kind == token::TokenKind::Comma => {
                    // The token after the comma must be an EOS
                    if stream.next().is_some() {
                        cx.sess().span_err(tok.span, NOTHING_AFTER_ERR);
                    }
                },
                None => {},
                // Otherwise, raise an error.
                Some(tokenstream::TokenTree::Token(tok, _)) => {
                    cx.sess().span_err(tok.span, NOTHING_AFTER_ERR);
                    return (Vec::new(), None);
                },
                _ => {
                    cx.sess().span_err(reason_tok.span, NOTHING_AFTER_ERR);
                    return (Vec::new(), None);
                },
            }

            break;
        }
    }

    (dangers, specified_reason)
}

fn emit_dangerous_call_lint(cx: &LateContext<'_>, expr: &'_ Expr<'_>, unaccepted_dangers: &[UnacceptedDanger]) {
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
    let unique_dangers = unaccepted_dangers
        .iter()
        .map(|danger| danger.id)
        .collect::<FxHashSet<_>>();

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
            for danger in unaccepted_dangers {
                if let Some(reason) = danger.reason {
                    diag.span_note(
                        danger.span,
                        format!(
                            "danger `{}` declared here with the justification `{}`",
                            danger.id,
                            reason.symbol.as_str(),
                        ),
                    );
                } else {
                    diag.span_note(danger.span, format!("danger `{}` declared here", danger.id));
                }
            }
        },
    );
}
