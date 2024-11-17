use crate::utils::{clippy_project_root, exit_with_failure, replace_region_in_file, UpdateMode};
use aho_corasick::AhoCorasickBuilder;
use itertools::Itertools;
use rustc_lexer::{LiteralKind, TokenKind, tokenize, unescape};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fmt::{self, Write};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, Write as _};
use std::ops::Range;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const GENERATED_FILE_COMMENT: &str = "// This file was generated by `cargo dev update_lints`.\n\
     // Use that command to update this file and do not edit by hand.\n\
     // Manual edits will be overwritten.\n\n";

const DOCS_LINK: &str = "https://rust-lang.github.io/rust-clippy/master/index.html";

/// Runs the `update_lints` command.
///
/// This updates various generated values from the lint source code.
///
/// `update_mode` indicates if the files should be updated or if updates should be checked for.
///
/// # Panics
///
/// Panics if a file path could not read from or then written to
pub fn update(update_mode: UpdateMode) {
    let (lints, deprecated_lints, renamed_lints) = gather_all();
    generate_lint_files(update_mode, &lints, &deprecated_lints, &renamed_lints);
}

fn generate_lint_files(
    update_mode: UpdateMode,
    lints: &[Lint],
    deprecated_lints: &[DeprecatedLint],
    renamed_lints: &[RenamedLint],
) {
    let internal_lints = Lint::internal_lints(lints);
    let mut usable_lints = Lint::usable_lints(lints);
    usable_lints.sort_by_key(|lint| lint.name.clone());

    replace_region_in_file(
        update_mode,
        Path::new("README.md"),
        "[There are over ",
        " lints included in this crate!]",
        |res| {
            write!(res, "{}", round_to_fifty(usable_lints.len())).unwrap();
        },
    );

    replace_region_in_file(
        update_mode,
        Path::new("book/src/README.md"),
        "[There are over ",
        " lints included in this crate!]",
        |res| {
            write!(res, "{}", round_to_fifty(usable_lints.len())).unwrap();
        },
    );

    replace_region_in_file(
        update_mode,
        Path::new("CHANGELOG.md"),
        "<!-- begin autogenerated links to lint list -->\n",
        "<!-- end autogenerated links to lint list -->",
        |res| {
            for lint in usable_lints
                .iter()
                .map(|l| &*l.name)
                .chain(deprecated_lints.iter().filter_map(|l| l.name.strip_prefix("clippy::")))
                .chain(renamed_lints.iter().filter_map(|l| l.old_name.strip_prefix("clippy::")))
                .sorted()
            {
                writeln!(res, "[`{lint}`]: {DOCS_LINK}#{lint}").unwrap();
            }
        },
    );

    // This has to be in lib.rs, otherwise rustfmt doesn't work
    replace_region_in_file(
        update_mode,
        Path::new("clippy_lints/src/lib.rs"),
        "// begin lints modules, do not remove this comment, it’s used in `update_lints`\n",
        "// end lints modules, do not remove this comment, it’s used in `update_lints`",
        |res| {
            for lint_mod in usable_lints.iter().map(|l| &l.module).unique().sorted() {
                writeln!(res, "mod {lint_mod};").unwrap();
            }
        },
    );

    process_file(
        "clippy_lints/src/declared_lints.rs",
        update_mode,
        &gen_declared_lints(internal_lints.iter(), usable_lints.iter()),
    );

    let content = gen_deprecated_lints_test(deprecated_lints);
    process_file("tests/ui/deprecated.rs", update_mode, &content);

    let content = gen_renamed_lints_test(renamed_lints);
    process_file("tests/ui/rename.rs", update_mode, &content);
}

