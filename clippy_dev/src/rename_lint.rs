use crate::parse::{Capture, Lint, LintKind, ParsedData, RenamedLint, RustSearcher, Token};
use crate::update_lints::generate_lint_files;
use crate::utils::{
    ErrAction, FileUpdater, UpdateMode, UpdateStatus, Version, delete_dir_if_exists, delete_file_if_exists,
    expect_action, try_rename_dir, try_rename_file, walk_dir_no_dot_or_target,
};
use core::mem;
use rustc_data_structures::fx::FxHashMap;
use rustc_lexer::TokenKind;
use std::collections::hash_map::Entry;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

/// Runs the `rename_lint` command.
///
/// This does the following:
/// * Adds an entry to `renamed_lints.rs`.
/// * Renames all lint attributes to the new name (e.g. `#[allow(clippy::lint_name)]`).
/// * Renames the lint struct to the new name.
/// * Renames the module containing the lint struct to the new name if it shares a name with the
///   lint.
///
/// # Panics
/// Panics for the following conditions:
/// * If a file path could not read from or then written to
/// * If either lint name has a prefix
/// * If `old_name` doesn't name an existing lint.
/// * If `old_name` names a deprecated or renamed lint.
#[expect(clippy::too_many_lines)]
pub fn rename(clippy_version: Version, old_name: &str, new_name: &str, uplift: bool) {
    if let Some((prefix, _)) = old_name.split_once("::") {
        panic!("`{old_name}` should not contain the `{prefix}` prefix");
    }
    if let Some((prefix, _)) = new_name.split_once("::") {
        panic!("`{new_name}` should not contain the `{prefix}` prefix");
    }

    let mut updater = FileUpdater::default();
    let mut data = ParsedData::collect();

    // Update any existing renames
    let new_name_prefixed = if uplift {
        new_name.to_owned()
    } else {
        String::from_iter(["clippy::", new_name])
    };
    for lint in data.lints.values_mut() {
        if let LintKind::Renamed(lint) = &mut lint.kind
            && lint.new_name.strip_prefix("clippy::") == Some(old_name)
        {
            lint.new_name.clone_from(&new_name_prefixed);
        }
    }

    // Mark the lint as renamed
    let Some(entry) = data.lints.get_mut(old_name) else {
        eprintln!("error: failed to find lint `{old_name}`");
        return;
    };
    let LintKind::Active(lint) = mem::replace(
        &mut entry.kind,
        LintKind::Renamed(RenamedLint {
            new_name: new_name_prefixed,
            version: clippy_version.rust_display().to_string(),
        }),
    ) else {
        eprintln!("error: lint `{old_name}` is already deprecated");
        return;
    };
    let lint_name_span = entry.name_span;

    let mut mod_edit = ModEdit::None;
    if uplift {
        let lint_file = &data.source_map.files[lint_name_span.file];
        let is_unique_mod = data
            .lints
            .values()
            .any(|x| data.source_map.files[x.name_span.file].module == lint_file.module);
        if is_unique_mod {
            if delete_file_if_exists(lint_file.path.as_ref()) {
                mod_edit = ModEdit::Delete;
            }
        } else {
            updater.update_file(&lint_file.path, &mut |_, src, dst| -> UpdateStatus {
                let mut start = &src[..lint.decl_span.start as usize];
                if start.ends_with("\n\n") {
                    start = &start[..start.len() - 1];
                }
                let mut end = &src[lint.decl_span.end as usize..];
                if end.starts_with("\n\n") {
                    end = &end[1..];
                }
                dst.push_str(start);
                dst.push_str(end);
                UpdateStatus::Changed
            });
        }
        delete_test_files(old_name, &data.lints);
    } else if let Entry::Vacant(entry) = data.lints.entry(new_name.to_owned()) {
        let lint_file = &mut data.source_map.files[lint.decl_span.file];
        if lint_file.module.ends_with(old_name)
            && lint_file
                .path
                .file_stem()
                .is_some_and(|x| x.as_encoded_bytes() == old_name.as_bytes())
        {
            let mut new_path = lint_file.path.with_file_name(new_name).into_os_string();
            new_path.push(".rs");
            if try_rename_file(lint_file.path.as_ref(), new_path.as_ref()) {
                lint_file.path = new_path.into();
                mod_edit = ModEdit::Rename;

                let mod_len = lint_file.module.len();
                lint_file.module.truncate(mod_len - old_name.len());
                lint_file.module.push_str(new_name);
            }
        }
        entry.insert(Lint {
            kind: LintKind::Active(lint),
            name_span: lint_name_span,
        });
        rename_test_files(old_name, new_name, &data.lints);
    } else {
        println!("Renamed `clippy::{old_name}` to `clippy::{new_name}`");
        println!("Since `{new_name}` already exists the existing code has not been changed");
        return;
    }

    let mut update_fn = file_update_fn(old_name, new_name, mod_edit);
    for e in walk_dir_no_dot_or_target(".") {
        let e = expect_action(e, ErrAction::Read, ".");
        if e.path().as_os_str().as_encoded_bytes().ends_with(b".rs") {
            updater.update_file(e.path(), &mut update_fn);
        }
    }
    generate_lint_files(UpdateMode::Change, &data);

    if uplift {
        println!("Uplifted `clippy::{old_name}` as `{new_name}`");
        if matches!(mod_edit, ModEdit::None) {
            println!("Only the rename has been registered, the code will need to be edited manually");
        } else {
            println!("All the lint's code has been deleted");
            println!("Make sure to inspect the results as some things may have been missed");
        }
    } else {
        println!("Renamed `clippy::{old_name}` to `clippy::{new_name}`");
        println!("All code referencing the old name has been updated");
        println!("Make sure to inspect the results as some things may have been missed");
    }
    println!("note: `cargo uibless` still needs to be run to update the test results");
}

