use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::get_attr;
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
        let mut inc = |id| *self.accepted_dangers.entry(id).or_default() += 1;

        for attr in get_attr(cx.sess(), attrs, "accept_danger") {
            for (_span, danger) in parsing::parse_single_reason_danger_list_attr(cx, attr).0 {
                inc(danger);
            }
        }

        for attr in get_attr(cx.sess(), attrs, "dangerous") {
            for (_span, danger, _ignored_reason) in parsing::parse_individually_reasoned_danger_list_attr(cx, attr) {
                inc(danger);
            }
        }
    }

    fn exit_lint_attrs(&mut self, cx: &LateContext<'_>, attrs: &'_ [rustc_ast::Attribute]) {
        // Both `accept_danger` and `dangerous` contribute to the accepted danger map.
        let mut dec = |id| match self.accepted_dangers.entry(id) {
            StdEntry::Occupied(mut entry) => {
                *entry.get_mut() -= 1;
                if *entry.get() == 0 {
                    entry.remove();
                }
            },
            StdEntry::Vacant(_) => unreachable!(),
        };

        for attr in get_attr(cx.sess(), attrs, "accept_danger") {
            for (_span, danger) in parsing::parse_single_reason_danger_list_attr(cx, attr).0 {
                dec(danger);
            }
        }

        for attr in get_attr(cx.sess(), attrs, "dangerous") {
            for (_span, danger, _ignored_reason) in parsing::parse_individually_reasoned_danger_list_attr(cx, attr) {
                dec(danger);
            }
        }
    }
}

struct UnacceptedDanger {
    span: Span,
    id: Symbol,
    reason: Symbol,
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
                for (danger_span, danger_id, danger_reason) in
                    parsing::parse_individually_reasoned_danger_list_attr(cx, attr)
                {
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

fn emit_dangerous_call_lint(cx: &LateContext<'_>, expr: &'_ Expr<'_>, unaccepted_dangers: &[UnacceptedDanger]) {
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
            {
                let mut danger_list = unique_dangers.iter().map(Symbol::as_str).collect::<Vec<_>>();
                danger_list.sort_unstable();
                danger_list.join(", ")
            },
        ),
        |diag| {
            for danger in unaccepted_dangers {
                diag.span_note(
                    danger.span,
                    format!(
                        "danger `{}` declared here with the justification `{}`",
                        danger.id,
                        danger.reason.as_str(),
                    ),
                );
            }
        },
    );
}

// === Parsing === //

// I had a feeling this is going to change a lot so I built some actual parser infrastructure...
mod parsing {
    use rustc_ast::ast::Attribute;
    use rustc_ast::token::{Delimiter, Lit, LitKind, Token, TokenKind};
    use rustc_ast::tokenstream::{DelimSpan, RefTokenTreeCursor, TokenStream, TokenTree};
    use rustc_ast::{AttrArgs, AttrKind};
    use rustc_data_structures::fx::FxHashSet;
    use rustc_errors::DiagnosticMessage;
    use rustc_lint::{LateContext, LintContext};
    use rustc_session::Session;
    use rustc_span::{sym, Span, Symbol};
    use std::cell::{Cell, RefCell};

    const RESERVED_PREFIXES: [&str; 9] = [
        "rust", "rustc", "clippy", "core", "std", "common", "mem", "race", "sync",
    ];

    const RESERVED_DANGERS: [&str; 9] = [
        "reason",
        "justification",
        "cfg",
        "edition",
        "version",
        "since",
        "author",
        "history",
        "panics",
    ];

    // === Core === //

    // LookaheadResult
    trait LookaheadResult {
        fn is_truthy(&self) -> bool;
    }

    impl LookaheadResult for bool {
        fn is_truthy(&self) -> bool {
            *self
        }
    }

    impl<T> LookaheadResult for Option<T> {
        fn is_truthy(&self) -> bool {
            self.is_some()
        }
    }

    impl<T, E> LookaheadResult for Result<T, E> {
        fn is_truthy(&self) -> bool {
            self.is_ok()
        }
    }

