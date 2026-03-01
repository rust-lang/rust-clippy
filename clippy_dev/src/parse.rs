pub mod cursor;

use self::cursor::{Capture, Cursor, IdentPat, UnexpectedErr};
use crate::ir::{
    ActiveLintData, ConfDef, ConfOpt, DeprecatedLintData, Lint, LintData, LintMap, LintName, LintPass, LintPassCtor,
    LintPassCtorArg, LintPassCtorArgs, LintPassMac, LintPasses, LintTool, ParsedLints, RenamedLintData,
};
use crate::utils::{ErrAction, Scoped, StrBuf, VecBuf, expect_action, walk_dir_no_dot_or_target};
use crate::{DiagCx, SourceFile, Span};
use core::panic::Location;
use rustc_arena::{DroplessArena, TypedArena};
use rustc_data_structures::fx::FxHashMap;
use std::collections::hash_map::{Entry, VacantEntry};
use std::{fs, path};

pub struct ParseCxImpl<'cx> {
    pub arena: &'cx DroplessArena,
    pub source_files: &'cx TypedArena<SourceFile<'cx>>,
    pub str_buf: StrBuf,
    pub str_list_buf: VecBuf<&'cx str>,
    pub dcx: DiagCx,
}
pub type ParseCx<'cx> = &'cx mut ParseCxImpl<'cx>;

/// Calls the given function inside a newly created parsing context.
pub fn new_parse_cx<'env, T>(f: impl for<'cx> FnOnce(&'cx mut Scoped<'cx, 'env, ParseCxImpl<'cx>>) -> T) -> T {
    let arena = DroplessArena::default();
    let source_files = TypedArena::default();
    f(&mut Scoped::new(ParseCxImpl {
        arena: &arena,
        source_files: &source_files,
        str_buf: StrBuf::with_capacity(128),
        str_list_buf: VecBuf::with_capacity(128),
        dcx: DiagCx::default(),
    }))
}

#[derive(Clone, Copy)]
enum PassTrait {
    EarlyLintPass,
    LateLintPass,
    Default,
}
impl PassTrait {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "EarlyLintPass" => Some(Self::EarlyLintPass),
            "LateLintPass" => Some(Self::LateLintPass),
            "Default" => Some(Self::Default),
            _ => None,
        }
    }
}

