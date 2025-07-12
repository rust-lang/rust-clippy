use crate::source_map::{SourceFile, SourceMap, Span};
use crate::utils::{ErrAction, expect_action, walk_dir_no_dot_or_target};
use core::ops::Range;
use core::slice;
use rustc_data_structures::fx::FxHashMap;
use rustc_index::{IndexVec, newtype_index};
use rustc_lexer::{self as lexer, FrontmatterAllowed};
use std::collections::hash_map::{Entry, VacantEntry};
use std::panic::Location;
use std::path::{self, Path};
use std::{fs, process};

#[derive(Clone, Copy)]
pub enum Token<'a> {
    /// Matches any number of comments / doc comments.
    AnyComment,
    AnyIdent,
    At,
    Ident(&'a str),
    CaptureIdent,
    LitStr,
    CaptureLitStr,
    Bang,
    CloseBrace,
    CloseBracket,
    CloseParen,
    /// This will consume the first colon even if the second doesn't exist.
    DoubleColon,
    Comma,
    Eq,
    FatArrow,
    Lifetime,
    Literal,
    Lt,
    Gt,
    OpenBrace,
    OpenBracket,
    OpenParen,
    OptLifetimeArgs,
    Pound,
    Semi,
}

#[derive(Clone, Copy)]
pub struct Capture {
    pub pos: u32,
    pub len: u32,
}
impl Capture {
    pub const EMPTY: Self = Self { pos: 0, len: 0 };
    pub fn to_index(self) -> Range<usize> {
        self.pos as usize..(self.pos + self.len) as usize
    }

    pub fn to_span(self, file: SourceFile) -> Span {
        Span {
            file,
            start: self.pos,
            end: self.pos + self.len,
        }
    }
}

pub struct RustSearcher<'txt> {
    text: &'txt str,
    cursor: lexer::Cursor<'txt>,
    pos: u32,
    next_token: lexer::Token,
}
impl<'txt> RustSearcher<'txt> {
    #[must_use]
    #[expect(clippy::inconsistent_struct_constructor)]
    pub fn new(text: &'txt str) -> Self {
        let mut cursor = lexer::Cursor::new(text, FrontmatterAllowed::Yes);
        Self {
            text,
            pos: 0,
            next_token: cursor.advance_token(),
            cursor,
        }
    }

