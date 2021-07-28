use std::borrow::Cow;
use std::iter;
use std::ops::{Deref, Range};

use clippy_utils::diagnostics::{span_lint, span_lint_and_sugg, span_lint_and_then};
use clippy_utils::source::{snippet_opt, snippet_with_applicability};
use rustc_ast::ast::{Expr, ExprKind, ImplKind, Item, ItemKind, MacCall, Path, StrLit, StrStyle};
use rustc_ast::token::{self, LitKind};
use rustc_ast::tokenstream::TokenStream;
use rustc_errors::Applicability;
use rustc_lexer::unescape::{self, EscapeError};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_parse::parser;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::symbol::{kw, Symbol};
use rustc_span::{sym, BytePos, Span, DUMMY_SP};

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `println!("")` to
    /// print a newline.
    ///
    /// ### Why is this bad?
    /// You should use `println!()`, which is simpler.
    ///
    /// ### Example
    /// ```rust
    /// // Bad
    /// println!("");
    ///
    /// // Good
    /// println!();
    /// ```
    pub PRINTLN_EMPTY_STRING,
    style,
    "using `println!(\"\")` with an empty string"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `print!()` with a format
    /// string that ends in a newline.
    ///
    /// ### Why is this bad?
    /// You should use `println!()` instead, which appends the
    /// newline.
    ///
    /// ### Example
    /// ```rust
    /// # let name = "World";
    /// print!("Hello {}!\n", name);
    /// ```
    /// use println!() instead
    /// ```rust
    /// # let name = "World";
    /// println!("Hello {}!", name);
    /// ```
    pub PRINT_WITH_NEWLINE,
    style,
    "using `print!()` with a format string that ends in a single newline"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for printing on *stdout*. The purpose of this lint
    /// is to catch debugging remnants.
    ///
    /// ### Why is this bad?
    /// People often print on *stdout* while debugging an
    /// application and might forget to remove those prints afterward.
    ///
    /// ### Known problems
    /// Only catches `print!` and `println!` calls.
    ///
    /// ### Example
    /// ```rust
    /// println!("Hello world!");
    /// ```
    pub PRINT_STDOUT,
    restriction,
    "printing on stdout"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for printing on *stderr*. The purpose of this lint
    /// is to catch debugging remnants.
    ///
    /// ### Why is this bad?
    /// People often print on *stderr* while debugging an
    /// application and might forget to remove those prints afterward.
    ///
    /// ### Known problems
    /// Only catches `eprint!` and `eprintln!` calls.
    ///
    /// ### Example
    /// ```rust
    /// eprintln!("Hello world!");
    /// ```
    pub PRINT_STDERR,
    restriction,
    "printing on stderr"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for use of `Debug` formatting. The purpose of this
    /// lint is to catch debugging remnants.
    ///
    /// ### Why is this bad?
    /// The purpose of the `Debug` trait is to facilitate
    /// debugging Rust code. It should not be used in user-facing output.
    ///
    /// ### Example
    /// ```rust
    /// # let foo = "bar";
    /// println!("{:?}", foo);
    /// ```
    pub USE_DEBUG,
    restriction,
    "use of `Debug`-based formatting"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns about the use of literals as `print!`/`println!` args.
    ///
    /// ### Why is this bad?
    /// Using literals as `println!` args is inefficient
    /// (c.f., https://github.com/matthiaskrgr/rust-str-bench) and unnecessary
    /// (i.e., just put the literal in the format string)
    ///
    /// ### Known problems
    /// Will also warn with macro calls as arguments that expand to literals
    /// -- e.g., `println!("{}", env!("FOO"))`.
    ///
    /// ### Example
    /// ```rust
    /// println!("{}", "foo");
    /// ```
    /// use the literal without formatting:
    /// ```rust
    /// println!("foo");
    /// ```
    pub PRINT_LITERAL,
    style,
    "printing a literal with a format string"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `writeln!(buf, "")` to
    /// print a newline.
    ///
    /// ### Why is this bad?
    /// You should use `writeln!(buf)`, which is simpler.
    ///
    /// ### Example
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// // Bad
    /// writeln!(buf, "");
    ///
    /// // Good
    /// writeln!(buf);
    /// ```
    pub WRITELN_EMPTY_STRING,
    style,
    "using `writeln!(buf, \"\")` with an empty string"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `write!()` with a format
    /// string that
    /// ends in a newline.
    ///
    /// ### Why is this bad?
    /// You should use `writeln!()` instead, which appends the
    /// newline.
    ///
    /// ### Example
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// # let name = "World";
    /// // Bad
    /// write!(buf, "Hello {}!\n", name);
    ///
    /// // Good
    /// writeln!(buf, "Hello {}!", name);
    /// ```
    pub WRITE_WITH_NEWLINE,
    style,
    "using `write!()` with a format string that ends in a single newline"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns about the use of literals as `write!`/`writeln!` args.
    ///
    /// ### Why is this bad?
    /// Using literals as `writeln!` args is inefficient
    /// (c.f., https://github.com/matthiaskrgr/rust-str-bench) and unnecessary
    /// (i.e., just put the literal in the format string)
    ///
    /// ### Known problems
    /// Will also warn with macro calls as arguments that expand to literals
    /// -- e.g., `writeln!(buf, "{}", env!("FOO"))`.
    ///
    /// ### Example
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// // Bad
    /// writeln!(buf, "{}", "foo");
    ///
    /// // Good
    /// writeln!(buf, "foo");
    /// ```
    pub WRITE_LITERAL,
    style,
    "writing a literal with a format string"
}