pub fn print_lints() {
    let (lint_list, _, _) = gather_all();
    let usable_lints = Lint::usable_lints(&lint_list);
    let usable_lint_count = usable_lints.len();
    let grouped_by_lint_group = Lint::by_lint_group(usable_lints.into_iter());

    for (lint_group, mut lints) in grouped_by_lint_group {
        println!("\n## {lint_group}");

        lints.sort_by_key(|l| l.name.clone());

        for lint in lints {
            println!("* [{}]({DOCS_LINK}#{}) ({})", lint.name, lint.name, lint.desc);
        }
    }

    println!("there are {usable_lint_count} lints");
}

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
#[allow(clippy::too_many_lines)]
pub fn rename(old_name: &str, new_name: &str, uplift: bool) {
    if let Some((prefix, _)) = old_name.split_once("::") {
        panic!("`{old_name}` should not contain the `{prefix}` prefix");
    }
    if let Some((prefix, _)) = new_name.split_once("::") {
        panic!("`{new_name}` should not contain the `{prefix}` prefix");
    }

    let (mut lints, deprecated_lints, mut renamed_lints) = gather_all();
    let mut old_lint_index = None;
    let mut found_new_name = false;
    for (i, lint) in lints.iter().enumerate() {
        if lint.name == old_name {
            old_lint_index = Some(i);
        } else if lint.name == new_name {
            found_new_name = true;
        }
    }
    let old_lint_index = old_lint_index.unwrap_or_else(|| panic!("could not find lint `{old_name}`"));

    let lint = RenamedLint {
        old_name: format!("clippy::{old_name}"),
        new_name: if uplift {
            new_name.into()
        } else {
            format!("clippy::{new_name}")
        },
    };

    // Renamed lints and deprecated lints shouldn't have been found in the lint list, but check just in
    // case.
    assert!(
        !renamed_lints.iter().any(|l| lint.old_name == l.old_name),
        "`{old_name}` has already been renamed"
    );
    assert!(
        !deprecated_lints.iter().any(|l| lint.old_name == l.name),
        "`{old_name}` has already been deprecated"
    );

    // Update all lint level attributes. (`clippy::lint_name`)
    for file in WalkDir::new(clippy_project_root())
        .into_iter()
        .map(Result::unwrap)
        .filter(|f| {
            let name = f.path().file_name();
            let ext = f.path().extension();
            (ext == Some(OsStr::new("rs")) || ext == Some(OsStr::new("fixed")))
                && name != Some(OsStr::new("rename.rs"))
                && name != Some(OsStr::new("deprecated_lints.rs"))
        })
    {
        rewrite_file(file.path(), |s| {
            replace_ident_like(s, &[(&lint.old_name, &lint.new_name)])
        });
    }

    let version = crate::new_lint::get_stabilization_version();
    rewrite_file(Path::new("clippy_lints/src/deprecated_lints.rs"), |s| {
        insert_at_marker(
            s,
            "// end renamed lints. used by `cargo dev rename_lint`",
            &format!(
                "#[clippy::version = \"{version}\"]\n    \
                (\"{}\", \"{}\"),\n    ",
                lint.old_name, lint.new_name,
            ),
        )
    });

    renamed_lints.push(lint);
    renamed_lints.sort_by(|lhs, rhs| {
        lhs.new_name
            .starts_with("clippy::")
            .cmp(&rhs.new_name.starts_with("clippy::"))
            .reverse()
            .then_with(|| lhs.old_name.cmp(&rhs.old_name))
    });

    if uplift {
        write_file(Path::new("tests/ui/rename.rs"), &gen_renamed_lints_test(&renamed_lints));
        println!(
            "`{old_name}` has be uplifted. All the code inside `clippy_lints` related to it needs to be removed manually."
        );
    } else if found_new_name {
        write_file(Path::new("tests/ui/rename.rs"), &gen_renamed_lints_test(&renamed_lints));
        println!(
            "`{new_name}` is already defined. The old linting code inside `clippy_lints` needs to be updated/removed manually."
        );
    } else {
        // Rename the lint struct and source files sharing a name with the lint.
        let lint = &mut lints[old_lint_index];
        let old_name_upper = old_name.to_uppercase();
        let new_name_upper = new_name.to_uppercase();
        lint.name = new_name.into();

        // Rename test files. only rename `.stderr` and `.fixed` files if the new test name doesn't exist.
        if try_rename_file(
            Path::new(&format!("tests/ui/{old_name}.rs")),
            Path::new(&format!("tests/ui/{new_name}.rs")),
        ) {
            try_rename_file(
                Path::new(&format!("tests/ui/{old_name}.stderr")),
                Path::new(&format!("tests/ui/{new_name}.stderr")),
            );
            try_rename_file(
                Path::new(&format!("tests/ui/{old_name}.fixed")),
                Path::new(&format!("tests/ui/{new_name}.fixed")),
            );
        }

        // Try to rename the file containing the lint if the file name matches the lint's name.
        let replacements;
        let replacements = if lint.module == old_name
            && try_rename_file(
                Path::new(&format!("clippy_lints/src/{old_name}.rs")),
                Path::new(&format!("clippy_lints/src/{new_name}.rs")),
            ) {
            // Edit the module name in the lint list. Note there could be multiple lints.
            for lint in lints.iter_mut().filter(|l| l.module == old_name) {
                lint.module = new_name.into();
            }
            replacements = [(&*old_name_upper, &*new_name_upper), (old_name, new_name)];
            replacements.as_slice()
        } else if !lint.module.contains("::")
            // Catch cases like `methods/lint_name.rs` where the lint is stored in `methods/mod.rs`
            && try_rename_file(
                Path::new(&format!("clippy_lints/src/{}/{old_name}.rs", lint.module)),
                Path::new(&format!("clippy_lints/src/{}/{new_name}.rs", lint.module)),
            )
        {
            // Edit the module name in the lint list. Note there could be multiple lints, or none.
            let renamed_mod = format!("{}::{old_name}", lint.module);
            for lint in lints.iter_mut().filter(|l| l.module == renamed_mod) {
                lint.module = format!("{}::{new_name}", lint.module);
            }
            replacements = [(&*old_name_upper, &*new_name_upper), (old_name, new_name)];
            replacements.as_slice()
        } else {
            replacements = [(&*old_name_upper, &*new_name_upper), ("", "")];
            &replacements[0..1]
        };

        // Don't change `clippy_utils/src/renamed_lints.rs` here as it would try to edit the lint being
        // renamed.
        for (_, file) in clippy_lints_src_files().filter(|(rel_path, _)| rel_path != OsStr::new("deprecated_lints.rs"))
        {
            rewrite_file(file.path(), |s| replace_ident_like(s, replacements));
        }

        generate_lint_files(UpdateMode::Change, &lints, &deprecated_lints, &renamed_lints);
        println!("{old_name} has been successfully renamed");
    }

    println!("note: `cargo uitest` still needs to be run to update the test results");
}