    #[must_use]
    pub fn get_capture(&self, capture: Capture) -> &'txt str {
        &self.text[capture.to_index()]
    }

    #[must_use]
    pub fn peek_text(&self) -> &'txt str {
        &self.text[self.pos as usize..(self.pos + self.next_token.len) as usize]
    }

    #[must_use]
    pub fn peek_span(&self, file: SourceFile) -> Span {
        Span {
            file,
            start: self.pos,
            end: self.pos + self.next_token.len,
        }
    }

    #[track_caller]
    fn get_unexpected_err(&self, file: SourceFile) -> Error {
        ErrorKind::UnexpectedToken {
            token: self.peek_text().into(),
            span: self.peek_span(file),
        }
        .into()
    }

    #[must_use]
    pub fn peek_len(&self) -> u32 {
        self.next_token.len
    }

    #[must_use]
    pub fn peek(&self) -> lexer::TokenKind {
        self.next_token.kind
    }

    #[must_use]
    pub fn pos(&self) -> u32 {
        self.pos
    }

    #[must_use]
    pub fn at_end(&self) -> bool {
        self.next_token.kind == lexer::TokenKind::Eof
    }

    /// Steps to the next token, or `TokenKind::Eof` if there are no more tokens.
    pub fn step(&mut self) {
        // `next_token.len` is zero for the eof marker.
        self.pos += self.next_token.len;
        self.next_token = self.cursor.advance_token();
    }

    /// Consumes the next token if it matches the requested value and captures the value if
    /// requested. Returns `true` if a token was matched.
    #[expect(clippy::too_many_lines)]
    fn read_token(&mut self, token: Token<'_>, captures: &mut slice::IterMut<'_, Capture>) -> bool {
        loop {
            match (token, self.next_token.kind) {
                (_, lexer::TokenKind::Whitespace)
                | (
                    Token::AnyComment,
                    lexer::TokenKind::BlockComment { terminated: true, .. } | lexer::TokenKind::LineComment { .. },
                ) => self.step(),
                (Token::AnyComment, _) => return true,
                (Token::At, lexer::TokenKind::At)
                | (Token::AnyIdent, lexer::TokenKind::Ident)
                | (Token::Bang, lexer::TokenKind::Bang)
                | (Token::CloseBrace, lexer::TokenKind::CloseBrace)
                | (Token::CloseBracket, lexer::TokenKind::CloseBracket)
                | (Token::CloseParen, lexer::TokenKind::CloseParen)
                | (Token::Comma, lexer::TokenKind::Comma)
                | (Token::Eq, lexer::TokenKind::Eq)
                | (Token::Lifetime, lexer::TokenKind::Lifetime { .. })
                | (Token::Lt, lexer::TokenKind::Lt)
                | (Token::Gt, lexer::TokenKind::Gt)
                | (Token::OpenBrace, lexer::TokenKind::OpenBrace)
                | (Token::OpenBracket, lexer::TokenKind::OpenBracket)
                | (Token::OpenParen, lexer::TokenKind::OpenParen)
                | (Token::Pound, lexer::TokenKind::Pound)
                | (Token::Semi, lexer::TokenKind::Semi)
                | (
                    Token::LitStr,
                    lexer::TokenKind::Literal {
                        kind: lexer::LiteralKind::Str { terminated: true } | lexer::LiteralKind::RawStr { .. },
                        ..
                    },
                )
                | (
                    Token::Literal,
                    lexer::TokenKind::Literal {
                        kind:
                            lexer::LiteralKind::Int { .. }
                            | lexer::LiteralKind::Float { .. }
                            | lexer::LiteralKind::Byte { terminated: true }
                            | lexer::LiteralKind::ByteStr { terminated: true }
                            | lexer::LiteralKind::Char { terminated: true }
                            | lexer::LiteralKind::CStr { terminated: true }
                            | lexer::LiteralKind::Str { terminated: true }
                            | lexer::LiteralKind::RawByteStr { .. }
                            | lexer::LiteralKind::RawCStr { .. }
                            | lexer::LiteralKind::RawStr { .. },
                        ..
                    },
                ) => {
                    self.step();
                    return true;
                },
                (Token::Literal, lexer::TokenKind::Ident) if matches!(self.peek_text(), "true" | "false") => {
                    self.step();
                    return true;
                },
                (Token::Ident(x), lexer::TokenKind::Ident) if x == self.peek_text() => {
                    self.step();
                    return true;
                },
                (Token::DoubleColon, lexer::TokenKind::Colon) => {
                    self.step();
                    if matches!(self.next_token.kind, lexer::TokenKind::Colon) {
                        self.step();
                        return true;
                    }
                    return false;
                },
                (Token::FatArrow, lexer::TokenKind::Eq) => {
                    self.step();
                    if matches!(self.next_token.kind, lexer::TokenKind::Gt) {
                        self.step();
                        return true;
                    }
                    return false;
                },
                (Token::OptLifetimeArgs, lexer::TokenKind::Lt) => {
                    self.step();
                    while self.read_token(Token::Lifetime, captures) {
                        if !self.read_token(Token::Comma, captures) {
                            break;
                        }
                    }
                    return self.read_token(Token::Gt, captures);
                },
                #[expect(clippy::match_same_arms)]
                (Token::OptLifetimeArgs, _) => return true,
                #[rustfmt::skip]
                (
                    Token::CaptureLitStr,
                    lexer::TokenKind::Literal {
                        kind:
                            lexer::LiteralKind::Str { terminated: true }
                            | lexer::LiteralKind::RawStr { n_hashes: Some(_) },
                        ..
                    },
                )
                | (Token::CaptureIdent, lexer::TokenKind::Ident) => {
                    *captures.next().unwrap() = Capture { pos: self.pos, len: self.next_token.len };
                    self.step();
                    return true;
                },
                _ => return false,
            }
        }
    }

    #[must_use]
    pub fn find_token(&mut self, token: Token<'_>) -> bool {
        let mut captures = [].iter_mut();
        while !self.read_token(token, &mut captures) {
            self.step();
            if self.at_end() {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn find_any_ident(&mut self) -> Option<&'txt str> {
        loop {
            match self.next_token.kind {
                lexer::TokenKind::Ident => {
                    let res = self.peek_text();
                    self.step();
                    return Some(res);
                },
                lexer::TokenKind::Eof => return None,
                _ => self.step(),
            }
        }
    }

    #[must_use]
    pub fn find_ident(&mut self, s: &str) -> bool {
        while let Some(x) = self.find_any_ident() {
            if x == s {
                return true;
            }
        }
        false
    }

    #[must_use]
    pub fn match_token(&mut self, token: Token<'_>) -> bool {
        let mut captures = [].iter_mut();
        self.read_token(token, &mut captures)
    }

    #[must_use]
    pub fn match_tokens(&mut self, tokens: &[Token<'_>], captures: &mut [Capture]) -> bool {
        let mut captures = captures.iter_mut();
        tokens.iter().all(|&t| self.read_token(t, &mut captures))
    }
}

pub struct ActiveLint {
    pub group: String,
    pub decl_span: Span,
}

pub struct DeprecatedLint {
    pub reason: String,
    pub version: String,
}

pub struct RenamedLint {
    pub new_name: String,
    pub version: String,
}

pub enum LintKind {
    Active(ActiveLint),
    Deprecated(DeprecatedLint),
    Renamed(RenamedLint),
}

pub struct Lint {
    pub kind: LintKind,
    pub name_span: Span,
}

pub struct LintPassData {
    pub name: String,
    /// Span of the `impl_lint_pass` or `declare_lint_pass` macro call.
    pub mac_span: Span,
}

newtype_index! {
    #[orderable]
    pub struct LintPass {}
}

pub struct LintRegistration {
    pub name: String,
    pub pass: LintPass,
}

pub struct ParsedData {
    pub source_map: SourceMap,
    pub lints: FxHashMap<String, Lint>,
    pub lint_passes: IndexVec<LintPass, LintPassData>,
    pub lint_registrations: Vec<LintRegistration>,
    pub deprecated_span: Range<u32>,
    pub renamed_span: Range<u32>,
}
impl ParsedData {
    #[expect(clippy::default_trait_access)]
    pub fn collect() -> Self {
        // 2025-05: Initial capacities should fit everything without reallocating.
        let mut parser = Parser {
            source_map: SourceMap::with_capacity(8, 1000),
            lints: FxHashMap::with_capacity_and_hasher(1000, Default::default()),
            lint_passes: IndexVec::with_capacity(400),
            lint_registrations: Vec::with_capacity(1000),
            deprecated_span: 0..0,
            renamed_span: 0..0,
            errors: Vec::new(),
        };
        parser.parse_src_files();
        parser.parse_deprecated_lints();

        if !parser.errors.is_empty() {
            for error in &parser.errors {
                match &error.kind {
                    ErrorKind::DuplicateLint { name, span, prev_span } => {
                        eprint!(
                            "error: duplicate lint `{name}` found\n  at: {}\n  previous: {}\n",
                            span.display(&parser.source_map),
                            prev_span.display(&parser.source_map),
                        );
                    },
                    ErrorKind::NotLintName(span) => {
                        eprint!(
                            "error: invalid lint name found\n  at: {}\n",
                            span.display(&parser.source_map),
                        );
                    },
                    ErrorKind::LintMissingPrefix(span) => {
                        eprint!(
                            "error: lint name missing `clippy::` prefix\n  at: {}\n",
                            span.display(&parser.source_map),
                        );
                    },
                    ErrorKind::StrLit(span) => {
                        eprint!(
                            "error: invalid string literal\n  at: {}\n",
                            span.display(&parser.source_map),
                        );
                    },
                    ErrorKind::StrLitEol(span) => {
                        eprint!(
                            "error: string literal contains a line ending\n  at: {}\n",
                            span.display(&parser.source_map),
                        );
                    },
                    ErrorKind::UnexpectedToken { token, span } => {
                        eprint!(
                            "error: unexpected token `{token}`\n  at: {}\n",
                            span.display(&parser.source_map),
                        );
                    },
                }
                eprintln!("  error-src: {}", error.loc);
            }
            process::exit(1);
        }

        ParsedData {
            source_map: parser.source_map,
            lints: parser.lints,
            lint_passes: parser.lint_passes,
            lint_registrations: parser.lint_registrations,
            deprecated_span: parser.deprecated_span,
            renamed_span: parser.renamed_span,
        }
    }
}

enum ErrorKind {
    /// Multiple lint declarations with the same name.
    DuplicateLint {
        name: String,
        span: Span,
        prev_span: Span,
    },
    /// A string literal is not a valid lint name.
    NotLintName(Span),
    /// A lint name is missing the `clippy::` prefix.
    LintMissingPrefix(Span),
    /// Error when parsing a string literal.
    StrLit(Span),
    // A string literal contains a line terminator.
    StrLitEol(Span),
    // A token not expected in the source was found.
    UnexpectedToken {
        token: String,
        span: Span,
    },
}
struct Error {
    kind: ErrorKind,
    loc: &'static Location<'static>,
}
impl From<ErrorKind> for Error {
    #[track_caller]
    fn from(kind: ErrorKind) -> Self {
        Self {
            kind,
            loc: Location::caller(),
        }
    }
}

struct Parser {
    source_map: SourceMap,
    lints: FxHashMap<String, Lint>,
    lint_passes: IndexVec<LintPass, LintPassData>,
    lint_registrations: Vec<LintRegistration>,
    deprecated_span: Range<u32>,
    renamed_span: Range<u32>,
    errors: Vec<Error>,
}
impl Parser {
    /// Parses all source files looking for lint declarations (`declare_clippy_lint! { .. }`).
    fn parse_src_files(&mut self) {
        for e in expect_action(fs::read_dir("."), ErrAction::Read, ".") {
            let e = expect_action(e, ErrAction::Read, ".");
            if !expect_action(e.file_type(), ErrAction::Read, ".").is_dir() {
                continue;
            }
            let Ok(mut crate_path) = e.file_name().into_string() else {
                continue;
            };
            if crate_path.starts_with("clippy_lints") && crate_path != "clippy_lints_internal" {
                let krate = self.source_map.add_new_crate(&crate_path);
                crate_path.push(path::MAIN_SEPARATOR);
                crate_path.push_str("src");
                for e in walk_dir_no_dot_or_target(&crate_path) {
                    let e = expect_action(e, ErrAction::Read, &crate_path);
                    if let Some(path) = e.path().to_str()
                        && let Some(path) = path.strip_suffix(".rs")
                        && let Some(path) = path.get(crate_path.len() + 1..)
                    {
                        let module = if path == "lib" {
                            String::new()
                        } else {
                            let path = if let Some(path) = path.strip_suffix("mod")
                                && let Some(path) = path.strip_suffix(path::MAIN_SEPARATOR)
                            {
                                path
                            } else {
                                path
                            };
                            path.replace(path::MAIN_SEPARATOR, "::")
                        };
                        let file = self.source_map.load_new_file(e.path(), krate, module);
                        self.parse_src_file(file);
                    }
                }
            }
        }
    }

    /// Parse a source file looking for `declare_clippy_lint` macro invocations.
    fn parse_src_file(&mut self, file: SourceFile) {
        #[allow(clippy::enum_glob_use)]
        use Token::*;
        #[rustfmt::skip]
        static LINT_DECL_TOKENS: &[Token<'_>] = &[
            // { /// docs
            OpenBrace, AnyComment,
            // #[clippy::version = "version"]
            Pound, OpenBracket, Ident("clippy"), DoubleColon, Ident("version"), Eq, LitStr, CloseBracket,
            // pub NAME, GROUP, "desc"
            Ident("pub"), CaptureIdent, Comma, AnyComment, CaptureIdent, Comma, AnyComment, LitStr,
        ];
        #[rustfmt::skip]
        static LINT_DECL_EXTRA_TOKENS: &[Token<'_>] = &[
            // , @option = value
            Comma, AnyComment, At, AnyIdent, Eq, Literal,
        ];
        #[rustfmt::skip]
        static LINT_PASS_TOKENS: &[Token<'_>] = &[
            // ( name <'lt> => [
            OpenParen, AnyComment, CaptureIdent, OptLifetimeArgs, FatArrow, OpenBracket,
        ];

        let mut searcher = RustSearcher::new(&self.source_map.files[file].contents);
        let mut captures = [Capture::EMPTY; 2];
        while let Some(ident) = searcher.find_any_ident() {
            #[expect(clippy::cast_possible_truncation)]
            let start = searcher.pos - ident.len() as u32;
            if searcher.match_token(Bang) {
                match ident {
                    "declare_clippy_lint" => {
                        if !searcher.match_tokens(LINT_DECL_TOKENS, &mut captures) {
                            self.errors.push(searcher.get_unexpected_err(file));
                            return;
                        }
                        while searcher.match_tokens(LINT_DECL_EXTRA_TOKENS, &mut []) {
                            // nothing
                        }
                        if !searcher.match_token(CloseBrace) {
                            self.errors.push(searcher.get_unexpected_err(file));
                            return;
                        }
                        let name_span = captures[0].to_span(file);
                        let name = searcher.get_capture(captures[0]).to_ascii_lowercase();
                        if let Some(e) = get_vacant_lint(name, name_span, &mut self.lints, &mut self.errors) {
                            e.insert(Lint {
                                kind: LintKind::Active(ActiveLint {
                                    group: searcher.get_capture(captures[1]).into(),
                                    decl_span: Span {
                                        file,
                                        start,
                                        end: searcher.pos(),
                                    },
                                }),
                                name_span,
                            });
                        }
                    },
                    "impl_lint_pass" | "declare_lint_pass" => {
                        if !searcher.match_tokens(LINT_PASS_TOKENS, &mut captures) {
                            self.errors.push(searcher.get_unexpected_err(file));
                            return;
                        }
                        let pass = self.lint_passes.next_index();
                        let pass_name = captures[0];
                        while searcher.match_tokens(&[AnyComment, CaptureIdent], &mut captures) {
                            // Read a path expression.
                            while searcher.match_token(DoubleColon) {
                                // Overwrite the previous capture. The last segment is the lint name we want.
                                if !searcher.match_tokens(&[CaptureIdent], &mut captures) {
                                    self.errors.push(searcher.get_unexpected_err(file));
                                    return;
                                }
                            }
                            self.lint_registrations.push(LintRegistration {
                                name: searcher.get_capture(captures[0]).to_ascii_lowercase(),
                                pass,
                            });
                            if !searcher.match_token(Comma) {
                                break;
                            }
                        }
                        if !searcher.match_tokens(&[CloseBracket, CloseParen], &mut []) {
                            self.errors.push(searcher.get_unexpected_err(file));
                            return;
                        }
                        self.lint_passes.push(LintPassData {
                            name: searcher.get_capture(pass_name).to_owned(),
                            mac_span: Span {
                                file,
                                start,
                                end: searcher.pos(),
                            },
                        });
                    },
                    _ => {},
                }
            }
        }
    }

    pub fn parse_deprecated_lints(&mut self) {
        #[allow(clippy::enum_glob_use)]
        use Token::*;
        #[rustfmt::skip]
        static DECL_TOKENS: &[Token<'_>] = &[
            // #[clippy::version = "version"]
            Pound, OpenBracket, Ident("clippy"), DoubleColon, Ident("version"), Eq, CaptureLitStr, CloseBracket,
            // ("first", "second"),
            OpenParen, CaptureLitStr, Comma, CaptureLitStr, CloseParen, Comma,
        ];
        #[rustfmt::skip]
        static DEPRECATED_TOKENS: &[Token<'_>] = &[
            // !{ DEPRECATED(DEPRECATED_VERSION) = [
            Bang, OpenBrace, Ident("DEPRECATED"), OpenParen, Ident("DEPRECATED_VERSION"), CloseParen, Eq, OpenBracket,
        ];
        #[rustfmt::skip]
        static RENAMED_TOKENS: &[Token<'_>] = &[
            // !{ RENAMED(RENAMED_VERSION) = [
            Bang, OpenBrace, Ident("RENAMED"), OpenParen, Ident("RENAMED_VERSION"), CloseParen, Eq, OpenBracket,
        ];
        #[rustfmt::skip]
        static END_TOKENS: &[Token<'_>] = &[
            // ]}
            CloseBracket, CloseBrace,
        ];

        let krate = self.source_map.add_crate("clippy_lints");
        let file = self.source_map.load_file(
            Path::new("clippy_lints/src/deprecated_lints.rs"),
            krate,
            "deprecated_lints",
        );
        let file_data = &self.source_map.files[file];

        let mut captures = [Capture::EMPTY; 3];
        let mut searcher = RustSearcher::new(&file_data.contents);
        // First instance is the macro definition.
        assert!(
            searcher.find_ident("declare_with_version"),
            "error parsing `clippy_lints/src/deprecated_lints.rs`",
        );

        if !searcher.find_ident("declare_with_version") || !searcher.match_tokens(DEPRECATED_TOKENS, &mut []) {
            self.errors.push(searcher.get_unexpected_err(file));
            return;
        }
        let start = searcher.pos();
        let mut end = start;
        while searcher.match_tokens(DECL_TOKENS, &mut captures) {
            end = searcher.pos();
            let name_span = captures[1].to_span(file);
            let (Some(version), Some(name), Some(reason)) = (
                parse_str_single_line(captures[0], searcher.text, file, &mut self.errors),
                parse_clippy_lint_name(captures[1], searcher.text, file, &mut self.errors),
                parse_str_single_line(captures[2], searcher.text, file, &mut self.errors),
            ) else {
                continue;
            };
            if let Some(e) = get_vacant_lint(name, name_span, &mut self.lints, &mut self.errors) {
                e.insert(Lint {
                    kind: LintKind::Deprecated(DeprecatedLint { reason, version }),
                    name_span,
                });
            }
        }
        self.deprecated_span = start..end;
        if !searcher.match_tokens(END_TOKENS, &mut []) {
            self.errors.push(searcher.get_unexpected_err(file));
            return;
        }

        if !searcher.find_ident("declare_with_version") || !searcher.match_tokens(RENAMED_TOKENS, &mut []) {
            self.errors.push(searcher.get_unexpected_err(file));
            return;
        }
        let start = searcher.pos();
        let mut end = start;
        while searcher.match_tokens(DECL_TOKENS, &mut captures) {
            end = searcher.pos();
            let name_span = captures[1].to_span(file);
            let (Some(version), Some(name), Some(new_name)) = (
                parse_str_single_line(captures[0], searcher.text, file, &mut self.errors),
                parse_clippy_lint_name(captures[1], searcher.text, file, &mut self.errors),
                parse_maybe_clippy_lint_name(captures[2], searcher.text, file, &mut self.errors),
            ) else {
                continue;
            };
            if let Some(e) = get_vacant_lint(name, name_span, &mut self.lints, &mut self.errors) {
                e.insert(Lint {
                    kind: LintKind::Renamed(RenamedLint { new_name, version }),
                    name_span,
                });
            }
        }
        self.renamed_span = start..end;
        if !searcher.match_tokens(END_TOKENS, &mut []) {
            self.errors.push(searcher.get_unexpected_err(file));
            #[expect(clippy::needless_return)]
            return;
        }
    }
}

#[track_caller]
fn get_vacant_lint<'a>(
    name: String,
    name_span: Span,
    lints: &'a mut FxHashMap<String, Lint>,
    errors: &mut Vec<Error>,
) -> Option<VacantEntry<'a, String, Lint>> {
    match lints.entry(name) {
        Entry::Vacant(e) => Some(e),
        Entry::Occupied(e) => {
            errors.push(
                ErrorKind::DuplicateLint {
                    name: e.key().clone(),
                    span: name_span,
                    prev_span: e.get().name_span,
                }
                .into(),
            );
            None
        },
    }
}