#[derive(Default)]
pub struct Write {
    in_debug_impl: bool,
}

impl_lint_pass!(Write => [
    PRINT_WITH_NEWLINE,
    PRINTLN_EMPTY_STRING,
    PRINT_STDOUT,
    PRINT_STDERR,
    USE_DEBUG,
    PRINT_LITERAL,
    WRITE_WITH_NEWLINE,
    WRITELN_EMPTY_STRING,
    WRITE_LITERAL
]);

impl EarlyLintPass for Write {
    fn check_item(&mut self, _: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Impl(box ImplKind {
            of_trait: Some(trait_ref),
            ..
        }) = &item.kind
        {
            let trait_name = trait_ref
                .path
                .segments
                .iter()
                .last()
                .expect("path has at least one segment")
                .ident
                .name;
            if trait_name == sym::Debug {
                self.in_debug_impl = true;
            }
        }
    }

    fn check_item_post(&mut self, _: &EarlyContext<'_>, _: &Item) {
        self.in_debug_impl = false;
    }

    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        fn is_build_script(cx: &EarlyContext<'_>) -> bool {
            // Cargo sets the crate name for build scripts to `build_script_build`
            cx.sess
                .opts
                .crate_name
                .as_ref()
                .map_or(false, |crate_name| crate_name == "build_script_build")
        }

        if mac.path == sym!(print) {
            if !is_build_script(cx) {
                span_lint(cx, PRINT_STDOUT, mac.span(), "use of `print!`");
            }
            self.lint_print_with_newline(cx, mac);
        } else if mac.path == sym!(println) {
            if !is_build_script(cx) {
                span_lint(cx, PRINT_STDOUT, mac.span(), "use of `println!`");
            }
            self.lint_println_empty_string(cx, mac);
        } else if mac.path == sym!(eprint) {
            span_lint(cx, PRINT_STDERR, mac.span(), "use of `eprint!`");
            self.lint_print_with_newline(cx, mac);
        } else if mac.path == sym!(eprintln) {
            span_lint(cx, PRINT_STDERR, mac.span(), "use of `eprintln!`");
            self.lint_println_empty_string(cx, mac);
        } else if mac.path == sym!(write) {
            if let (Some(fmt_str), dest) = self.check_tts(cx, mac.args.inner_tokens(), true) {
                if check_newlines(&fmt_str) {
                    let (nl_span, only_nl) = newline_span(&fmt_str);
                    let nl_span = match (dest, only_nl) {
                        // Special case of `write!(buf, "\n")`: Mark everything from the end of
                        // `buf` for removal so no trailing comma [`writeln!(buf, )`] remains.
                        (Some(dest_expr), true) => Span::new(dest_expr.span.hi(), nl_span.hi(), nl_span.ctxt()),
                        _ => nl_span,
                    };
                    span_lint_and_then(
                        cx,
                        WRITE_WITH_NEWLINE,
                        mac.span(),
                        "using `write!()` with a format string that ends in a single newline",
                        |err| {
                            err.multipart_suggestion(
                                "use `writeln!()` instead",
                                vec![(mac.path.span, String::from("writeln")), (nl_span, String::new())],
                                Applicability::MachineApplicable,
                            );
                        },
                    );
                }
            }
        } else if mac.path == sym!(writeln) {
            if let (Some(fmt_str), expr) = self.check_tts(cx, mac.args.inner_tokens(), true) {
                if fmt_str.symbol == kw::Empty {
                    let mut applicability = Applicability::MachineApplicable;
                    // FIXME: remove this `#[allow(...)]` once the issue #5822 gets fixed
                    #[allow(clippy::option_if_let_else)]
                    let suggestion = if let Some(e) = expr {
                        snippet_with_applicability(cx, e.span, "v", &mut applicability)
                    } else {
                        applicability = Applicability::HasPlaceholders;
                        Cow::Borrowed("v")
                    };

                    span_lint_and_sugg(
                        cx,
                        WRITELN_EMPTY_STRING,
                        mac.span(),
                        format!("using `writeln!({}, \"\")`", suggestion).as_str(),
                        "replace it with",
                        format!("writeln!({})", suggestion),
                        applicability,
                    );
                }
            }
        }
    }
}