/// Runs the `deprecate` command
///
/// This does the following:
/// * Adds an entry to `deprecated_lints.rs`.
/// * Removes the lint declaration (and the entire file if applicable)
///
/// # Panics
///
/// If a file path could not read from or written to
pub fn deprecate(name: &str, reason: &str) {
    let prefixed_name = if name.starts_with("clippy::") {
        name.to_owned()
    } else {
        format!("clippy::{name}")
    };
    let stripped_name = &prefixed_name[8..];

    let (mut lints, mut deprecated_lints, renamed_lints) = gather_all();
    let Some(lint) = lints.iter().find(|l| l.name == stripped_name) else {
        eprintln!("error: failed to find lint `{name}`");
        return;
    };

    let mod_path = {
        let mut mod_path = PathBuf::from(format!("clippy_lints/src/{}", lint.module));
        if mod_path.is_dir() {
            mod_path = mod_path.join("mod");
        }

        mod_path.set_extension("rs");
        mod_path
    };

    let deprecated_lints_path = &*clippy_project_root().join("clippy_lints/src/deprecated_lints.rs");

    if remove_lint_declaration(stripped_name, &mod_path, &mut lints).unwrap_or(false) {
        let version = crate::new_lint::get_stabilization_version();
        rewrite_file(deprecated_lints_path, |s| {
            insert_at_marker(
                s,
                "// end deprecated lints. used by `cargo dev deprecate_lint`",
                &format!("#[clippy::version = \"{version}\"]\n    (\"{prefixed_name}\", \"{reason}\"),\n    ",),
            )
        });

        deprecated_lints.push(DeprecatedLint {
            name: prefixed_name,
            reason: reason.into(),
        });

        generate_lint_files(UpdateMode::Change, &lints, &deprecated_lints, &renamed_lints);
        println!("info: `{name}` has successfully been deprecated");
        println!("note: you must run `cargo uitest` to update the test results");
    } else {
        eprintln!("error: lint not found");
    }
}