#[track_caller]
fn parse_str_lit(capture: Capture, text: &str, file: SourceFile, errors: &mut Vec<Error>) -> Option<String> {
    let s = &text[capture.to_index()];
    let (s, is_raw) = if let Some(s) = s.strip_prefix("r") {
        (s.trim_matches('#'), true)
    } else {
        (s, false)
    };
    let Some(s) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        errors.push(ErrorKind::StrLit(capture.to_span(file)).into());
        return None;
    };

    let mut no_err = true;
    let res = if is_raw {
        #[expect(clippy::cast_possible_truncation)]
        rustc_literal_escaper::check_raw_str(s, |sp, ch| {
            if let Err(e) = ch
                && e.is_fatal()
            {
                errors.push(
                    ErrorKind::StrLit(Span {
                        file,
                        start: capture.pos + sp.start as u32,
                        end: capture.pos + sp.end as u32,
                    })
                    .into(),
                );
                no_err = false;
            }
        });
        s.into()
    } else {
        let mut res = String::with_capacity(s.len());
        #[expect(clippy::cast_possible_truncation)]
        rustc_literal_escaper::unescape_str(s, |sp, ch| match ch {
            Ok(ch) => res.push(ch),
            Err(e) if e.is_fatal() => {
                errors.push(
                    ErrorKind::StrLit(Span {
                        file,
                        start: capture.pos + sp.start as u32,
                        end: capture.pos + sp.end as u32,
                    })
                    .into(),
                );
                no_err = false;
            },
            _ => {},
        });
        res
    };
    no_err.then_some(res)
}

