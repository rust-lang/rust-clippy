use crate::generate::gen_sorted_lints_file;
use crate::ir::{ActiveLint, ActiveLintData, Lint, LintData, LintPass, LintPassKind, LintPassMac};
use crate::parse::cursor::Cursor;
use crate::utils::{FileUpdater, VecBuf, Version, create_new_dir};
use crate::{SourceFile, Span, UpdateMode, new_parse_cx};
use std::collections::hash_map::Entry;
use std::fmt::Write as _;
use std::path::{self, MAIN_SEPARATOR_STR as PATH_SEP, PathBuf};

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
                let passes = data.lint_passes.all_in_same_file_as_mut(pass_idx);
                let mut ranges = VecBuf::with_capacity(lints.len() + passes.len());
                let mut copy = mk_sorted_lints_copy_fn(add_mod, name_snake);
                gen_sorted_lints_file(src, dst, &mut lints, passes, &mut ranges, &mut copy);
                if add_mod {
                    // No existing module list was found; just put it at the start.
                    dst.insert_str(0, &format!("mod {name_snake};\n\n"));
                }
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

fn write_lint_check_file(dst: &mut String, name_upper: &str, is_late_pass: bool, has_msrv: bool) {
    let (cx_ty, cx_lt, msrv_arg, msrv_import) = if is_late_pass {
        (
            "LateContext",
            "<'_>",
            ", msrv: Msrv",
            "use clippy_utils::msrvs::{self, Msrv};\n",
        )
    } else {
        (
            "EarlyContext",
            "",
            ", msrv: &MsrvStack",
            "use clippy_utils::msrvs::{self, MsrvStack};\n",
        )
    };
    let (msrv_arg, msrv_import) = if has_msrv { (msrv_arg, msrv_import) } else { ("", "") };
    let _ = write!(
        dst,
        "\
{msrv_import}use rustc_lint::{cx_ty};

use super::{name_upper};

pub(super) fn check(cx: &{cx_ty}{cx_lt}{msrv_arg}) {{
    todo!(\"implement lint logic\");
}}\n"
    );
}

fn write_test_file(dst: &mut String, name: &str, has_msrv: bool) {
    let msrv_contents = if has_msrv {
        "\n
    // TODO: set `xx` to on below the required MSRV and `yy` to the required MSRV.
    #[clippy::msrv = \"1.xx\"]
    {
        // TODO: test which requires the msrv to be set
    };
    #[clippy::msrv =\"1.yy\"]
    {
        // TODO: same test as above
    }\n"
    } else {
        ""
    };
    let _ = write!(
        dst,
        "\
#![warn(clippy::{name})]

fn main() {{
    // TODO: fill in tests{msrv_contents}
}}\n",
    );
}

fn write_cargo_manifest(dst: &mut String, name: &str, todo: &str) {
    let _ = write!(
        dst,
        "\
[package]
name = \"{name}\"
version = \"0.1.0\"
publish = false

[workspace]

# TODO: {todo}\n",
    );
}

fn write_lint_file(dst: &mut String, lint: &ActiveLint<'_, '_>, pass: &LintPass<'_>, has_msrv: bool) {
    let (pass_ty, pass_lt, cx_ty, msrv_ty, msrv_ctor, extract_msrv) = if pass.is_late {
        ("LateLintPass", "<'_>", "LateContext", "Msrv", "conf.msrv", "")
    } else {
        (
            "EarlyLintPass",
            "",
            "EarlyContext",
            "MsrvStack",
            "MsrvStack::new(conf.msrv)",
            "\n    extract_msrv_attr!();",
        )
    };
    let extract_msrv = if has_msrv {
        let _ = write!(
            dst,
            "use clippy_config::Conf;\n\
            use clippy_utils::msrvs::{{self, {msrv_ty}}};\n",
        );
        extract_msrv
    } else {
        ""
    };
    let pass_mac = pass.mac.name();
    let pass_name = pass.name;

    let _ = write!(
        dst,
        "use rustc_lint::{{{cx_ty}, {pass_ty}}};\n\
        use rustc_session::{pass_mac};\n\n",
    );
    lint.gen_mac(dst);
    dst.push_str("\n\n");
    pass.gen_mac(dst);
    dst.push_str("\n\n");
    if has_msrv {
        let _ = write!(
            dst,
            "\
pub struct {pass_name} {{
    msrv: {msrv_ty},
}}

impl {pass_name} {{
    pub fn new(conf: &'static Conf) -> Self {{
        Self {{ msrv: {msrv_ctor} }}
    }}
}}\n\n"
        );
    }
    let _ = writeln!(
        dst,
        "\
impl {pass_ty}{pass_lt} for {pass_name} {{
    // TODO: implement lint logic{extract_msrv}
}}\n",
    );
}

fn add_lint_pass(
    src: &str,
    dst: &mut String,
    name_snake: &str,
    name_pascal: &str,
    new_pass: LintPassKind,
    has_msrv: bool,
) {
    let (comment, closure_start) = match new_pass {
        LintPassKind::Early => ("// add early passes here, used by `cargo dev new_lint`", "||"),
        LintPassKind::Late => ("// add late passes here, used by `cargo dev new_lint`", "|_|"),
    };
    let (closure_kind, ctor_call) = if has_msrv { ("move ", "::new(conf)") } else { ("", "") };
    let pos = if let Some(pos) = src.find(comment) {
        dst.push_str(&src[..pos]);
        let _ = write!(
            dst,
            "Box::new({closure_kind}{closure_start} Box::new({name_snake}::{name_pascal}{ctor_call}))\
            \n    ",
        );
        pos
    } else {
        0
    };
    dst.push_str(&src[pos..]);
}

enum ModPos {
    /// The position of the name of the module to insert before.
    Name(u32),
    /// The position of the end of the module list after the final semicolon.
    End(u32),
}

/// Copies the source text to the destination adding a module declaration if `add_mod` is true.
fn mk_sorted_lints_copy_fn(mut add_mod: bool, mod_name: &str) -> impl FnMut(&str, &mut String) {
    move |src, dst| {
        if add_mod && let Some(pos) = find_mod_decl_after(&mut Cursor::new(src), mod_name) {
            match pos {
                ModPos::Name(pos) => {
                    let (pre, post) = src.split_at(pos as usize);
                    dst.extend([pre, mod_name, ";\npub mod ", post]);
                },
                ModPos::End(pos) => {
                    let (pre, post) = src.split_at(pos as usize);
                    dst.extend([pre, "pub mod ", mod_name, ";\n", post]);
                },
            }
            add_mod = false;
        } else {
            dst.push_str(src);
        }
    }
}

/// Gets the position to insert a pub module with the specified name. Returns
/// `None` if a module list could not be found.
fn find_mod_decl_after(cursor: &mut Cursor<'_>, mod_name: &str) -> Option<ModPos> {
    if !(cursor.find_ident("pub") && cursor.eat_ident("mod")) {
        return None;
    }
    let mut end = None;
    while let Some(name) = cursor.capture_ident() {
        if !cursor.eat_semi() {
            break;
        }
        if cursor.get_text(name) > mod_name {
            return Some(ModPos::Name(name.pos));
        }
        end = Some(cursor.pos());
        if !(cursor.eat_ident("pub") && cursor.eat_ident("mod")) {
            break;
        }
    }
    end.map(ModPos::End)
}