fn remove_lint_declaration(name: &str, path: &Path, lints: &mut Vec<Lint>) -> io::Result<bool> {
    fn remove_lint(name: &str, lints: &mut Vec<Lint>) {
        lints.iter().position(|l| l.name == name).map(|pos| lints.remove(pos));
    }

    fn remove_test_assets(name: &str) {
        let test_file_stem = format!("tests/ui/{name}");
        let path = Path::new(&test_file_stem);

        // Some lints have their own directories, delete them
        if path.is_dir() {
            let _ = fs::remove_dir_all(path);
            return;
        }

        // Remove all related test files
        let _ = fs::remove_file(path.with_extension("rs"));
        let _ = fs::remove_file(path.with_extension("stderr"));
        let _ = fs::remove_file(path.with_extension("fixed"));
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

    if path.exists() {
        if let Some(lint) = lints.iter().find(|l| l.name == name) {
            if lint.module == name {
                // The lint name is the same as the file, we can just delete the entire file
                fs::remove_file(path)?;
            } else {
                // We can't delete the entire file, just remove the declaration

                if let Some(Some("mod.rs")) = path.file_name().map(OsStr::to_str) {
                    // Remove clippy_lints/src/some_mod/some_lint.rs
                    let mut lint_mod_path = path.to_path_buf();
                    lint_mod_path.set_file_name(name);
                    lint_mod_path.set_extension("rs");

                    let _ = fs::remove_file(lint_mod_path);
                }

                let mut content =
                    fs::read_to_string(path).unwrap_or_else(|_| panic!("failed to read `{}`", path.to_string_lossy()));

                eprintln!(
                    "warn: you will have to manually remove any code related to `{name}` from `{}`",
                    path.display()
                );

                assert!(
                    content[lint.declaration_range.clone()].contains(&name.to_uppercase()),
                    "error: `{}` does not contain lint `{}`'s declaration",
                    path.display(),
                    lint.name
                );

                // Remove lint declaration (declare_clippy_lint!)
                content.replace_range(lint.declaration_range.clone(), "");

                // Remove the module declaration (mod xyz;)
                let mod_decl = format!("\nmod {name};");
                content = content.replacen(&mod_decl, "", 1);

                remove_impl_lint_pass(&lint.name.to_uppercase(), &mut content);
                fs::write(path, content).unwrap_or_else(|_| panic!("failed to write to `{}`", path.to_string_lossy()));
            }

            remove_test_assets(name);
            remove_lint(name, lints);
            return Ok(true);
        }
    }

    Ok(false)
}

/// Replace substrings if they aren't bordered by identifier characters. Returns `None` if there
/// were no replacements.
fn replace_ident_like(contents: &str, replacements: &[(&str, &str)]) -> Option<String> {
    fn is_ident_char(c: u8) -> bool {
        matches!(c, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
    }

    let searcher = AhoCorasickBuilder::new()
        .match_kind(aho_corasick::MatchKind::LeftmostLongest)
        .build(replacements.iter().map(|&(x, _)| x.as_bytes()))
        .unwrap();

    let mut result = String::with_capacity(contents.len() + 1024);
    let mut pos = 0;
    let mut edited = false;
    for m in searcher.find_iter(contents) {
        let (old, new) = replacements[m.pattern()];
        result.push_str(&contents[pos..m.start()]);
        result.push_str(
            if !is_ident_char(contents.as_bytes().get(m.start().wrapping_sub(1)).copied().unwrap_or(0))
                && !is_ident_char(contents.as_bytes().get(m.end()).copied().unwrap_or(0))
            {
                edited = true;
                new
            } else {
                old
            },
        );
        pos = m.end();
    }
    result.push_str(&contents[pos..]);
    edited.then_some(result)
}

fn round_to_fifty(count: usize) -> usize {
    count / 50 * 50
}

fn process_file(path: impl AsRef<Path>, update_mode: UpdateMode, content: &str) {
    if update_mode == UpdateMode::Check {
        let old_content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("Cannot read from {}: {e}", path.as_ref().display()));
        if content != old_content {
            exit_with_failure();
        }
    } else {
        fs::write(&path, content.as_bytes())
            .unwrap_or_else(|e| panic!("Cannot write to {}: {e}", path.as_ref().display()));
    }
}

/// Lint data parsed from the Clippy source code.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Lint {
    name: String,
    group: String,
    desc: String,
    module: String,
    declaration_range: Range<usize>,
}

