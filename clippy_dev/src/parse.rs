use crate::source_map::{SourceFile, SourceMap, Span};
use crate::utils::{ErrAction, expect_action, walk_dir_no_dot_or_target};
use core::ops::Range;
use core::slice;
use rustc_data_structures::fx::FxHashMap;
use rustc_lexer::{self as lexer, FrontmatterAllowed};
use std::fs;
use std::path::{self, Path};

#[derive(Clone, Copy)]
pub enum Token<'a> {
    /// Matches any number of comments / doc comments.
    AnyComment,
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
    Lifetime,
    Lt,
    Gt,
    OpenBrace,
    OpenBracket,
    OpenParen,
    Pound,
    Semi,
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
    pub fn peek_text(&self) -> &'txt str {
        &self.text[self.pos as usize..(self.pos + self.next_token.len) as usize]
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
    fn read_token(&mut self, token: Token<'_>, captures: &mut slice::IterMut<'_, &mut &'txt str>) -> bool {
        loop {
            match (token, self.next_token.kind) {
                (_, lexer::TokenKind::Whitespace)
                | (
                    Token::AnyComment,
                    lexer::TokenKind::BlockComment { terminated: true, .. } | lexer::TokenKind::LineComment { .. },
                ) => self.step(),
                (Token::AnyComment, _) => return true,
                (Token::Bang, lexer::TokenKind::Bang)
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
                ) => {
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
                    **captures.next().unwrap() = self.peek_text();
                    self.step();
                    return true;
                },
                _ => return false,
            }
        }
    }

    #[must_use]
    pub fn find_token(&mut self, token: Token<'_>) -> bool {
        let mut capture = [].iter_mut();
        while !self.read_token(token, &mut capture) {
            self.step();
            if self.at_end() {
                return false;
            }
        }
        true
    }

    #[must_use]
    pub fn find_capture_token(&mut self, token: Token<'_>) -> Option<&'txt str> {
        let mut res = "";
        let mut capture = &mut res;
        let mut capture = slice::from_mut(&mut capture).iter_mut();
        while !self.read_token(token, &mut capture) {
            self.step();
            if self.at_end() {
                return None;
            }
        }
        Some(res)
    }

    #[must_use]
    pub fn match_tokens(&mut self, tokens: &[Token<'_>], captures: &mut [&mut &'txt str]) -> bool {
        let mut captures = captures.iter_mut();
        tokens.iter().all(|&t| self.read_token(t, &mut captures))
    }
}

pub struct ActiveLint {
    pub group: String,
    pub span: Span,
}

pub struct DeprecatedLint {
    pub reason: String,
    pub version: String,
}

pub struct RenamedLint {
    pub new_name: String,
    pub version: String,
}

pub enum Lint {
    Active(ActiveLint),
    Deprecated(DeprecatedLint),
    Renamed(RenamedLint),
}

pub struct ParsedData {
    pub source_map: SourceMap,
    pub lints: FxHashMap<String, Lint>,
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
            deprecated_span: 0..0,
            renamed_span: 0..0,
        };
        parser.parse_src_files();
        parser.parse_deprecated_lints();

        ParsedData {
            source_map: parser.source_map,
            lints: parser.lints,
            deprecated_span: parser.deprecated_span,
            renamed_span: parser.renamed_span,
        }
    }
}

struct Parser {
    source_map: SourceMap,
    lints: FxHashMap<String, Lint>,
    deprecated_span: Range<u32>,
    renamed_span: Range<u32>,
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
        static DECL_TOKENS: &[Token<'_>] = &[
            // !{ /// docs
            Bang, OpenBrace, AnyComment,
            // #[clippy::version = "version"]
            Pound, OpenBracket, Ident("clippy"), DoubleColon, Ident("version"), Eq, LitStr, CloseBracket,
            // pub NAME, GROUP,
            Ident("pub"), CaptureIdent, Comma, AnyComment, CaptureIdent, Comma,
        ];