/// Parsed impl block of a lint pass.
#[derive(Clone, Copy)]
enum PassImplKind {
    Trait(PassTrait),
    New(LintPassCtorArgs),
    UnexpectedErr(UnexpectedErr<'static>),
    SpannedErr(Capture, &'static str, &'static Location<'static>),
}

#[derive(Clone, Copy)]
struct PassImpl<'cx> {
    ty: &'cx str,
    kind: PassImplKind,
}

impl<'cx> ParseCxImpl<'cx> {
    #[track_caller]
    fn get_vacant_lint<'map>(
        &mut self,
        map: &'map mut LintMap<'cx>,
        name: &'cx str,
        name_sp: Span<'cx>,
    ) -> Option<VacantEntry<'map, &'cx str, Lint<'cx>>> {
        match map.entry(name) {
            Entry::Vacant(e) => Some(e),
            Entry::Occupied(e) => {
                self.dcx.emit_duplicate_lint(name_sp, e.get().name_sp);
                None
            },
        }
    }

    fn add_impl_to_pass(&mut self, file: &'cx SourceFile<'cx>, impl_: &PassImplKind, pass: &mut LintPass<'_>) {
        match impl_ {
            PassImplKind::Trait(PassTrait::EarlyLintPass) => pass.is_early = true,
            PassImplKind::Trait(PassTrait::LateLintPass) => pass.is_late = true,
            PassImplKind::Trait(PassTrait::Default) => pass.ctor.add_default(),
            &PassImplKind::New(args) => pass.ctor = LintPassCtor::New(args),
            PassImplKind::UnexpectedErr(e) => e.emit(&mut self.dcx, file),
            &PassImplKind::SpannedErr(capture, msg, loc) => {
                self.dcx.emit_spanned_err_loc(capture.mk_sp(file), msg, loc);
            },
        }
    }

    pub fn parse_conf_mac(&mut self) -> ConfDef<'cx> {
        #[allow(clippy::enum_glob_use)]
        use cursor::Pat::*;

        let file = &*self.source_files.alloc(SourceFile::load(self.str_buf.alloc_collect(
            self.arena,
            [
                "clippy_config",
                path::MAIN_SEPARATOR_STR,
                "src",
                path::MAIN_SEPARATOR_STR,
                "conf.rs",
            ],
        )));

        let mut data = ConfDef {
            decl_sp: Span::new(file, 0..0),
            opts: Vec::with_capacity(100),
        };
        let mut cursor = Cursor::new(&file.contents);
        let mut captures = [Capture::EMPTY; 1];

        if let Err(expected) = cursor
            .find_mac_call("define_Conf")
            .ok_or("`define_Conf!`")
            .and_then(|name| {
                data.decl_sp.range.start = name.pos;
                cursor.eat_open_brace().ok_or("`{`")
            })
            .and_then(|()| {
                cursor.eat_list(|cursor| {
                    let docs = cursor.capture_doc_lines();
                    let mut lints: &mut [_] = &mut [];
                    let mut lints_range = None;
                    let mut started = docs.len != 0;
                    while let Some((attr_start, name)) = cursor.capture_opt_attr_start()? {
                        started = true;
                        if cursor.get_text(name) == "lints" {
                            cursor
                                .eat_open_paren()
                                .ok_or("`(`")
                                .and_then(|()| {
                                    cursor.capture_list(&mut self.str_list_buf, self.arena, |cursor| {
                                        Ok(cursor.capture_ident().map(|x| cursor.get_text(x)))
                                    })
                                })
                                .and_then(|res| {
                                    lints = res;
                                    cursor.match_all(&[CloseParen, CloseBracket], &mut [])
                                })?;
                            lints_range = Some(attr_start..cursor.pos());
                        } else {
                            cursor.find_close_bracket().ok_or("`]`")?;
                        }
                    }
                    match cursor.opt_match_all(&[CaptureIdent, Colon], &mut captures) {
                        Ok(true) => {},
                        Ok(false) if started => return Err("an identifier"),
                        Ok(false) => return Ok(false),
                        Err(e) => return Err(e),
                    }
                    cursor.find_eq().ok_or("`=`")?;
                    cursor.eat_list_item();
                    data.opts.push(ConfOpt {
                        name: cursor.get_text(captures[0]),
                        decl_range: docs.pos..cursor.pos(),
                        lints,
                        lints_range: lints_range.unwrap_or(captures[0].pos..captures[0].pos),
                    });
                    Ok(true)
                })
            })
            .and_then(|()| cursor.eat_close_brace().ok_or("`}`"))
        {
            cursor.mk_unexpected_err(expected).emit(&mut self.dcx, file);
        }

        data.decl_sp.range.end = cursor.pos();
        data
    }

    /// Finds and parses all lint declarations.
    pub fn parse_lint_decls(&mut self) -> ParsedLints<'cx> {
        let mut data = ParsedLints {
            #[expect(clippy::default_trait_access)]
            lints: LintMap(FxHashMap::with_capacity_and_hasher(1000, Default::default())),
            lint_passes: LintPasses(Vec::with_capacity(400)),
            deprecated_file: self.source_files.alloc(SourceFile::load(self.str_buf.alloc_collect(
                self.arena,
                [
                    "clippy_lints",
                    path::MAIN_SEPARATOR_STR,
                    "src",
                    path::MAIN_SEPARATOR_STR,
                    "deprecated_lints.rs",
                ],
            ))),
        };

        for e in expect_action(fs::read_dir("."), ErrAction::Read, ".") {
            let e = expect_action(e, ErrAction::Read, ".");

            // Skip if this isn't a lint crate's directory.
            let mut crate_path = if expect_action(e.file_type(), ErrAction::Read, ".").is_dir()
                && let Ok(crate_path) = e.file_name().into_string()
                && crate_path.starts_with("clippy_lints")
                && crate_path != "clippy_lints_internal"
            {
                crate_path
            } else {
                continue;
            };

            crate_path.push(path::MAIN_SEPARATOR);
            crate_path.push_str("src");
            for e in walk_dir_no_dot_or_target(&crate_path) {
                let e = expect_action(e, ErrAction::Read, &crate_path);
                if e.path().as_os_str().as_encoded_bytes().ends_with(b".rs")
                    && let Some(file_path) = e.path().to_str()
                    && file_path != data.deprecated_file.path.get()
                {
                    let file = self
                        .source_files
                        .alloc(SourceFile::load(self.arena.alloc_str(file_path)));
                    self.parse_lint_src_file(&mut data, file);
                }
            }
        }

        self.parse_deprecated_lints(&mut data);
        data
    }

    /// Parse a source file looking for `declare_clippy_lint` macro invocations.
    #[expect(clippy::too_many_lines)]
    fn parse_lint_src_file(&mut self, data: &mut ParsedLints<'cx>, file: &'cx SourceFile<'cx>) {
        #[allow(clippy::enum_glob_use)]
        use cursor::Pat::*;

        let mut cursor = Cursor::new(&file.contents);
        let mut captures = [Capture::EMPTY; 6];
        let mut pass_impls = Vec::new();
        let mut has_derive_default = false;
        let first_lint_pass = data.lint_passes.len();

        while let Some(mac_name) = cursor.find_capture_ident() {
            match cursor.get_text(mac_name) {
                "declare_clippy_lint" if cursor.eat_bang() => {
                    #[rustfmt::skip]
                    static DECL_START: &[cursor::Pat] = &[
                        // { /// docs
                        OpenBrace, CaptureDocLines,
                        // #[clippy::version = "version"]
                        Pound, OpenBracket, Ident(IdentPat::clippy), DoubleColon,
                        Ident(IdentPat::version), Eq, CaptureLitStr, CloseBracket,
                        // pub NAME, GROUP, "desc",
                        Ident(IdentPat::r#pub), CaptureIdent, Comma,
                        CaptureLineComments, CaptureIdent, Comma, CaptureLitStr,
                    ];
                    #[rustfmt::skip]
                    static OPTION: &[cursor::Pat] = &[
                        // @option = value
                        AnyComments, At, AnyIdent, Eq, Lit,
                    ];

                    let mut opts_text = "";
                    if let Err(expected) = cursor
                        .match_all(DECL_START, &mut captures)
                        .and_then(|()| {
                            if cursor.eat_comma() {
                                let pos = cursor.pos();
                                cursor.eat_list(|cursor| cursor.match_all(OPTION, &mut []).map(|()| true))?;
                                opts_text = file.contents[pos as usize..cursor.pos() as usize].trim();
                            }
                            Ok(())
                        })
                        .and_then(|()| cursor.eat_close_brace().ok_or("`}`"))
                    {
                        cursor.mk_unexpected_err(expected).emit(&mut self.dcx, file);
                    } else if let [docs, version, name, group_comments, group, desc] = captures
                        && let name_sp = name.mk_sp(file)
                        && let name = self.str_buf.alloc_ascii_lower(self.arena, cursor.get_text(name))
                        && let (Some(e), Some(version)) = (
                            self.get_vacant_lint(&mut data.lints, name, name_sp),
                            self.parse_version(cursor.get_text(version), version.mk_sp(file)),
                        )
                    {
                        e.insert(Lint {
                            name_sp,
                            version,
                            data: LintData::Active(ActiveLintData {
                                decl_range: mac_name.pos..cursor.pos(),
                                docs: cursor.get_text(docs),
                                group_comments: cursor.get_text(group_comments),
                                group: cursor.get_text(group),
                                desc: cursor.get_text(desc),
                                opts: opts_text,
                            }),
                        });
                    }
                },
                mac @ ("declare_lint_pass" | "impl_lint_pass") if cursor.eat_bang() => {
                    let mut has_lt = false;
                    let mut lints: &mut [_] = &mut [];
                    if let Err(expected) = cursor
                        .match_all(&[OpenParen, CaptureDocLines, CaptureIdent], &mut captures)
                        .and_then(|()| cursor.opt_match_all(&[Lt, CaptureLifetime, Gt], &mut captures[2..]))
                        .and_then(|res| {
                            has_lt = res;
                            cursor.match_all(&[FatArrow, OpenBracket], &mut [])
                        })
                        .and_then(|()| {
                            cursor.capture_list(&mut self.str_list_buf, self.arena, |cursor| {
                                cursor.capture_opt_path(&mut self.str_buf, self.arena)
                            })
                        })
                        .and_then(|res| {
                            lints = res;
                            cursor.match_all(&[CloseBracket, CloseParen, Semi], &mut [])
                        })
                    {
                        cursor.mk_unexpected_err(expected).emit(&mut self.dcx, file);
                    } else {
                        data.lint_passes.push(LintPass {
                            docs: cursor.get_text(captures[0]),
                            name: cursor.get_text(captures[1]),
                            lt: has_lt.then(|| cursor.get_text(captures[2])),
                            mac: if matches!(mac, "declare_lint_pass") {
                                LintPassMac::Declare
                            } else {
                                LintPassMac::Impl
                            },
                            decl_sp: Span::new(file, mac_name.pos..cursor.pos()),
                            lints,
                            ctor: LintPassCtor::Unit,
                            is_early: false,
                            is_late: false,
                        });
                    }
                },
                "impl" if let Some(impl_) = self.parse_lint_impl(file, &mut cursor) => {
                    match data.lint_passes[first_lint_pass..]
                        .iter_mut()
                        .find(|pass| pass.name == impl_.ty)
                    {
                        Some(pass) => self.add_impl_to_pass(file, &impl_.kind, pass),
                        None => pass_impls.push(impl_),
                    }
                },
                "derive" if cursor.eat_open_paren() => {
                    while let Some(name) = cursor.capture_ident() {
                        if cursor.get_text(name) == "Default" {
                            has_derive_default = true;
                            break;
                        }
                        if !cursor.eat_comma() {
                            break;
                        }
                    }
                    let _ = cursor.find_unnested_close_paren();
                },
                "struct" if has_derive_default => {
                    has_derive_default = false;
                    if let Some(ty) = cursor.capture_ident() {
                        let ty = cursor.get_text(ty);
                        if let Some(pass) = data.lint_passes[first_lint_pass..]
                            .iter_mut()
                            .find(|pass| pass.name == ty)
                        {
                            pass.ctor.add_default();
                        } else {
                            pass_impls.push(PassImpl {
                                ty,
                                kind: PassImplKind::Trait(PassTrait::Default),
                            });
                        }
                    }
                },
                "enum" => has_derive_default = false,
                _ => {},
            }
        }

        for impl_ in &pass_impls {
            if let Some(pass) = data.lint_passes[first_lint_pass..]
                .iter_mut()
                .find(|pass| pass.name == impl_.ty)
            {
                self.add_impl_to_pass(file, &impl_.kind, pass);
            }
        }
    }

    fn parse_lint_impl(&mut self, file: &'cx SourceFile<'cx>, cursor: &mut Cursor<'cx>) -> Option<PassImpl<'cx>> {
        #[allow(clippy::enum_glob_use)]
        use cursor::Pat::*;

        cursor.opt_match_all(&[Lt, Lifetime, Gt], &mut []).ok()?;
        let name = cursor.capture_ident().map(|c| cursor.get_text(c))?;
        match PassTrait::from_str(name) {
            Some(trait_) => {
                let pats: &[_] = match trait_ {
                    PassTrait::LateLintPass => &[Lt, Lifetime, Gt, Ident(IdentPat::r#for)],
                    PassTrait::EarlyLintPass | PassTrait::Default => &[Ident(IdentPat::r#for)],
                };
                if let Err(expected) = cursor.match_all(pats, &mut []) {
                    cursor.mk_unexpected_err(expected).emit(&mut self.dcx, file);
                    None
                } else {
                    cursor
                        .capture_ident()
                        .filter(|_| {
                            cursor.opt_match_all(&[Lt, Lifetime, Gt], &mut []).is_ok() && cursor.eat_open_brace()
                        })
                        .map(|name| PassImpl {
                            ty: cursor.get_text(name),
                            kind: PassImplKind::Trait(trait_),
                        })
                }
            },
            None if cursor.opt_match_all(&[Lt, Lifetime, Gt], &mut []).is_ok() && cursor.eat_open_brace() => {
                while cursor.find_unnested_ident("fn") {
                    if !cursor.eat_ident("new") {
                        continue;
                    }
                    if !cursor.eat_open_paren() {
                        return Some(PassImpl {
                            ty: name,
                            kind: PassImplKind::UnexpectedErr(cursor.mk_unexpected_err("`(`")),
                        });
                    }
                    let mut args = LintPassCtorArgs::default();
                    let res = cursor.eat_list(|cursor| {
                        if !cursor.find_unnested_colon() {
                            return Ok(false);
                        }
                        let _ = cursor.eat_and() && cursor.eat_lifetime();
                        let Some(ty) = cursor.capture_ident() else {
                            return Err(PassImplKind::UnexpectedErr(cursor.mk_unexpected_err("an identifier")));
                        };
                        let Some(arg) = LintPassCtorArg::from_str(cursor.get_text(ty)) else {
                            return Err(PassImplKind::SpannedErr(ty, "unexpected parameter type, expected `TyCtxt`, `Conf`, `FormatArgsStorage` or `AttrStorage`", Location::caller()));
                        };
                        if args.try_push(arg).is_err() {
                            return Err(PassImplKind::SpannedErr(ty, "duplicate parameter type", Location::caller()));
                        }
                        cursor.eat_list_item();
                        Ok(true)
                    });
                    let kind = match res {
                        Ok(()) => {
                            match cursor
                                .eat_close_paren()
                                .ok_or("`(`")
                                .and_then(|()| cursor.find_unnested_close_brace().ok_or("`}`"))
                            {
                                Ok(()) => PassImplKind::New(args),
                                Err(expected) => PassImplKind::UnexpectedErr(cursor.mk_unexpected_err(expected)),
                            }
                        },
                        Err(kind) => {
                            let _ = cursor.find_unnested_close_paren() && cursor.find_unnested_close_brace();
                            kind
                        },
                    };
                    return Some(PassImpl { ty: name, kind });
                }
                None
            },
            None => None,
        }
    }

    fn parse_deprecated_lints(&mut self, data: &mut ParsedLints<'cx>) {
        #[allow(clippy::enum_glob_use)]
        use cursor::Pat::*;

        #[rustfmt::skip]
        static DECL_TOKENS: &[cursor::Pat] = &[
            // #[clippy::version = "version"]
            Pound, OpenBracket, Ident(IdentPat::clippy), DoubleColon,
            Ident(IdentPat::version), Eq, CaptureLitStr, CloseBracket,
            // ("first", "second")
            OpenParen, CaptureLitStr, Comma, CaptureLitStr, CloseParen,
        ];
        #[rustfmt::skip]
        static DEPRECATED_TOKENS: &[cursor::Pat] = &[
            // !{ DEPRECATED(DEPRECATED_VERSION) = [
            Bang, OpenBrace, Ident(IdentPat::DEPRECATED), OpenParen,
            Ident(IdentPat::DEPRECATED_VERSION), CloseParen, Eq, OpenBracket,
        ];
        #[rustfmt::skip]
        static RENAMED_TOKENS: &[cursor::Pat] = &[
            // !{ RENAMED(RENAMED_VERSION) = [
            Bang, OpenBrace, Ident(IdentPat::RENAMED), OpenParen,
            Ident(IdentPat::RENAMED_VERSION), CloseParen, Eq, OpenBracket,
        ];

        let file = data.deprecated_file;
        let mut cursor = Cursor::new(&file.contents);
        let mut captures = [Capture::EMPTY; 3];

        if let Err(expected) = cursor
            .find_ident("declare_with_version")
            .ok_or("`declare_with_version`")
            .and_then(|()| {
                cursor
                    .find_ident("declare_with_version")
                    .ok_or("`declare_with_version`")
            })
            .and_then(|()| cursor.match_all(DEPRECATED_TOKENS, &mut []))
            .and_then(|()| {
                cursor.eat_list(|cursor| {
                    let parsed = cursor.opt_match_all(DECL_TOKENS, &mut captures)?;
                    let name_sp = captures[1].mk_sp(file);
                    if parsed
                        && let (Some(version), Some(name), Some(reason)) = (
                            self.parse_version(cursor.get_text(captures[0]), captures[0].mk_sp(file)),
                            self.parse_clippy_lint_name(cursor.get_text(captures[1]), name_sp),
                            self.parse_str_lit(cursor.get_text(captures[2]), captures[0].mk_sp(file)),
                        )
                        && let Some(e) = self.get_vacant_lint(&mut data.lints, name, name_sp)
                    {
                        e.insert(Lint {
                            name_sp,
                            version,
                            data: LintData::Deprecated(DeprecatedLintData { reason }),
                        });
                    }
                    Ok(parsed)
                })
            })
            .and_then(|()| {
                cursor
                    .find_ident("declare_with_version")
                    .ok_or("`declare_with_version`")
            })
            .and_then(|()| cursor.match_all(RENAMED_TOKENS, &mut []))
            .and_then(|()| {
                cursor.eat_list(|cursor| {
                    let parsed = cursor.opt_match_all(DECL_TOKENS, &mut captures)?;
                    let name_sp = captures[1].mk_sp(file);
                    if parsed
                        && let (Some(version), Some(name), Some(new_name)) = (
                            self.parse_version(cursor.get_text(captures[0]), captures[0].mk_sp(file)),
                            self.parse_clippy_lint_name(cursor.get_text(captures[1]), name_sp),
                            self.parse_lint_name(cursor.get_text(captures[2]), captures[0].mk_sp(file)),
                        )
                        && let Some(e) = self.get_vacant_lint(&mut data.lints, name, name_sp)
                    {
                        e.insert(Lint {
                            name_sp,
                            version,
                            data: LintData::Renamed(RenamedLintData { new_name }),
                        });
                    }
                    Ok(parsed)
                })
            })
        {
            cursor.mk_unexpected_err(expected).emit(&mut self.dcx, file);
        }
    }

    /// Removes the line splices and surrounding quotes from a string literal.
    fn parse_str_lit(&mut self, s: &'cx str, sp: Span<'cx>) -> Option<&'cx str> {
        let (s, is_raw, sp_base) = if let Some(trimmed) = s.strip_prefix("r") {
            let trimmed = trimmed.trim_start_matches('#');
            #[expect(clippy::cast_possible_truncation)]
            let sp_base = (s.len() - trimmed.len() + 1) as u32;
            (trimmed.trim_end_matches('#'), true, sp_base)
        } else {
            (s, false, 1)
        };
        let sp_base = sp.range.start + sp_base;
        let s = s
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or_else(|| panic!("expected quoted string, found `{s}`"));

        let mut is_ok = true;
        if is_raw {
            rustc_literal_escaper::check_raw_str(s, |range, c| {
                if c.is_err_and(|e| e.is_fatal()) {
                    #[expect(clippy::cast_possible_truncation)]
                    let range = range.start as u32 + sp_base..range.end as u32 + sp_base;
                    self.dcx
                        .emit_spanned_err(Span::new(sp.file, range), "invalid string escape sequence");
                    is_ok = false;
                }
            });
            is_ok.then_some(s)
        } else {
            self.str_buf.with(|buf| {
                rustc_literal_escaper::unescape_str(s, |range, c| match c {
                    Ok(c) => buf.push(c),
                    Err(e) if e.is_fatal() => {
                        #[expect(clippy::cast_possible_truncation)]
                        let range = range.start as u32 + sp_base..range.end as u32 + sp_base;
                        self.dcx
                            .emit_spanned_err(Span::new(sp.file, range), "invalid string escape sequence");
                        is_ok = false;
                    },
                    Err(_) => {},
                });
                is_ok.then(|| {
                    if buf == s {
                        s
                    } else if buf.is_empty() {
                        ""
                    } else {
                        self.arena.alloc_str(buf)
                    }
                })
            })
        }
    }

    #[track_caller]
    fn parse_lint_name(&mut self, s: &'cx str, sp: Span<'cx>) -> Option<LintName<'cx>> {
        let s = self.parse_str_lit(s, sp)?;
        let (tool, name) = match s.split_once("::") {
            Some((tool, name)) if let Some(tool) = LintTool::from_prefix(tool) => (tool, name),
            Some(..) => {
                self.dcx.emit_spanned_err(sp, "unknown lint tool");
                return None;
            },
            None => (LintTool::Rustc, s),
        };
        if name
            .bytes()
            .all(|c| matches!(c, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
        {
            Some(LintName { tool, name })
        } else {
            self.dcx.emit_spanned_err(sp, "unparsable lint name");
            None
        }
    }

    #[track_caller]
    fn parse_clippy_lint_name(&mut self, s: &'cx str, sp: Span<'cx>) -> Option<&'cx str> {
        let name = self.parse_lint_name(s, sp)?;
        if name.tool == LintTool::Clippy {
            Some(name.name)
        } else {
            self.dcx.emit_not_clippy_lint_name(sp);
            None
        }
    }

    #[track_caller]
    fn parse_version(&mut self, s: &'cx str, sp: Span<'cx>) -> Option<&'cx str> {
        let s = self.parse_str_lit(s, sp)?;
        if s.bytes().all(|c| matches!(c, b'0'..=b'9' | b'.')) || matches!(s, "pre 1.29.0" | "CURRENT_RUSTC_VERSION") {
            Some(s)
        } else {
            self.dcx.emit_spanned_err(sp, "unparsable version number");
            None
        }
    }
}