impl Lint {
    #[must_use]
    fn new(name: &str, group: &str, desc: &str, module: &str, declaration_range: Range<usize>) -> Self {
        Self {
            name: name.to_lowercase(),
            group: group.into(),
            desc: remove_line_splices(desc),
            module: module.into(),
            declaration_range,
        }
    }

    /// Returns all non-deprecated lints and non-internal lints
    #[must_use]
    fn usable_lints(lints: &[Self]) -> Vec<Self> {
        lints
            .iter()
            .filter(|l| !l.group.starts_with("internal"))
            .cloned()
            .collect()
    }

    /// Returns all internal lints
    #[must_use]
    fn internal_lints(lints: &[Self]) -> Vec<Self> {
        lints.iter().filter(|l| l.group == "internal").cloned().collect()
    }

    /// Returns the lints in a `HashMap`, grouped by the different lint groups
    #[must_use]
    fn by_lint_group(lints: impl Iterator<Item = Self>) -> HashMap<String, Vec<Self>> {
        lints.map(|lint| (lint.group.to_string(), lint)).into_group_map()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct DeprecatedLint {
    name: String,
    reason: String,
}
impl DeprecatedLint {
    fn new(name: &str, reason: &str) -> Self {
        Self {
            name: remove_line_splices(name),
            reason: remove_line_splices(reason),
        }
    }
}

struct RenamedLint {
    old_name: String,
    new_name: String,
}
impl RenamedLint {
    fn new(old_name: &str, new_name: &str) -> Self {
        Self {
            old_name: remove_line_splices(old_name),
            new_name: remove_line_splices(new_name),
        }
    }
}

/// Generates the code for registering lints
#[must_use]
fn gen_declared_lints<'a>(
    internal_lints: impl Iterator<Item = &'a Lint>,
    usable_lints: impl Iterator<Item = &'a Lint>,
) -> String {
    let mut details: Vec<_> = internal_lints
        .map(|l| (false, &l.module, l.name.to_uppercase()))
        .chain(usable_lints.map(|l| (true, &l.module, l.name.to_uppercase())))
        .collect();
    details.sort_unstable();

    let mut output = GENERATED_FILE_COMMENT.to_string();
    output.push_str("pub static LINTS: &[&crate::LintInfo] = &[\n");

    for (is_public, module_name, lint_name) in details {
        if !is_public {
            output.push_str("    #[cfg(feature = \"internal\")]\n");
        }
        let _: fmt::Result = writeln!(output, "    crate::{module_name}::{lint_name}_INFO,");
    }
    output.push_str("];\n");

    output
}

fn gen_deprecated_lints_test(lints: &[DeprecatedLint]) -> String {
    let mut res: String = GENERATED_FILE_COMMENT.into();
    for lint in lints {
        writeln!(res, "#![warn({})] //~ ERROR: lint `{}`", lint.name, lint.name).unwrap();
    }
    res.push_str("\nfn main() {}\n");
    res
}

fn gen_renamed_lints_test(lints: &[RenamedLint]) -> String {
    let mut seen_lints = HashSet::new();
    let mut res: String = GENERATED_FILE_COMMENT.into();

    res.push_str("#![allow(clippy::duplicated_attributes)]\n");
    for lint in lints {
        if seen_lints.insert(&lint.new_name) {
            writeln!(res, "#![allow({})]", lint.new_name).unwrap();
        }
    }
    seen_lints.clear();
    for lint in lints {
        if seen_lints.insert(&lint.old_name) {
            writeln!(res, "#![warn({})] //~ ERROR: lint `{}`", lint.old_name, lint.old_name).unwrap();
        }
    }
    res.push_str("\nfn main() {}\n");
    res
}

/// Gathers all lints defined in `clippy_lints/src`
fn gather_all() -> (Vec<Lint>, Vec<DeprecatedLint>, Vec<RenamedLint>) {
    let mut lints = Vec::with_capacity(1000);
    let mut deprecated_lints = Vec::with_capacity(50);
    let mut renamed_lints = Vec::with_capacity(50);

    for (rel_path, file) in clippy_lints_src_files() {
        let path = file.path();
        let contents =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("Cannot read from `{}`: {e}", path.display()));
        let module = rel_path
            .components()
            .map(|c| c.as_os_str().to_str().unwrap())
            .collect::<Vec<_>>()
            .join("::");

        // If the lints are stored in mod.rs, we get the module name from
        // the containing directory:
        let module = if let Some(module) = module.strip_suffix("::mod.rs") {
            module
        } else {
            module.strip_suffix(".rs").unwrap_or(&module)
        };

        if module == "deprecated_lints" {
            parse_deprecated_contents(&contents, &mut deprecated_lints, &mut renamed_lints);
        } else {
            parse_contents(&contents, module, &mut lints);
        }
    }
    (lints, deprecated_lints, renamed_lints)
}