/// Given a format string that ends in a newline and its span, calculates the span of the
/// newline, or the format string itself if the format string consists solely of a newline.
/// Return this and a boolean indicating whether it only consisted of a newline.
fn newline_span(fmtstr: &StrLit) -> (Span, bool) {
    let sp = fmtstr.span;
    let contents = &fmtstr.symbol.as_str();

    if *contents == r"\n" {
        return (sp, true);
    }

    let newline_sp_hi = sp.hi()
        - match fmtstr.style {
            StrStyle::Cooked => BytePos(1),
            StrStyle::Raw(hashes) => BytePos((1 + hashes).into()),
        };

    let newline_sp_len = if contents.ends_with('\n') {
        BytePos(1)
    } else if contents.ends_with(r"\n") {
        BytePos(2)
    } else {
        panic!("expected format string to contain a newline");
    };

    (sp.with_lo(newline_sp_hi - newline_sp_len).with_hi(newline_sp_hi), false)
}

/// Stores a list of replacement spans for each argument, but only if all the replacements used an
/// empty format string.
#[derive(Default)]
struct SimpleFormatArgs {
    unnamed: Vec<Vec<Span>>,
    named: Vec<(Symbol, Vec<Span>)>,
}
impl SimpleFormatArgs {
    fn get_unnamed(&self) -> impl Iterator<Item = &[Span]> {
        self.unnamed.iter().map(|x| match x.as_slice() {
            // Ignore the dummy span added from out of order format arguments.
            [DUMMY_SP] => &[],
            x => x,
        })
    }

    fn get_named(&self, n: &Path) -> &[Span] {
        self.named.iter().find(|x| *n == x.0).map_or(&[], |x| x.1.as_slice())
    }

    fn push(&mut self, arg: rustc_parse_format::Argument<'_>, span: Span) {
        use rustc_parse_format::{
            AlignUnknown, ArgumentImplicitlyIs, ArgumentIs, ArgumentNamed, CountImplied, FormatSpec,
        };

        const SIMPLE: FormatSpec<'_> = FormatSpec {
            fill: None,
            align: AlignUnknown,
            flags: 0,
            precision: CountImplied,
            precision_span: None,
            width: CountImplied,
            width_span: None,
            ty: "",
            ty_span: None,
        };

        match arg.position {
            ArgumentIs(n) | ArgumentImplicitlyIs(n) => {
                if self.unnamed.len() <= n {
                    // Use a dummy span to mark all unseen arguments.
                    self.unnamed.resize_with(n, || vec![DUMMY_SP]);
                    if arg.format == SIMPLE {
                        self.unnamed.push(vec![span]);
                    } else {
                        self.unnamed.push(Vec::new());
                    }
                } else {
                    let args = &mut self.unnamed[n];
                    match (args.as_mut_slice(), arg.format == SIMPLE) {
                        // A non-empty format string has been seen already.
                        ([], _) => (),
                        // Replace the dummy span, if it exists.
                        ([dummy @ DUMMY_SP], true) => *dummy = span,
                        ([_, ..], true) => args.push(span),
                        ([_, ..], false) => *args = Vec::new(),
                    }
                }
            },
            ArgumentNamed(n) => {
                if let Some(x) = self.named.iter_mut().find(|x| x.0 == n) {
                    match x.1.as_slice() {
                        // A non-empty format string has been seen already.
                        [] => (),
                        [_, ..] if arg.format == SIMPLE => x.1.push(span),
                        [_, ..] => x.1 = Vec::new(),
                    }
                } else if arg.format == SIMPLE {
                    self.named.push((n, vec![span]));
                } else {
                    self.named.push((n, Vec::new()));
                }
            },
        };
    }
}

