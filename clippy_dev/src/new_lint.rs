use crate::generate::gen_sorted_lints_file;
use crate::parse::cursor::Cursor;
use crate::parse::{ActiveLint, ActiveLintData, Lint, LintData, LintPass, LintPassMac};
use crate::utils::{FileUpdater, VecBuf, Version, create_new_dir};
use crate::{SourceFile, Span, UpdateMode, new_parse_cx};
use rustc_lexer::{DocStyle, TokenKind};
use std::collections::hash_map::Entry;
use std::path::{self, MAIN_SEPARATOR_STR as PATH_SEP, PathBuf};

#[derive(Clone, Copy)]
enum LintPassKind {
    Early,
    Late,
}

/// Creates the files required to implement and test a new lint and runs `update_lints`.
///
/// # Errors
///
/// This function errors out if the files couldn't be created or written to.
#[expect(clippy::too_many_lines)]
pub fn create(clippy_version: Version, pass: &str, name: &str, group: &str, has_msrv: bool) {
    new_parse_cx(|cx| {
        let cx = &mut **cx;
        let mut data = cx.parse_lint_decls();
        let conf_data = has_msrv.then(|| cx.parse_conf_mac());
        match (pass, group) {
            ("cargo", "cargo") => {},
            ("cargo", _) => cx
                .dcx
                .emit_spanless_err("a lint added to the `cargo` pass must be part of the `cargo` group"),
            (_, "cargo") => cx
                .dcx
                .emit_spanless_err("a lint added to the `cargo` group must be part of the `cargo` pass"),
            _ => {},
        }
        let (pass_idx, new_pass) = match pass {
            "early" => (None, LintPassKind::Early),
            "late" => (None, LintPassKind::Late),
            _ => {
                let pass_name = cx.str_buf.alloc_kebab_to_pascal(cx.arena, pass);
                let pass_idx = data.lint_passes.iter().position(|p| p.name == pass_name);
                if pass_idx.is_none() {
                    cx.dcx.emit_spanless_err(format!("unknown lint pass `{pass}`"));
                }
                (pass_idx, LintPassKind::Early)
            },
        };
        let name_snake = cx.str_buf.alloc_kebab_to_snake(cx.arena, name);
        let Entry::Vacant(vacant_lint) = data.lints.entry(name_snake) else {
            cx.dcx.emit_unknown_lint(name);
            cx.dcx.exit_assume_err();
        };
        cx.dcx.exit_on_err();

        let name_pascal = cx.str_buf.alloc_kebab_to_pascal(cx.arena, name);
        let name_upper = cx.str_buf.alloc_ascii_upper(cx.arena, name_snake);
        let version = cx.str_buf.alloc_display(cx.arena, clippy_version.rust_display());
        let mut lint_data = ActiveLintData {
            decl_range: 0..0,
            docs: if group == "restriction" {
                RESTRICTION_DESC
            } else {
                DEFAULT_DESC
            },
            group_comments: "",
            group,
            desc: r#""default lint description""#,
            opts: "",
        };

        let mut updater = FileUpdater::default();

        // Edit clippy source to add the new lint.
        if let Some(pass_idx) = pass_idx {
            let lint_pass = &mut data.lint_passes[pass_idx];
            let file = lint_pass.decl_sp.file;
            let is_late_pass = lint_pass.is_late;

            lint_pass.lints = cx.str_list_buf.with(|buf| {
                buf.extend(lint_pass.lints.iter().copied());
                buf.push(name_upper);
                cx.arena.alloc_slice(buf)
            });
            lint_data.decl_range = lint_pass.decl_sp.range.end..lint_pass.decl_sp.range.end;
            vacant_lint.insert(Lint {
                name_sp: Span::new(file, lint_data.decl_range),
                version,
                data: LintData::Active(lint_data),
            });

            let add_mod = if let Some((path, "mod.rs" | "lib.rs")) = file.path.get().rsplit_once(path::MAIN_SEPARATOR) {
                updater.write_new_file(String::from_iter([path, PATH_SEP, name_snake, ".rs"]), |dst| {
                    write_lint_check_file(dst, name_upper, is_late_pass, has_msrv);
                });
                true
            } else {
                false
            };
            updater.change_loaded_file(file, |src, dst| {
                let mut lints: Vec<_> = data.lints.lints_in_file(file).collect();
                let passes = data.lint_passes.in_same_file_as_mut(pass_idx);
                let mut ranges = VecBuf::with_capacity(lints.len() + passes.len());
                let mut copy = mk_sorted_lints_copy_fn(add_mod, name_snake);
                gen_sorted_lints_file(src, dst, &mut lints, passes, &mut ranges, &mut copy);
            });
        } else {
            // Create a new lint pass.
            let path = cx
                .str_buf
                .alloc_collect(cx.arena, ["clippy_lints", PATH_SEP, "src", PATH_SEP, name, ".rs"]);
            let file = cx.source_files.alloc(SourceFile::new_empty(path));
            vacant_lint.insert(Lint {
                name_sp: Span::new(file, 0..0),
                version,
                data: LintData::Active(lint_data),
            });

            updater.write_new_file(path, |dst| {
                write_lint_file(
                    dst,
                    &ActiveLint {
                        name: name_snake,
                        version,
                        data: &lint_data,
                    },
                    &LintPass {
                        docs: "",
                        name: name_pascal,
                        lt: None,
                        mac: if has_msrv {
                            LintPassMac::Impl
                        } else {
                            LintPassMac::Declare
                        },
                        decl_sp: Span::new(file, 0..0),
                        lints: cx.arena.alloc_slice(&[name_upper]),
                        is_early: matches!(new_pass, LintPassKind::Early),
                        is_late: matches!(new_pass, LintPassKind::Late),
                    },
                    has_msrv,
                );
            });
            updater.change_file("clippy_lints/src/lib.rs", |src, dst| {
                add_lint_pass(src, dst, name_snake, name_pascal, new_pass, has_msrv);
            });
        }

        // Register the lint with the MSRV option.
        if let Some(mut data) = conf_data
            && let Some(opt) = data.opts.iter_mut().find(|x| x.name == "msrv")
        {
            opt.lints = cx.str_list_buf.with(|buf| {
                buf.extend(opt.lints.iter().copied());
                buf.push(name_snake);
                cx.arena.alloc_slice(buf)
            });
            updater.change_loaded_file(data.decl_sp.file, |src, dst| data.gen_file(src, dst));
        }

        // Create test files.
        if group == "cargo" {
            let mut path = PathBuf::from_iter(["tests", "ui-cargo", name_snake]);
            create_new_dir(&path);

            let mut mk_project = |name: &str, todo: &str| {
                path.push(name);
                create_new_dir(&path);
                path.push("Cargo.toml");
                updater.write_new_file(&path, |dst| write_cargo_manifest(dst, name_snake, todo));
                path.pop();
                path.push("src");
                create_new_dir(&path);
                path.push("main.rs");
                updater.write_new_file(&path, |dst| write_test_file(dst, name_snake, has_msrv));
                path.pop();
                path.pop();
                path.pop();
            };
            mk_project("pass", "Add contents that should pass");
            mk_project("fail", "Add contents the should fail");
        } else {
            updater.write_new_file(
                String::from_iter(["tests", PATH_SEP, "ui", PATH_SEP, name_snake, ".rs"]),
                |dst| write_test_file(dst, name_snake, has_msrv),
            );
        }

        data.gen_decls(UpdateMode::Change);
    });
}