fn clippy_lints_src_files() -> impl Iterator<Item = (PathBuf, DirEntry)> {
    let root_path = clippy_project_root().join("clippy_lints/src");
    let iter = WalkDir::new(&root_path).into_iter();
    iter.map(Result::unwrap)
        .filter(|f| f.path().extension() == Some(OsStr::new("rs")))
        .map(move |f| (f.path().strip_prefix(&root_path).unwrap().to_path_buf(), f))
}

macro_rules! match_tokens {
    ($iter:ident, $($token:ident $({$($fields:tt)*})? $(($capture:ident))?)*) => {
         {
            $(#[allow(clippy::redundant_pattern)] let Some(LintDeclSearchResult {
                    token_kind: TokenKind::$token $({$($fields)*})?,
                    content: $($capture @)? _,
                    ..
            }) = $iter.next() else {
                continue;
            };)*
            #[allow(clippy::unused_unit)]
            { ($($($capture,)?)*) }
        }
    }
}

pub(crate) use match_tokens;

pub(crate) struct LintDeclSearchResult<'a> {
    pub token_kind: TokenKind,
    pub content: &'a str,
    pub range: Range<usize>,
}

/// Parse a source file looking for `declare_clippy_lint` macro invocations.
fn parse_contents(contents: &str, module: &str, lints: &mut Vec<Lint>) {
    let mut offset = 0usize;
    let mut iter = tokenize(contents).map(|t| {
        let range = offset..offset + t.len as usize;
        offset = range.end;

        LintDeclSearchResult {
            token_kind: t.kind,
            content: &contents[range.clone()],
            range,
        }
    });

    while let Some(LintDeclSearchResult { range, .. }) = iter.find(
        |LintDeclSearchResult {
             token_kind, content, ..
         }| token_kind == &TokenKind::Ident && *content == "declare_clippy_lint",
    ) {
        let start = range.start;
        let mut iter = iter
            .by_ref()
            .filter(|t| !matches!(t.token_kind, TokenKind::Whitespace | TokenKind::LineComment { .. }));
        // matches `!{`
        match_tokens!(iter, Bang OpenBrace);
        match iter.next() {
            // #[clippy::version = "version"] pub
            Some(LintDeclSearchResult {
                token_kind: TokenKind::Pound,
                ..
            }) => {
                match_tokens!(iter, OpenBracket Ident Colon Colon Ident Eq Literal{..} CloseBracket Ident);
            },
            // pub
            Some(LintDeclSearchResult {
                token_kind: TokenKind::Ident,
                ..
            }) => (),
            _ => continue,
        }

        let (name, group, desc) = match_tokens!(
            iter,
            // LINT_NAME
            Ident(name) Comma
            // group,
            Ident(group) Comma
            // "description"
            Literal{..}(desc)
        );

        if let Some(end) = iter.find_map(|t| {
            if let LintDeclSearchResult {
                token_kind: TokenKind::CloseBrace,
                range,
                ..
            } = t
            {
                Some(range.end)
            } else {
                None
            }
        }) {
            lints.push(Lint::new(name, group, desc, module, start..end));
        }
    }
}