        let mut searcher = RustSearcher::new(&self.source_map.files[file].contents);
        #[expect(clippy::cast_possible_truncation)]
        while searcher.find_token(Ident("declare_clippy_lint")) {
            let start = searcher.pos() - "declare_clippy_lint".len() as u32;
            let (mut name, mut group) = ("", "");
            if searcher.match_tokens(DECL_TOKENS, &mut [&mut name, &mut group]) && searcher.find_token(CloseBrace) {
                self.lints.insert(
                    name.to_ascii_lowercase(),
                    Lint::Active(ActiveLint {
                        group: group.into(),
                        span: Span {
                            file,
                            start,
                            end: searcher.pos(),
                        },
                    }),
                );
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

        let krate = self.source_map.add_crate("clippy_lints");
        let file = self.source_map.load_file(
            Path::new("clippy_lints/src/deprecated_lints.rs"),
            krate,
            "deprecated_lints",
        );
        let file = &self.source_map.files[file];

        let mut searcher = RustSearcher::new(&file.contents);
        // First instance is the macro definition.
        assert!(
            searcher.find_token(Ident("declare_with_version")),
            "error parsing `clippy_lints/src/deprecated_lints.rs`"
        );

        if searcher.find_token(Ident("declare_with_version")) && searcher.match_tokens(DEPRECATED_TOKENS, &mut []) {
            let start = searcher.pos();
            let mut end = start;
            let mut version = "";
            let mut name = "";
            let mut reason = "";
            while searcher.match_tokens(DECL_TOKENS, &mut [&mut version, &mut name, &mut reason]) {
                self.lints.insert(
                    parse_clippy_lint_name(&file.path, name),
                    Lint::Deprecated(DeprecatedLint {
                        reason: parse_str_single_line(&file.path, reason),
                        version: parse_str_single_line(&file.path, version),
                    }),
                );
                end = searcher.pos();
            }
            self.deprecated_span = start..end;
        } else {
            panic!("error reading deprecated lints");
        }

        if searcher.find_token(Ident("declare_with_version")) && searcher.match_tokens(RENAMED_TOKENS, &mut []) {
            let start = searcher.pos();
            let mut end = start;
            let mut version = "";
            let mut old_name = "";
            let mut new_name = "";
            while searcher.match_tokens(DECL_TOKENS, &mut [&mut version, &mut old_name, &mut new_name]) {
                self.lints.insert(
                    parse_clippy_lint_name(&file.path, old_name),
                    Lint::Renamed(RenamedLint {
                        new_name: parse_maybe_clippy_lint_name(&file.path, new_name),
                        version: parse_str_single_line(&file.path, version),
                    }),
                );
                end = searcher.pos();
            }
            self.renamed_span = start..end;
        } else {
            panic!("error reading renamed lints");
        }
    }
}

fn assert_lint_name(path: &Path, s: &str) {
    assert!(
        s.bytes()
            .all(|c| matches!(c, b'_' | b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z')),
        "error parsing `{}`: `{s}` is not a valid lint name",
        path.display(),
    );
}

fn parse_str_lit(s: &str) -> String {
    let (s, is_raw) = if let Some(s) = s.strip_prefix("r") {
        (s.trim_matches('#'), true)
    } else {
        (s, false)
    };
    let s = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or_else(|| panic!("expected quoted string, found `{s}`"));

    if is_raw {
        s.into()
    } else {
        let mut res = String::with_capacity(s.len());
        rustc_literal_escaper::unescape_str(s, &mut |_, ch| {
            if let Ok(ch) = ch {
                res.push(ch);
            }
        });
        res
    }
}

fn parse_str_single_line(path: &Path, s: &str) -> String {
    let s = parse_str_lit(s);
    assert!(
        !s.contains('\n'),
        "error parsing `{}`: `{s}` should be a single line string",
        path.display(),
    );
    s
}

fn parse_clippy_lint_name(path: &Path, s: &str) -> String {
    let mut s = parse_str_lit(s);
    let Some(name) = s.strip_prefix("clippy::") else {
        panic!(
            "error parsing `{}`: `{s}` needs to have the `clippy::` prefix",
            path.display()
        );
    };
    assert_lint_name(path, name);
    s.drain(.."clippy::".len());
    s
}

fn parse_maybe_clippy_lint_name(path: &Path, s: &str) -> String {
    let s = parse_str_lit(s);
    assert_lint_name(path, s.strip_prefix("clippy::").unwrap_or(&s));
    s
}