impl Write {
    /// Parses a format string into a collection of spans for each argument. This only keeps track
    /// of empty format arguments. Will also lint usages of debug format strings outside of debug
    /// impls.
    fn parse_fmt_string(&self, cx: &EarlyContext<'_>, str_lit: &StrLit) -> Option<SimpleFormatArgs> {
        use rustc_parse_format::{ParseMode, Parser, Piece};

        let str_sym = str_lit.symbol_unescaped.as_str();
        let style = match str_lit.style {
            StrStyle::Cooked => None,
            StrStyle::Raw(n) => Some(n as usize),
        };

        let mut parser = Parser::new(&str_sym, style, snippet_opt(cx, str_lit.span), false, ParseMode::Format);
        let mut args = SimpleFormatArgs::default();

        while let Some(arg) = parser.next() {
            let arg = match arg {
                Piece::String(_) => continue,
                Piece::NextArgument(arg) => arg,
            };
            let span = parser
                .arg_places
                .last()
                .map_or(DUMMY_SP, |&x| str_lit.span.from_inner(x));

            if !self.in_debug_impl && arg.format.ty == "?" {
                // FIXME: modify rustc's fmt string parser to give us the current span
                span_lint(cx, USE_DEBUG, span, "use of `Debug`-based formatting");
            }

            args.push(arg, span);
        }

        parser.errors.is_empty().then(move || args)
    }