/// Parse a source file looking for `declare_deprecated_lint` macro invocations.
fn parse_deprecated_contents(contents: &str, deprecated: &mut Vec<DeprecatedLint>, renamed: &mut Vec<RenamedLint>) {
    let Some((_, contents)) = contents.split_once("\ndeclare_with_version! { DEPRECATED") else {
        return;
    };
    let Some((deprecated_src, renamed_src)) = contents.split_once("\ndeclare_with_version! { RENAMED") else {
        return;
    };

    for line in deprecated_src.lines() {
        let mut offset = 0usize;
        let mut iter = tokenize(line).map(|t| {
            let range = offset..offset + t.len as usize;
            offset = range.end;

            LintDeclSearchResult {
                token_kind: t.kind,
                content: &line[range.clone()],
                range,
            }
        });

        let (name, reason) = match_tokens!(
            iter,
            // ("old_name",
            Whitespace OpenParen Literal{kind: LiteralKind::Str{..},..}(name) Comma
            // "new_name"),
            Whitespace Literal{kind: LiteralKind::Str{..},..}(reason) CloseParen Comma
        );
        deprecated.push(DeprecatedLint::new(name, reason));
    }
    for line in renamed_src.lines() {
        let mut offset = 0usize;
        let mut iter = tokenize(line).map(|t| {
            let range = offset..offset + t.len as usize;
            offset = range.end;

            LintDeclSearchResult {
                token_kind: t.kind,
                content: &line[range.clone()],
                range,
            }
        });

        let (old_name, new_name) = match_tokens!(
            iter,
            // ("old_name",
            Whitespace OpenParen Literal{kind: LiteralKind::Str{..},..}(old_name) Comma
            // "new_name"),
            Whitespace Literal{kind: LiteralKind::Str{..},..}(new_name) CloseParen Comma
        );
        renamed.push(RenamedLint::new(old_name, new_name));
    }
}

/// Removes the line splices and surrounding quotes from a string literal
fn remove_line_splices(s: &str) -> String {
    let s = s
        .strip_prefix('r')
        .unwrap_or(s)
        .trim_matches('#')
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or_else(|| panic!("expected quoted string, found `{s}`"));
    let mut res = String::with_capacity(s.len());
    unescape::unescape_unicode(s, unescape::Mode::Str, &mut |range, ch| {
        if ch.is_ok() {
            res.push_str(&s[range]);
        }
    });
    res
}
fn try_rename_file(old_name: &Path, new_name: &Path) -> bool {
    match OpenOptions::new().create_new(true).write(true).open(new_name) {
        Ok(file) => drop(file),
        Err(e) if matches!(e.kind(), io::ErrorKind::AlreadyExists | io::ErrorKind::NotFound) => return false,
        Err(e) => panic_file(e, new_name, "create"),
    };
    match fs::rename(old_name, new_name) {
        Ok(()) => true,
        Err(e) => {
            drop(fs::remove_file(new_name));
            if e.kind() == io::ErrorKind::NotFound {
                false
            } else {
                panic_file(e, old_name, "rename");
            }
        },
    }
}

