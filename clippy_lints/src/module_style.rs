use clippy_utils::diagnostics::span_lint_and_then;
use rustc_ast::ast::{self, Inline, ItemKind, ModKind};
use rustc_data_structures::fx::FxHashSet;
use rustc_lint::{EarlyContext, EarlyLintPass, Level, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LOCAL_CRATE;
use rustc_span::{FileName, SourceFile, Span, SyntaxContext};
use std::path::{Path, PathBuf};

declare_clippy_lint! {
    /// ### What it does
    /// Checks that module layout uses only self named module files; bans `mod.rs` files.
    ///
    /// ### Why restrict this?
    /// Having multiple module layout styles in a project can be confusing.
    ///
    /// ### Example
    /// ```text
    /// src/
    ///   stuff/
    ///     stuff_files.rs
    ///     mod.rs
    ///   lib.rs
    /// ```
    /// Use instead:
    /// ```text
    /// src/
    ///   stuff/
    ///     stuff_files.rs
    ///   stuff.rs
    ///   lib.rs
    /// ```
    #[clippy::version = "1.57.0"]
    pub MOD_MODULE_FILES,
    restriction,
    "checks that module layout is consistent"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks that module layout uses only `mod.rs` files.
    ///
    /// ### Why restrict this?
    /// Having multiple module layout styles in a project can be confusing.
    ///
    /// ### Example
    /// ```text
    /// src/
    ///   stuff/
    ///     stuff_files.rs
    ///   stuff.rs
    ///   lib.rs
    /// ```
    /// Use instead:
    /// ```text
    /// src/
    ///   stuff/
    ///     stuff_files.rs
    ///     mod.rs
    ///   lib.rs
    /// ```
    #[clippy::version = "1.57.0"]
    pub SELF_NAMED_MODULE_FILES,
    restriction,
    "checks that module layout is consistent"
}

impl_lint_pass!(ModStyle => [MOD_MODULE_FILES, SELF_NAMED_MODULE_FILES]);

#[derive(Default)]
pub struct ModStyle {
    mod_folders: FxHashSet<PathBuf>,
    working_dir: Option<PathBuf>,
}

impl EarlyLintPass for ModStyle {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _: &ast::Crate) {
        let Some(working_dir) = cx.sess().opts.working_dir.local_path() else {
            return;
        };
        if cx.builder.lint_level(SELF_NAMED_MODULE_FILES).level != Level::Allow {
            self.mod_folders = cx
                .sess()
                .source_map()
                .files()
                .iter()
                .filter_map(|file| {
                    let path = try_trim_path_prefix(file, working_dir)?;
                    if path.extension()?.eq("rs") {
                        let mut mod_folder = path.to_path_buf();
                        mod_folder.pop();
                        Some(mod_folder)
                    } else {
                        None
                    }
                })
                .collect();
        }
        self.working_dir = Some(working_dir.to_path_buf());
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        if cx.builder.lint_level(MOD_MODULE_FILES).level == Level::Allow
            && cx.builder.lint_level(SELF_NAMED_MODULE_FILES).level == Level::Allow
        {
            return;
        }
        if let ItemKind::Mod(.., mod_kind) = &item.kind
            && let ModKind::Loaded(_, inline, mod_spans, _) = mod_kind
            && matches!(inline, Inline::No)
            && let mod_file = cx.sess().source_map().lookup_source_file(mod_spans.inner_span.lo())
            && let Some(working_dir) = self.working_dir.as_ref()
            && let Some(mod_path) = try_trim_path_prefix(&mod_file, working_dir.as_path())
        {
            check_mod_module(cx, mod_path, &mod_file);
            check_self_named_module(cx, mod_path, &mod_file, &self.mod_folders);
        }
    }
}

fn try_trim_path_prefix<'a>(file: &'a SourceFile, prefix: &'a Path) -> Option<&'a Path> {
    if let FileName::Real(name) = &file.name
        && let Some(mut path) = name.local_path()
        && file.cnum == LOCAL_CRATE
    {
        if !path.is_relative() {
            path = path.strip_prefix(prefix).ok()?;
        }
        Some(path)
    } else {
        None
    }
}

fn check_self_named_module(cx: &EarlyContext<'_>, path: &Path, file: &SourceFile, mod_folders: &FxHashSet<PathBuf>) {
    if !path.ends_with("mod.rs")
        && let mut mod_folder = path.with_extension("")
        && mod_folders.contains(&mod_folder)
    {
        span_lint_and_then(
            cx,
            SELF_NAMED_MODULE_FILES,
            Span::new(file.start_pos, file.start_pos, SyntaxContext::root(), None),
            format!("`mod.rs` files are required, found `{}`", path.display()),
            |diag| {
                mod_folder.push("mod.rs");
                diag.help(format!("move `{}` to `{}`", path.display(), mod_folder.display()));
            },
        );
    }
}

/// We should not emit a lint for test modules in the presence of `mod.rs`.
/// Using `mod.rs` in integration tests is a [common pattern](https://doc.rust-lang.org/book/ch11-03-test-organization.html#submodules-in-integration-test)
/// for code-sharing between tests.
fn check_mod_module(cx: &EarlyContext<'_>, path: &Path, file: &SourceFile) {
    if path.ends_with("mod.rs") && !path.starts_with("tests") {
        span_lint_and_then(
            cx,
            MOD_MODULE_FILES,
            Span::new(file.start_pos, file.start_pos, SyntaxContext::root(), None),
            format!("`mod.rs` files are not allowed, found `{}`", path.display()),
            |diag| {
                let mut mod_file = path.to_path_buf();
                mod_file.pop();
                mod_file.set_extension("rs");

                diag.help(format!("move `{}` to `{}`", path.display(), mod_file.display()));
            },
        );
    }
}