#[track_caller]
fn parse_str_single_line(capture: Capture, text: &str, file: SourceFile, errors: &mut Vec<Error>) -> Option<String> {
    let s = parse_str_lit(capture, text, file, errors)?;
    if s.contains('\n') {
        errors.push(ErrorKind::StrLitEol(capture.to_span(file)).into());
        None
    } else {
        Some(s)
    }
}

fn is_lint_name(s: &str) -> bool {
    s.bytes()
        .all(|c| matches!(c, b'_' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z'))
}

#[track_caller]
fn parse_clippy_lint_name(capture: Capture, text: &str, file: SourceFile, errors: &mut Vec<Error>) -> Option<String> {
    let mut s = parse_str_lit(capture, text, file, errors)?;
    let Some(name) = s.strip_prefix("clippy::") else {
        errors.push(ErrorKind::LintMissingPrefix(capture.to_span(file)).into());
        return None;
    };
    if !is_lint_name(name) {
        errors.push(ErrorKind::NotLintName(capture.to_span(file)).into());
        return None;
    }
    s.drain(.."clippy::".len());
    Some(s)
}

#[track_caller]
fn parse_maybe_clippy_lint_name(
    capture: Capture,
    text: &str,
    file: SourceFile,
    errors: &mut Vec<Error>,
) -> Option<String> {
    let s = parse_str_lit(capture, text, file, errors)?;
    if !is_lint_name(s.strip_prefix("clippy::").unwrap_or(&s)) {
        errors.push(ErrorKind::NotLintName(capture.to_span(file)).into());
        return None;
    }
    Some(s)
}