    // ParseContext
    struct ParseContext<'s> {
        rustc_session: &'s Session,
        while_parsing: RefCell<Vec<Symbol>>,
        got_stuck: Cell<bool>,
    }

    #[must_use]
    struct WhileParsingGuard<'c> {
        cx: &'c ParseContext<'c>,
        top: Symbol,
    }

    impl Drop for WhileParsingGuard<'_> {
        fn drop(&mut self) {
            let popped = self.cx.while_parsing.borrow_mut().pop();
            debug_assert_eq!(popped, Some(self.top));
        }
    }

    impl<'s> ParseContext<'s> {
        fn new(rustc_session: &'s Session) -> Self {
            Self {
                rustc_session,
                while_parsing: RefCell::new(Vec::new()),
                got_stuck: Cell::new(false),
            }
        }

        fn enter<'c, 't>(&'c self, span: DelimSpan, stream: &'t TokenStream) -> ParseSequence<'c, 't> {
            ParseSequence {
                context: self,
                cursor: ParseCursor {
                    raw: stream.trees(),
                    span,
                },
                expectations: Vec::new(),
            }
        }

        fn while_parsing(&self, what: Symbol) -> WhileParsingGuard<'_> {
            self.while_parsing.borrow_mut().push(what);

            WhileParsingGuard { cx: self, top: what }
        }

        fn got_stuck(&self) -> bool {
            self.got_stuck.get()
        }
    }

    // ParseSequence
    struct ParseSequence<'c, 't> {
        context: &'c ParseContext<'c>,
        cursor: ParseCursor<'t>,
        expectations: Vec<Symbol>,
    }

    impl<'c, 't> ParseSequence<'c, 't> {
        // fn enter<'t2>(&self, span: DelimSpan, stream: &'t2 TokenStream) -> ParseSequence<'c, 't2> {
        //     self.context.enter(span, stream)
        // }

        fn while_parsing(&self, what: Symbol) -> WhileParsingGuard<'c> {
            self.context.while_parsing(what)
        }

        fn expect<R: LookaheadResult>(&mut self, expectation: Symbol, f: impl FnOnce(&mut ParseCursor<'t>) -> R) -> R {
            let res = self.cursor.lookahead(|c| f(c));
            if res.is_truthy() {
                self.expectations.clear();
            } else {
                self.expectations.push(expectation);
            }
            res
        }

        fn stuck(&mut self, recover: impl FnOnce(&mut ParseCursor<'t>)) {
            // Mark that we got stuck
            self.context.got_stuck.set(true);

            // Emit the error message
            let span = self.cursor.next_span();

            let expectations = self.expectations.iter().copied().collect::<FxHashSet<_>>();
            let mut expectations = expectations.iter().map(Symbol::as_str).collect::<Vec<_>>();
            expectations.sort_unstable();

            let expectations = expectations.join(", ");

            let while_parsing = {
                let stack = self.context.while_parsing.borrow();
                if stack.is_empty() {
                    String::new()
                } else {
                    format!(
                        " while parsing {}",
                        stack.iter().rev().map(Symbol::as_str).collect::<Vec<_>>().join(" in ")
                    )
                }
            };

            self.rustc_session()
                .span_err(span, format!("expected {expectations}{while_parsing}"));

            // Attempt to get unstuck
            recover(&mut self.cursor);
        }

        fn error(&mut self, sp: Span, msg: impl Into<DiagnosticMessage>, recover: impl FnOnce(&mut ParseCursor<'t>)) {
            self.context.got_stuck.set(true);
            self.rustc_session().span_err(sp, msg);
            recover(&mut self.cursor);
        }

        fn next_span(&self) -> Span {
            self.cursor.next_span()
        }

        fn rustc_session(&self) -> &'c Session {
            self.context.rustc_session
        }
    }

    // ParseCursor
    #[derive(Clone)]
    struct ParseCursor<'t> {
        raw: RefTokenTreeCursor<'t>,
        span: DelimSpan,
    }

    impl<'t> ParseCursor<'t> {
        fn lookahead<R: LookaheadResult>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
            let mut fork = self.clone();
            let res = f(&mut fork);
            if res.is_truthy() {
                *self = fork;
            }
            res
        }

        fn consume(&mut self) -> Option<&'t TokenTree> {
            self.raw.next()
        }

        fn peek(&self) -> Option<&'t TokenTree> {
            self.raw.clone().next()
        }

        fn next_span(&self) -> Span {
            self.peek().map_or(self.span.close, TokenTree::span)
        }
    }

    // === Helpers === //

    fn parse_eos(c: &mut ParseCursor<'_>) -> bool {
        c.lookahead(|c| c.consume().is_none())
    }

    fn parse_turbo(c: &mut ParseCursor<'_>) -> Option<Span> {
        c.lookahead(|c| {
            if let Some(TokenTree::Token(
                Token {
                    kind: TokenKind::ModSep,
                    span,
                },
                _,
            )) = c.consume()
            {
                Some(*span)
            } else {
                None
            }
        })
    }

    fn parse_ident(c: &mut ParseCursor<'_>) -> Option<(Symbol, bool)> {
        c.lookahead(|c| {
            if let Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Ident(sym, raw),
                    ..
                },
                _,
            )) = c.consume()
            {
                Some((*sym, *raw))
            } else {
                None
            }
        })
    }

    fn parse_comma(c: &mut ParseCursor<'_>) -> Option<Span> {
        c.lookahead(|c| {
            if let Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Comma,
                    span,
                },
                _,
            )) = c.consume()
            {
                Some(*span)
            } else {
                None
            }
        })
    }

    fn parse_equals(c: &mut ParseCursor<'_>) -> Option<Span> {
        c.lookahead(|c| {
            if let Some(TokenTree::Token(
                Token {
                    kind: TokenKind::Eq,
                    span,
                },
                _,
            )) = c.consume()
            {
                Some(*span)
            } else {
                None
            }
        })
    }

    fn parse_str_lit(c: &mut ParseCursor<'_>) -> Option<Symbol> {
        c.lookahead(|c| {
            if let Some(TokenTree::Token(
                Token {
                    kind:
                        TokenKind::Literal(Lit {
                            symbol,
                            kind: LitKind::Str | LitKind::StrRaw(_),
                            ..
                        }),
                    ..
                },
                _,
            )) = c.consume()
            {
                Some(*symbol)
            } else {
                None
            }
        })
    }

    fn skip_until_before_next_comma_or_eos(c: &mut ParseCursor<'_>) {
        while !parse_eos(c) && parse_comma(&mut c.clone()).is_none() {
            c.consume();
        }
    }

    // === Grammar === //

    fn parse_path(s: &mut ParseSequence<'_, '_>) -> Option<Symbol> {
        let _guard = s.while_parsing(Symbol::intern("a path"));
        let mut is_subsequent = false;
        let mut builder = String::new();
        let start = s.next_span();

        loop {
            // Parse turbo delimiter
            if is_subsequent {
                if s.expect(Symbol::intern("`::`"), parse_turbo).is_none() {
                    // If we don't have one, assume that the path is done.

                    // ...but first, we need to validate the identifier.
                    if let Some(reserved) = RESERVED_DANGERS.iter().find(|v| **v == builder) {
                        s.error(
                            start.until(s.next_span()),
                            format!("`{reserved}` cannot be the name of a danger"),
                            skip_until_before_next_comma_or_eos,
                        );
                        return None;
                    }

                    // N.B. the fact that this can only happen if we attempt to parse a subsequent
                    // identifier ensures that we can't just build a path out of nothing.
                    return Some(Symbol::intern(&builder));
                }

                builder.push_str("::");
            }

            // Parse identifier
            let sess = s.rustc_session();
            let Some((ident, _)) = s.expect(Symbol::intern("<identifier>"), |c| {
                parse_ident(c).filter(|(ident, _)| !ident.is_reserved(|| sess.edition()))
            }) else {
                // Whoops! This is malformed.
                s.stuck(skip_until_before_next_comma_or_eos);
                return None;
            };

            // Ensure that this isn't a reserved prefix
            if !is_subsequent {
                if let Some(reserved) = RESERVED_PREFIXES.iter().find(|v| &***v == ident.as_str()) {
                    s.error(
                        start.until(s.next_span()),
                        format!("`{reserved}` is a reserved danger prefix"),
                        skip_until_before_next_comma_or_eos,
                    );
                }
            }

            builder.push_str(ident.as_str());

            is_subsequent = true;
        }
    }

    fn parse_individually_reasoned_danger_list(s: &mut ParseSequence<'_, '_>) -> Vec<(Span, Symbol, Symbol)> {
        let _guard = s.while_parsing(Symbol::intern("the dangers list"));

        let mut dangers = Vec::new();
        let mut is_subsequent = false;

        loop {
            // Handle EOS
            if s.expect(Symbol::intern("`)`"), parse_eos) {
                break;
            }

            // Handle comma if necessary
            if is_subsequent && s.expect(Symbol::intern("`,`"), parse_comma).is_none() {
                s.stuck(skip_until_before_next_comma_or_eos);
                continue;
            }

            // Handle another EOS because we don't want to get stuck in `parse_path`, which treats
            // empty paths as errors.
            if s.expect(Symbol::intern("`)`"), parse_eos) {
                break;
            }

            let danger_start = s.next_span();

            // Handle a non-empty path.
            let Some(danger) = parse_path(s) else {
                // Our recovery routine has already put us into the position of parsing the next
                // danger.
                is_subsequent = true;
                continue;
            };

            // Handle the reason.
            let reason = {
                let _guard = s.while_parsing(Symbol::intern("the danger's reason string"));

                if s.expect(Symbol::intern("`=`"), parse_equals).is_none() {
                    s.stuck(skip_until_before_next_comma_or_eos);
                    continue;
                }

                let Some(reason) = s.expect(Symbol::intern("a reason string"), parse_str_lit) else {
                    s.stuck(skip_until_before_next_comma_or_eos);
                    continue;
                };
                reason
            };

            dangers.push((danger_start.until(s.next_span()), danger, reason));
            is_subsequent = true;
        }

        dangers
    }

    fn parse_single_reason_danger_list(s: &mut ParseSequence<'_, '_>) -> (Vec<(Span, Symbol)>, Option<Symbol>) {
        let _guard = s.while_parsing(Symbol::intern("the dangers list"));

        let mut dangers = Vec::new();
        let mut is_subsequent = false;

        loop {
            // Handle EOS
            if s.expect(Symbol::intern("`)`"), parse_eos) {
                return (dangers, None);
            }

            // Handle comma if necessary
            if is_subsequent && s.expect(Symbol::intern("`,`"), parse_comma).is_none() {
                s.stuck(skip_until_before_next_comma_or_eos);
                continue;
            }

            // Handle another EOS because we don't want to get stuck in `parse_path`, which treats
            // empty paths as errors.
            if s.expect(Symbol::intern("`)`"), parse_eos) {
                return (dangers, None);
            }

            // Handle `reason = "text"` syntax
            if s.expect(Symbol::intern("`reason`"), |c| {
                parse_ident(c).filter(|(s, _)| *s == sym::reason)
            })
            .is_some()
            {
                let _guard = s.while_parsing(Symbol::intern("the reason attribute"));

                // Expect `=`
                if s.expect(Symbol::intern("`=`"), parse_equals).is_none() {
                    s.stuck(skip_until_before_next_comma_or_eos);
                    continue;
                }

                // Expect a reason literal
                let Some(reason) = s.expect(Symbol::intern("<reason string>"), parse_str_lit) else {
                    s.stuck(skip_until_before_next_comma_or_eos);
                    continue;
                };

                // Allow an optional `,`
                let _guard = s.expect(Symbol::intern("`,`"), parse_comma);

                // Expect an EOS
                if !s.expect(Symbol::intern("`)`"), parse_eos) {
                    s.stuck(skip_until_before_next_comma_or_eos);
                    continue;
                }

                return (dangers, Some(reason));
            };

            let danger_start = s.next_span();

            // Handle a non-empty path.
            let Some(danger) = parse_path(s) else {
                // Our recovery routine has already put us into the position of parsing the next
                // danger.
                is_subsequent = true;
                continue;
            };

            dangers.push((danger_start.until(s.next_span()), danger));
            is_subsequent = true;
        }
    }

    // === Drivers === //

    fn parse_paren_attr<R: Default>(
        cx: &LateContext<'_>,
        attr: &Attribute,
        f: impl FnOnce(&mut ParseSequence<'_, '_>) -> R,
    ) -> R {
        const EXPECTATION: &str = "expected a delimited attribute with a list of danger identifiers";

        let span = attr.span;

        // Expect a normal non doc-comment attribute.
        let AttrKind::Normal(attr) = &attr.kind else {
            cx.sess().span_err(span, EXPECTATION);
            return R::default();
        };

        // Expect it to be a delimited attribute of the form #[attr(...)] and not #[attr {...}]
        let AttrArgs::Delimited(attr) = &attr.item.args else {
            cx.sess().span_err(span, EXPECTATION);
            return R::default();
        };

        if attr.delim != Delimiter::Parenthesis {
            cx.sess().span_err(span, EXPECTATION);
            return R::default();
        }

        // Parse the attribute arguments
        let cx = ParseContext::new(cx.sess());
        let res = f(&mut cx.enter(attr.dspan, &attr.tokens));
        if cx.got_stuck() { R::default() } else { res }
    }

    pub fn parse_individually_reasoned_danger_list_attr(
        cx: &LateContext<'_>,
        attr: &Attribute,
    ) -> Vec<(Span, Symbol, Symbol)> {
        parse_paren_attr(cx, attr, parse_individually_reasoned_danger_list)
    }

    pub fn parse_single_reason_danger_list_attr(
        cx: &LateContext<'_>,
        attr: &Attribute,
    ) -> (Vec<(Span, Symbol)>, Option<Symbol>) {
        parse_paren_attr(cx, attr, parse_single_reason_danger_list)
    }
}