#[allow(clippy::needless_pass_by_value)]
fn panic_file(error: io::Error, name: &Path, action: &str) -> ! {
    panic!("failed to {action} file `{}`: {error}", name.display())
}

fn insert_at_marker(text: &str, marker: &str, new_text: &str) -> Option<String> {
    let i = text.find(marker)?;
    let (pre, post) = text.split_at(i);
    Some([pre, new_text, post].into_iter().collect())
}

fn rewrite_file(path: &Path, f: impl FnOnce(&str) -> Option<String>) {
    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(path)
        .unwrap_or_else(|e| panic_file(e, path, "open"));
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .unwrap_or_else(|e| panic_file(e, path, "read"));
    if let Some(new_contents) = f(&buf) {
        file.rewind().unwrap_or_else(|e| panic_file(e, path, "write"));
        file.write_all(new_contents.as_bytes())
            .unwrap_or_else(|e| panic_file(e, path, "write"));
        file.set_len(new_contents.len() as u64)
            .unwrap_or_else(|e| panic_file(e, path, "write"));
    }
}

fn write_file(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap_or_else(|e| panic_file(e, path, "write"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_contents() {
        static CONTENTS: &str = r#"
            declare_clippy_lint! {
                #[clippy::version = "Hello Clippy!"]
                pub PTR_ARG,
                style,
                "really long \
                text"
            }

            declare_clippy_lint!{
                #[clippy::version = "Test version"]
                pub DOC_MARKDOWN,
                pedantic,
                "single line"
            }
        "#;
        let mut result = Vec::new();
        parse_contents(CONTENTS, "module_name", &mut result);
        for r in &mut result {
            r.declaration_range = Range::default();
        }

        let expected = vec![
            Lint::new(
                "ptr_arg",
                "style",
                "\"really long text\"",
                "module_name",
                Range::default(),
            ),
            Lint::new(
                "doc_markdown",
                "pedantic",
                "\"single line\"",
                "module_name",
                Range::default(),
            ),
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn test_usable_lints() {
        let lints = vec![
            Lint::new(
                "should_assert_eq2",
                "Not Deprecated",
                "\"abc\"",
                "module_name",
                Range::default(),
            ),
            Lint::new(
                "should_assert_eq2",
                "internal",
                "\"abc\"",
                "module_name",
                Range::default(),
            ),
            Lint::new(
                "should_assert_eq2",
                "internal_style",
                "\"abc\"",
                "module_name",
                Range::default(),
            ),
        ];
        let expected = vec![Lint::new(
            "should_assert_eq2",
            "Not Deprecated",
            "\"abc\"",
            "module_name",
            Range::default(),
        )];
        assert_eq!(expected, Lint::usable_lints(&lints));
    }

    #[test]
    fn test_by_lint_group() {
        let lints = vec![
            Lint::new("should_assert_eq", "group1", "\"abc\"", "module_name", Range::default()),
            Lint::new(
                "should_assert_eq2",
                "group2",
                "\"abc\"",
                "module_name",
                Range::default(),
            ),
            Lint::new("incorrect_match", "group1", "\"abc\"", "module_name", Range::default()),
        ];
        let mut expected: HashMap<String, Vec<Lint>> = HashMap::new();
        expected.insert("group1".to_string(), vec![
            Lint::new("should_assert_eq", "group1", "\"abc\"", "module_name", Range::default()),
            Lint::new("incorrect_match", "group1", "\"abc\"", "module_name", Range::default()),
        ]);
        expected.insert("group2".to_string(), vec![Lint::new(
            "should_assert_eq2",
            "group2",
            "\"abc\"",
            "module_name",
            Range::default(),
        )]);
        assert_eq!(expected, Lint::by_lint_group(lints.into_iter()));
    }
}