#[derive(Clone, Copy)]
enum ModEdit {
    None,
    Delete,
    Rename,
}

fn is_lint_test(lint: &str, name: &str, lints: &FxHashMap<String, Lint>) -> bool {
    let is_num = |c: char| c.is_ascii_digit();
    let Some(suffix) = name.strip_prefix(lint) else {
        return false;
    };
    let suffix = suffix.trim_start_matches(is_num);
    let Some(suffix) = suffix.strip_prefix('_') else {
        return true;
    };

    // Some lint names are a prefix of other lint names. Make sure we don't mix test files
    // between the two lints.
    !(lints.contains_key(name.trim_end_matches(is_num))
        || suffix
            .bytes()
            .zip(suffix.len() - name.len()..)
            .any(|(c, i)| c == b'_' && lints.contains_key(name[..i].trim_end_matches(is_num))))
}

fn collect_ui_test_names(lint: &str, lints: &FxHashMap<String, Lint>, dst: &mut Vec<(String, bool)>) {
    for e in fs::read_dir("tests/ui").expect("error reading `tests/ui`") {
        let e = e.expect("error reading `tests/ui`");
        if let Ok(name) = e.file_name().into_string() {
            if e.file_type().is_ok_and(|ty| ty.is_dir()) {
                if is_lint_test(lint, &name, lints) {
                    dst.push((name, false));
                }
            } else if let Some((name_only, _)) = name.split_once('.')
                && is_lint_test(lint, name_only, lints)
            {
                dst.push((name, true));
            }
        }
    }
}

fn collect_ui_toml_test_names(lint: &str, lints: &FxHashMap<String, Lint>, dst: &mut Vec<(String, bool)>) {
    for e in fs::read_dir("tests/ui-toml").expect("error reading `tests/ui-toml`") {
        let e = e.expect("error reading `tests/ui-toml`");
        if e.file_type().is_ok_and(|ty| ty.is_dir())
            && let Ok(name) = e.file_name().into_string()
            && is_lint_test(lint, &name, lints)
        {
            dst.push((name, false));
        }
    }
}

/// Renames all test files for the given lint.
///
/// If `rename_prefixed` is `true` this will also rename tests which have the lint name as a prefix.
fn rename_test_files(old_name: &str, new_name: &str, lints: &FxHashMap<String, Lint>) {
    let mut tests = Vec::new();

    let mut old_buf = OsString::from("tests/ui/");
    let mut new_buf = OsString::from("tests/ui/");
    collect_ui_test_names(old_name, lints, &mut tests);
    for &(ref name, is_file) in &tests {
        old_buf.push(name);
        new_buf.push(new_name);
        new_buf.push(&name[old_name.len()..]);
        if is_file {
            try_rename_file(old_buf.as_ref(), new_buf.as_ref());
        } else {
            try_rename_dir(old_buf.as_ref(), new_buf.as_ref());
        }
        old_buf.truncate("tests/ui/".len());
        new_buf.truncate("tests/ui/".len());
    }

    tests.clear();
    old_buf.truncate("tests/ui".len());
    new_buf.truncate("tests/ui".len());
    old_buf.push("-toml/");
    new_buf.push("-toml/");
    collect_ui_toml_test_names(old_name, lints, &mut tests);
    for (name, _) in &tests {
        old_buf.push(name);
        new_buf.push(new_name);
        new_buf.push(&name[old_name.len()..]);
        try_rename_dir(old_buf.as_ref(), new_buf.as_ref());
        old_buf.truncate("tests/ui-toml/".len());
        new_buf.truncate("tests/ui-toml/".len());
    }
}

fn delete_test_files(lint: &str, lints: &FxHashMap<String, Lint>) {
    let mut tests = Vec::new();

    let mut buf = OsString::from("tests/ui/");
    collect_ui_test_names(lint, lints, &mut tests);
    for &(ref name, is_file) in &tests {
        buf.push(name);
        if is_file {
            delete_file_if_exists(buf.as_ref());
        } else {
            delete_dir_if_exists(buf.as_ref());
        }
        buf.truncate("tests/ui/".len());
    }

    buf.truncate("tests/ui".len());
    buf.push("-toml/");

    tests.clear();
    collect_ui_toml_test_names(lint, lints, &mut tests);
    for (name, _) in &tests {
        buf.push(name);
        delete_dir_if_exists(buf.as_ref());
        buf.truncate("tests/ui-toml/".len());
    }
}

