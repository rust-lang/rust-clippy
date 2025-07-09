use crate::parse::{ActiveLint, DeprecatedLint, Lint, ParsedData};
use crate::update_lints::generate_lint_files;
use crate::utils::{FileUpdater, UpdateMode, UpdateStatus, Version, delete_dir_if_exists, delete_file_if_exists};
use core::mem;
use std::ffi::OsStr;
use std::path::Path;

/// Runs the `deprecate` command
///
/// This does the following:
/// * Adds an entry to `deprecated_lints.rs`.
/// * Removes the lint declaration (and the entire file if applicable)
///
/// # Panics
///
/// If a file path could not read from or written to
pub fn deprecate(clippy_version: Version, name: &str, reason: &str) {
    if let Some((prefix, _)) = name.split_once("::") {
        panic!("`{name}` should not contain the `{prefix}` prefix");
    }

    let mut data = ParsedData::collect();
    let Some(entry) = data.lints.get_mut(name) else {
        eprintln!("error: failed to find lint `{name}`");
        return;
    };
    let Lint::Active(lint) = mem::replace(
        entry,
        Lint::Deprecated(DeprecatedLint {
            reason: reason.into(),
            version: clippy_version.rust_display().to_string(),
        }),
    ) else {
        eprintln!("error: lint `{name}` is already deprecated");
        return;
    };

    remove_lint_declaration(name, &lint, &data);
    generate_lint_files(UpdateMode::Change, &data);
    println!("info: `{name}` has successfully been deprecated");
    println!("note: you must run `cargo uitest` to update the test results");
}

fn remove_lint_declaration(name: &str, lint: &ActiveLint, data: &ParsedData) {
    fn remove_test_assets(name: &str) {
        let test_file_stem = format!("tests/ui/{name}");
        let path = Path::new(&test_file_stem);

        // Some lints have their own directories, delete them
        if path.is_dir() {
            delete_dir_if_exists(path);
        } else {
            // Remove all related test files
            delete_file_if_exists(&path.with_extension("rs"));
            delete_file_if_exists(&path.with_extension("stderr"));
            delete_file_if_exists(&path.with_extension("fixed"));
        }
    }

    fn remove_impl_lint_pass(lint_name_upper: &str, content: &mut String) {
        let impl_lint_pass_start = content.find("impl_lint_pass!").unwrap_or_else(|| {
            content
                .find("declare_lint_pass!")
                .unwrap_or_else(|| panic!("failed to find `impl_lint_pass`"))
        });
        let mut impl_lint_pass_end = content[impl_lint_pass_start..]
            .find(']')
            .expect("failed to find `impl_lint_pass` terminator");

        impl_lint_pass_end += impl_lint_pass_start;
        if let Some(lint_name_pos) = content[impl_lint_pass_start..impl_lint_pass_end].find(lint_name_upper) {
            let mut lint_name_end = impl_lint_pass_start + (lint_name_pos + lint_name_upper.len());
            for c in content[lint_name_end..impl_lint_pass_end].chars() {
                // Remove trailing whitespace
                if c == ',' || c.is_whitespace() {
                    lint_name_end += 1;
                } else {
                    break;
                }
            }

            content.replace_range(impl_lint_pass_start + lint_name_pos..lint_name_end, "");
        }
    }

    let lint_file = &data.source_map.files[lint.span.file];
    if data.lints.values().any(|l| {
        if let Lint::Active(l) = l {
            let other_file = &data.source_map.files[l.span.file];
            other_file.krate == lint_file.krate && other_file.module.starts_with(&lint_file.module)
        } else {
            false
        }
    }) {
        // Try to delete a sub-module that matches the lint's name
        let removed_mod = if lint_file.path.file_name().map(OsStr::as_encoded_bytes) == Some(b"mod.rs") {
            let mut path = lint_file.path.clone();
            path.set_file_name(name);
            path.set_extension("rs");
            delete_file_if_exists(&path)
        } else {
            false
        };

        FileUpdater::default().update_file(&lint_file.path, &mut |_, src, dst| {
            let (a, b, c, d) = if removed_mod
                && let mod_decl = format!("\nmod {name};")
                && let Some(mod_start) = src.find(&mod_decl)
            {
                if mod_start < lint.span.start as usize {
                    (
                        mod_start,
                        mod_start + mod_decl.len(),
                        lint.span.start as usize,
                        lint.span.end as usize,
                    )
                } else {
                    (
                        lint.span.start as usize,
                        lint.span.end as usize,
                        mod_start,
                        mod_start + mod_decl.len(),
                    )
                }
            } else {
                (lint.span.start as usize, lint.span.end as usize, src.len(), src.len())
            };
            dst.push_str(&src[..a]);
            dst.push_str(&src[b..c]);
            dst.push_str(&src[d..]);
            remove_impl_lint_pass(&name.to_uppercase(), dst);
            UpdateStatus::Changed
        });
    } else {
        // No other lint in the same module or a sub-module.
        delete_file_if_exists(&lint_file.path);
    }
    remove_test_assets(name);
}