static DEFAULT_DESC: &str = "\
/// ### What it does
///
/// ### Why is this bad?
///
/// ### Example
/// ```no_run
/// // example code where clippy issues a warning
/// ```
/// Use instead:
/// ```no_run
/// // example code which does not raise clippy warning
/// ```";

static RESTRICTION_DESC: &str = "\
/// ### What it does
///
/// ### Why restrict this?
///
/// ### Example
/// ```no_run
/// // example code where clippy issues a warning
/// ```
/// Use instead:
/// ```no_run
/// // example code which does not raise clippy warning
/// ```";

#[rustfmt::skip]
fn write_lint_check_file(dst: &mut String, name_upper: &str, is_late_pass: bool, has_msrv: bool) {
    let (cx_ty, cx_lt, msrv_arg, msrv_import) = if is_late_pass {
        ("LateContext", "<'_>", ", msrv: Msrv", "use clippy_utils::msrvs::{self, Msrv};\n")
    } else {
        ("EarlyContext", "", ", msrv: &MsrvStack", "use clippy_utils::msrvs::{self, MsrvStack};\n")
    };
    let (msrv_arg, msrv_import) = if has_msrv { (msrv_arg, msrv_import) } else { ("", "") };

    dst.extend([
msrv_import, "use rustc_lint::", cx_ty, ";

use super::", name_upper, ";

pub(super) fn check(cx: &", cx_ty, cx_lt, msrv_arg, ") {
    todo!(\"implement lint logic\");
}
"]);
}

#[rustfmt::skip]
fn write_test_file(dst: &mut String, name: &str, has_msrv: bool) {
    let msrv_contents = if has_msrv {
        "

    // TODO: set `xx` to on below the required MSRV and `yy` to the required MSRV.
    #[clippy::msrv = \"1.xx\"]
    {
        // TODO: test which requires the msrv to be set
    };
    #[clippy::msrv =\"1.yy\"]
    {
        // TODO: same test as above
    }
"
    } else {
        ""
    };

    dst.extend(["\
#![warn(clippy::", name, ")]

fn main() {{
    // TODO: fill in tests", msrv_contents, "
}}
"]);
}

#[rustfmt::skip]
fn write_cargo_manifest(dst: &mut String, name: &str, todo: &str) {
    dst.extend(["\
[package]
name = \"", name, "\"
version = \"0.1.0\"
publish = false

[workspace]

# TODO: \n", todo, "
"]);
}

#[rustfmt::skip]
fn write_lint_file(dst: &mut String, lint: &ActiveLint<'_, '_>, pass: &LintPass<'_>, has_msrv: bool) {
    let (pass_ty, pass_lt, cx_ty, msrv_ty, msrv_ctor, extract_msrv) = if pass.is_late {
        ("LateLintPass", "<'_>", "LateContext", "Msrv", "conf.msrv.into()", "")
    } else {
        ("EarlyLintPass", "", "EarlyContext", "MsrvStack", "MsrvStack::new(conf.msrv)", "\n    extract_msrv_attr!();")
    };
    let extract_msrv = if has_msrv {
        dst.extend(["\
use clippy::config::Conf;
use clippy_utils::msrvs::{self, ", msrv_ty, "};
"]);
        extract_msrv
    } else {
        ""
    };
    let pass_mac = pass.mac.name();
    let pass_name = pass.name;

    dst.extend(["use rustc_lint::{", cx_ty, ", ", pass_ty, ", ", pass_mac, "};\n\n"]);
    lint.gen_mac(dst);
    dst.push_str("\n\n");
    pass.gen_mac(dst);

    if has_msrv {
        dst.extend(["

pub struct", pass_name, "{
    msrv: ", msrv_ty, ",
}

impl ", pass_name, "{
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: ", msrv_ctor, "}
    }
}"]);
    }

    dst.extend(["

impl ", pass_ty, pass_lt, " for ", pass_name, "{
    // TODO: implement lint logic", extract_msrv, "
}
"]);
}

fn add_lint_pass(
    src: &str,
    dst: &mut String,
    name_snake: &str,
    name_pascal: &str,
    new_pass: LintPassKind,
    has_msrv: bool,
) {
    let mod_pos = find_mod_decl_after(&mut Cursor::new(src), name_snake);
    let (pre, src) = src.split_at(mod_pos.pos as usize);
    dst.push_str(pre);
    dst.extend(mod_pos.insertion_text(name_snake));

    let comment = match new_pass {
        LintPassKind::Early => "// add early passes here, used by `cargo dev new_lint`",
        LintPassKind::Late => "// add late passes here, used by `cargo dev new_lint`",
    };
    let ctor_call = if has_msrv { "::new(conf)" } else { "" };
    let pos = src.find(comment).unwrap_or_else(|| panic!("failed to find: {comment}"));
    let (start, end) = src.split_at(pos);
    #[rustfmt::skip]
    dst.extend([
        start,
        name_pascal, ": ", name_snake, "::", name_pascal, " = ",
        name_snake, "::", name_pascal, ctor_call, ",\n        ",
        end,
    ]);
}

struct ModPos {
    pos: u32,
    kind: PosKind,
}
enum PosKind {
    /// The position is the end of all leading extern crate declarations and inner attributes/docs.
    NewList,
    /// The position is the start of the name of the module to insert before.
    Name,
    /// The position is the end of the module list after the final semicolon.
    End,
}
impl ModPos {
    fn new_list(pos: u32) -> Self {
        Self {
            pos,
            kind: PosKind::NewList,
        }
    }

    fn insertion_text(self, mod_name: &str) -> [&str; 3] {
        match self.kind {
            PosKind::NewList if self.pos == 0 => ["pub mod ", mod_name, ";\n\n"],
            PosKind::NewList => ["\n\npub mod ", mod_name, ";"],
            PosKind::Name => [mod_name, ";\npub mod ", ""],
            PosKind::End => ["\npub mod ", mod_name, ";"],
        }
    }
}

/// Copies the source text to the destination adding a module declaration if `add_mod` is true.
fn mk_sorted_lints_copy_fn(mut add_mod: bool, mod_name: &str) -> impl FnMut(&str, &mut String) {
    move |src, dst| {
        if add_mod {
            add_mod = false;
            let pos = find_mod_decl_after(&mut Cursor::new(src), mod_name);
            let (pre, post) = src.split_at(pos.pos as usize);
            dst.push_str(pre);
            dst.extend(pos.insertion_text(mod_name));
            dst.push_str(post);
            return;
        }
        dst.push_str(src);
    }
}

/// Gets the position to insert a pub module with the specified name.
fn find_mod_decl_after(cursor: &mut Cursor<'_>, mod_name: &str) -> ModPos {
    let mut lead_end = 0;
    let mut take_next_line_comment = true;
    loop {
        match cursor.peek() {
            TokenKind::Whitespace if take_next_line_comment => {
                take_next_line_comment = !cursor.peek_text().contains('\n');
            },
            TokenKind::LineComment { doc_style: None } if take_next_line_comment => {
                take_next_line_comment = false;
                lead_end = cursor.pos() + cursor.peek_len();
            },
            TokenKind::LineComment { doc_style } | TokenKind::BlockComment { doc_style, .. }
                if matches!(doc_style, Some(DocStyle::Inner)) =>
            {
                take_next_line_comment = false;
                lead_end = cursor.pos() + cursor.peek_len();
            },
            TokenKind::Whitespace | TokenKind::LineComment { .. } | TokenKind::BlockComment { .. } => {},
            TokenKind::Pound => {
                cursor.step();
                let is_inner = cursor.eat_bang();
                if !cursor.eat_open_bracket() {
                    return ModPos::new_list(lead_end);
                }
                cursor.eat_remaining_tt();
                if is_inner {
                    take_next_line_comment = true;
                    lead_end = cursor.pos();
                } else {
                    take_next_line_comment = false;
                }
                continue;
            },
            TokenKind::Ident => {
                let ident = cursor.peek_text();
                cursor.step();
                match ident {
                    "extern" if cursor.eat_ident("crate") && cursor.capture_ident().is_some() && cursor.eat_semi() => {
                        take_next_line_comment = true;
                        lead_end = cursor.pos();
                    },
                    "pub" if cursor.eat_ident("mod") => break,
                    _ => return ModPos::new_list(lead_end),
                }
                continue;
            },
            _ => return ModPos::new_list(lead_end),
        }
        cursor.step();
    }

    while let Some(name) = cursor.capture_ident() {
        if !cursor.eat_semi() {
            return ModPos::new_list(lead_end);
        }
        if cursor.get_text(name) > mod_name {
            return ModPos {
                pos: name.pos,
                kind: PosKind::Name,
            };
        }
        let end = cursor.pos();
        if cursor.at_multi_line_break() || !cursor.eat_ident("pub") || !cursor.eat_ident("mod") {
            return ModPos {
                pos: end,
                kind: PosKind::End,
            };
        }
    }
    ModPos::new_list(lead_end)
}