fn snake_to_pascal(s: &str) -> String {
    let mut dst = Vec::with_capacity(s.len());
    let mut iter = s.bytes();
    || -> Option<()> {
        dst.push(iter.next()?.to_ascii_uppercase());
        while let Some(c) = iter.next() {
            if c == b'_' {
                dst.push(iter.next()?.to_ascii_uppercase());
            } else {
                dst.push(c);
            }
        }
        Some(())
    }();
    String::from_utf8(dst).unwrap()
}

#[expect(clippy::too_many_lines)]
fn file_update_fn<'a, 'b>(
    old_name: &'a str,
    new_name: &'b str,
    mod_edit: ModEdit,
) -> impl use<'a, 'b> + FnMut(&Path, &str, &mut String) -> UpdateStatus {
    let old_name_pascal = snake_to_pascal(old_name);
    let new_name_pascal = snake_to_pascal(new_name);
    let old_name_upper = old_name.to_ascii_uppercase();
    let new_name_upper = new_name.to_ascii_uppercase();
    move |_, src, dst| {
        let mut copy_pos = 0u32;
        let mut changed = false;
        let mut searcher = RustSearcher::new(src);
        let mut captures = [Capture::EMPTY];
        loop {
            match searcher.peek() {
                TokenKind::Eof => break,
                TokenKind::Ident => {
                    let match_start = searcher.pos();
                    let text = searcher.peek_text();
                    searcher.step();
                    match text {
                        // clippy::lint_name
                        "clippy" => {
                            if searcher.match_tokens(&[Token::DoubleColon, Token::CaptureIdent], &mut captures)
                                && searcher.get_capture(captures[0]) == old_name
                            {
                                dst.push_str(&src[copy_pos as usize..captures[0].pos as usize]);
                                dst.push_str(new_name);
                                copy_pos = searcher.pos();
                                changed = true;
                            }
                        },
                        // mod lint_name
                        "mod" => {
                            if !matches!(mod_edit, ModEdit::None)
                                && searcher.match_tokens(&[Token::CaptureIdent], &mut captures)
                                && searcher.get_capture(captures[0]) == old_name
                            {
                                match mod_edit {
                                    ModEdit::Rename => {
                                        dst.push_str(&src[copy_pos as usize..captures[0].pos as usize]);
                                        dst.push_str(new_name);
                                        copy_pos = searcher.pos();
                                        changed = true;
                                    },
                                    ModEdit::Delete if searcher.match_tokens(&[Token::Semi], &mut []) => {
                                        let mut start = &src[copy_pos as usize..match_start as usize];
                                        if start.ends_with("\n\n") {
                                            start = &start[..start.len() - 1];
                                        }
                                        dst.push_str(start);
                                        copy_pos = searcher.pos();
                                        if src[copy_pos as usize..].starts_with("\n\n") {
                                            copy_pos += 1;
                                        }
                                        changed = true;
                                    },
                                    ModEdit::Delete | ModEdit::None => {},
                                }
                            }
                        },
                        // lint_name::
                        name if matches!(mod_edit, ModEdit::Rename) && name == old_name => {
                            let name_end = searcher.pos();
                            if searcher.match_tokens(&[Token::DoubleColon], &mut []) {
                                dst.push_str(&src[copy_pos as usize..match_start as usize]);
                                dst.push_str(new_name);
                                copy_pos = name_end;
                                changed = true;
                            }
                        },
                        // LINT_NAME or LintName
                        name => {
                            let replacement = if name == old_name_upper {
                                &new_name_upper
                            } else if name == old_name_pascal {
                                &new_name_pascal
                            } else {
                                continue;
                            };
                            dst.push_str(&src[copy_pos as usize..match_start as usize]);
                            dst.push_str(replacement);
                            copy_pos = searcher.pos();
                            changed = true;
                        },
                    }
                },
                // //~ lint_name
                TokenKind::LineComment { doc_style: None } => {
                    let text = searcher.peek_text();
                    if text.starts_with("//~")
                        && let Some(text) = text.strip_suffix(old_name)
                        && !text.ends_with(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
                    {
                        dst.push_str(&src[copy_pos as usize..searcher.pos() as usize + text.len()]);
                        dst.push_str(new_name);
                        copy_pos = searcher.pos() + searcher.peek_len();
                        changed = true;
                    }
                    searcher.step();
                },
                // ::lint_name
                TokenKind::Colon
                    if searcher.match_tokens(&[Token::DoubleColon, Token::CaptureIdent], &mut captures)
                        && searcher.get_capture(captures[0]) == old_name =>
                {
                    dst.push_str(&src[copy_pos as usize..captures[0].pos as usize]);
                    dst.push_str(new_name);
                    copy_pos = searcher.pos();
                    changed = true;
                },
                _ => searcher.step(),
            }
        }

        dst.push_str(&src[copy_pos as usize..]);
        UpdateStatus::from_changed(changed)
    }
}