    /// Checks the arguments of `print[ln]!` and `write[ln]!` calls. It will return a tuple of two
    /// `Option`s. The first `Option` of the tuple is the macro's format string. It includes
    /// the contents of the string, whether it's a raw string, and the span of the literal in the
    /// source. The second `Option` in the tuple is, in the `write[ln]!` case, the expression the
    /// `format_str` should be written to.
    ///
    /// Example:
    ///
    /// Calling this function on
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// # let something = "something";
    /// writeln!(buf, "string to write: {}", something);
    /// ```
    /// will return
    /// ```rust,ignore
    /// (Some("string to write: {}"), Some(buf))
    /// ```
    #[allow(clippy::too_many_lines)]
    fn check_tts<'a>(&self, cx: &EarlyContext<'a>, tts: TokenStream, is_write: bool) -> (Option<StrLit>, Option<Expr>) {
        let mut parser = parser::Parser::new(&cx.sess.parse_sess, tts, false, None);
        let expr = if is_write {
            match parser
                .parse_expr()
                .map(rustc_ast::ptr::P::into_inner)
                .map_err(|mut e| e.cancel())
            {
                // write!(e, ...)
                Ok(p) if parser.eat(&token::Comma) => Some(p),
                // write!(e) or error
                e => return (None, e.ok()),
            }
        } else {
            None
        };

        let fmtstr = match parser.parse_str_lit() {
            Ok(fmtstr) => fmtstr,
            Err(_) => return (None, expr),
        };

        let args = match self.parse_fmt_string(cx, &fmtstr) {
            Some(args) => args,
            None => return (Some(fmtstr), expr),
        };

        let lint = if is_write { WRITE_LITERAL } else { PRINT_LITERAL };
        let mut unnamed_args = args.get_unnamed();
        loop {
            if !parser.eat(&token::Comma) {
                return (Some(fmtstr), expr);
            }

            let comma_span = parser.prev_token.span;
            let token_expr = if let Ok(expr) = parser.parse_expr().map_err(|mut err| err.cancel()) {
                expr
            } else {
                return (Some(fmtstr), None);
            };
            let (fmt_spans, lit) = match &token_expr.kind {
                ExprKind::Lit(lit) => (unnamed_args.next().unwrap_or(&[]), lit),
                ExprKind::Assign(lhs, rhs, _) => match (&lhs.kind, &rhs.kind) {
                    (ExprKind::Path(_, p), ExprKind::Lit(lit)) => (args.get_named(p), lit),
                    _ => continue,
                },
                _ => {
                    unnamed_args.next();
                    continue;
                },
            };

            let replacement: String = match lit.token.kind {
                LitKind::Integer | LitKind::Float | LitKind::Err => continue,
                LitKind::StrRaw(_) | LitKind::ByteStrRaw(_) if matches!(fmtstr.style, StrStyle::Raw(_)) => {
                    lit.token.symbol.as_str().replace("{", "{{").replace("}", "}}")
                },
                LitKind::Str | LitKind::ByteStr if matches!(fmtstr.style, StrStyle::Cooked) => {
                    lit.token.symbol.as_str().replace("{", "{{").replace("}", "}}")
                },
                LitKind::StrRaw(_) | LitKind::Str | LitKind::ByteStrRaw(_) | LitKind::ByteStr => continue,
                LitKind::Byte | LitKind::Char => match &*lit.token.symbol.as_str() {
                    "\"" if matches!(fmtstr.style, StrStyle::Cooked) => "\\\"",
                    "\"" if matches!(fmtstr.style, StrStyle::Raw(0)) => continue,
                    "\\\\" if matches!(fmtstr.style, StrStyle::Raw(_)) => "\\",
                    "\\'" => "'",
                    "{" => "{{",
                    "}" => "}}",
                    x if matches!(fmtstr.style, StrStyle::Raw(_)) && x.starts_with('\\') => continue,
                    x => x,
                }
                .into(),
                LitKind::Bool => lit.token.symbol.as_str().deref().into(),
            };

            if !fmt_spans.is_empty() {
                span_lint_and_then(
                    cx,
                    lint,
                    token_expr.span,
                    "literal with an empty format string",
                    |diag| {
                        diag.multipart_suggestion(
                            "try this",
                            iter::once((comma_span.to(token_expr.span), String::new()))
                                .chain(fmt_spans.iter().copied().zip(iter::repeat(replacement)))
                                .collect(),
                            Applicability::MachineApplicable,
                        );
                    },
                );
            }
        }
    }

    fn lint_println_empty_string(&self, cx: &EarlyContext<'_>, mac: &MacCall) {
        if let (Some(fmt_str), _) = self.check_tts(cx, mac.args.inner_tokens(), false) {
            if fmt_str.symbol == kw::Empty {
                let name = mac.path.segments[0].ident.name;
                span_lint_and_sugg(
                    cx,
                    PRINTLN_EMPTY_STRING,
                    mac.span(),
                    &format!("using `{}!(\"\")`", name),
                    "replace it with",
                    format!("{}!()", name),
                    Applicability::MachineApplicable,
                );
            }
        }
    }

    fn lint_print_with_newline(&self, cx: &EarlyContext<'_>, mac: &MacCall) {
        if let (Some(fmt_str), _) = self.check_tts(cx, mac.args.inner_tokens(), false) {
            if check_newlines(&fmt_str) {
                let name = mac.path.segments[0].ident.name;
                let suggested = format!("{}ln", name);
                span_lint_and_then(
                    cx,
                    PRINT_WITH_NEWLINE,
                    mac.span(),
                    &format!("using `{}!()` with a format string that ends in a single newline", name),
                    |err| {
                        err.multipart_suggestion(
                            &format!("use `{}!` instead", suggested),
                            vec![(mac.path.span, suggested), (newline_span(&fmt_str).0, String::new())],
                            Applicability::MachineApplicable,
                        );
                    },
                );
            }
        }
    }
}

/// Checks if the format string contains a single newline that terminates it.
///
/// Literal and escaped newlines are both checked (only literal for raw strings).
fn check_newlines(fmtstr: &StrLit) -> bool {
    let mut has_internal_newline = false;
    let mut last_was_cr = false;
    let mut should_lint = false;

    let contents = &fmtstr.symbol.as_str();

    let mut cb = |r: Range<usize>, c: Result<char, EscapeError>| {
        let c = c.unwrap();

        if r.end == contents.len() && c == '\n' && !last_was_cr && !has_internal_newline {
            should_lint = true;
        } else {
            last_was_cr = c == '\r';
            if c == '\n' {
                has_internal_newline = true;
            }
        }
    };

    match fmtstr.style {
        StrStyle::Cooked => unescape::unescape_literal(contents, unescape::Mode::Str, &mut cb),
        StrStyle::Raw(_) => unescape::unescape_literal(contents, unescape::Mode::RawStr, &mut cb),
    }

    should_lint
}
