use clippy_utils::ast_utils::is_cfg_test;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet;
use rustc_ast::ast::{self, Inline, ItemKind, ModKind};
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext as _, impl_lint_pass};
use rustc_span::def_id::LOCAL_CRATE;
use rustc_span::{FileName, Ident, SourceFile, Span, SyntaxContext, sym};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

declare_clippy_lint! {
    /// ### What it does
    /// Checks that module layout does not use inline modules.
    /// Inline test modules (anything annotated with `#[cfg(test)]`) are not linted.
    ///
    /// ### Why restrict this?
    /// Having multiple module layout styles in a project can be confusing.
    ///
    /// ### Known problems
    /// The lint currently doesn't lint inline modules whose parent module is annotated
    /// with the `#[path]` attribute.
    ///
    /// ### Example
    /// ```ignore
    /// // in `src/lib.rs`
    /// mod foo {
    ///    /* module contents */
    /// }
    /// ```
    /// Use instead:
    /// ```ignore
    /// // in `src/lib.rs`
    /// mod foo;
    /// // in `src/foo.rs` (or `src/foo/mod.rs`)
    /// /* module contents */
    /// ```
    #[clippy::version = "1.97.0"]
    pub INLINE_MODULES,
    restriction,
    "checks that module layout does not use inline modules"
}

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
    /// Checks that the source file of an out-of-line module is not overridden with the
    /// built-in `#[path = "..."]` attribute.
    ///
    /// ### Why restrict this?
    /// With the conventional module layout, the source file of a module is predictable from
    /// its name, which keeps the module tree navigable for readers and for tooling.
    ///
    /// ### Known problems
    /// `#[path]` is the only way to express a non-standard module layout, and it is
    /// legitimately needed for generated code, platform-specific implementations, and tests.
    /// Those uses have to be annotated with `#[allow(clippy::path_attribute)]`.
    ///
    /// This lint does not fire for the `#[path]` attribute on an inline module, which only
    /// changes the directory that nested `mod` declarations are resolved against.
    ///
    /// ### Example
    /// ```ignore
    /// // in `src/lib.rs`
    /// #[path = "implementation.rs"]
    /// mod parser;
    /// ```
    /// Use instead:
    /// ```ignore
    /// // in `src/lib.rs`
    /// mod parser;
    /// // in `src/parser.rs` (or `src/parser/mod.rs`)
    /// ```
    #[clippy::version = "1.100.0"]
    pub PATH_ATTRIBUTE,
    restriction,
    "checks that the `#[path]` attribute is not used to override the source file of a module"
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

impl_lint_pass!(ModStyle => [
    INLINE_MODULES,
    MOD_MODULE_FILES,
    PATH_ATTRIBUTE,
    SELF_NAMED_MODULE_FILES,
]);

pub struct ModState {
    mod_file: Arc<SourceFile>,
    mod_ident: Ident,
    path_from_working_dir: Option<PathBuf>,
    contains_external: bool,
    has_path_attr: bool,
    is_cfg_test: bool,
}

#[derive(Default)]
pub struct ModStyle {
    working_dir: Option<PathBuf>,
    regular_mod_stack: Vec<ModState>,
    inline_mod_stack: Vec<ModState>,
}

impl ModStyle {
    fn inside_cfg_test_inline_mod(&self) -> bool {
        self.inline_mod_stack.last().is_some_and(|last| last.is_cfg_test)
    }

    fn get_relative_path_from_working_dir(&self, file: &SourceFile) -> Option<PathBuf> {
        try_trim_file_path_prefix(file, self.working_dir.as_ref()?).map(Path::to_path_buf)
    }

    /// All of the lints in this pass are `restriction` lints, so the module bookkeeping below is
    /// pure overhead when none of them can fire. `lints` must be kept in sync between
    /// [`EarlyLintPass::check_item`] and [`EarlyLintPass::check_item_post`] so that the
    /// `ModState` stacks stay balanced.
    fn all_allowed(cx: &EarlyContext<'_>, lints: &[&'static Lint]) -> bool {
        lints.iter().all(|lint| cx.builder.lint_level_spec(lint).is_allow())
    }
}

impl EarlyLintPass for ModStyle {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _: &ast::Crate) {
        self.working_dir = cx.sess().source_map().working_dir().local_path().map(Path::to_path_buf);
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        if Self::all_allowed(
            cx,
            &[
                INLINE_MODULES,
                MOD_MODULE_FILES,
                PATH_ATTRIBUTE,
                SELF_NAMED_MODULE_FILES,
            ],
        ) {
            return;
        }
        if let ItemKind::Mod(_, mod_ident, ModKind::Loaded(_, inline_or_not, mod_spans, ..)) = &item.kind {
            let path_attr = item.attrs.iter().find(|attr| attr.has_name(sym::path));
            let has_path_attr = path_attr.is_some();
            let mod_file = cx.sess().source_map().lookup_source_file(mod_spans.inner_span.lo());
            let path_from_working_dir = self.get_relative_path_from_working_dir(&mod_file);
            let current = ModState {
                mod_file,
                mod_ident: *mod_ident,
                path_from_working_dir,
                contains_external: false,
                has_path_attr,
                is_cfg_test: self.inside_cfg_test_inline_mod() || is_cfg_test(item),
            };
            match inline_or_not {
                Inline::Yes => {
                    if !current.is_cfg_test
                        && !item.span.from_expansion()
                        && self.regular_mod_stack.last().is_none_or(|last| !last.has_path_attr)
                        && let Some(path) = &current.path_from_working_dir
                    {
                        let opt_extra_mod_dir = self.regular_mod_stack.last().and_then(|last| {
                            if last.path_from_working_dir.as_ref()?.ends_with("mod.rs") {
                                None
                            } else {
                                Some(&last.mod_ident)
                            }
                        });
                        check_inline_module(
                            cx,
                            path,
                            *mod_ident,
                            item.span,
                            opt_extra_mod_dir
                                .into_iter()
                                .chain(self.inline_mod_stack.iter().map(|state| &state.mod_ident)),
                        );
                    }
                    self.inline_mod_stack.push(current);
                },
                Inline::No { .. } => {
                    if let Some(path_attr) = path_attr
                        && !path_attr.span.from_expansion()
                        // `mod_spans` points at the *overridden* file, so the declaration site is
                        // the file the attribute itself is written in.
                        && let Some(decl_file) = self.get_relative_path_from_working_dir(
                            &cx.sess().source_map().lookup_source_file(path_attr.span.lo()),
                        )
                    {
                        // The suggested paths are anchored at the directory that the enclosing
                        // modules resolve against, not at the directory of the declaration.
                        let opt_extra_mod_dir = self.regular_mod_stack.last().and_then(|last| {
                            if last.path_from_working_dir.as_ref()?.ends_with("mod.rs") {
                                None
                            } else {
                                Some(&last.mod_ident)
                            }
                        });
                        check_path_attribute(
                            cx,
                            path_attr,
                            current.mod_ident,
                            &decl_file,
                            opt_extra_mod_dir
                                .into_iter()
                                .chain(self.inline_mod_stack.iter().map(|state| &state.mod_ident)),
                        );
                    }
                    if !has_path_attr && let Some(last) = self.regular_mod_stack.last_mut() {
                        last.contains_external = true;
                    }
                    self.regular_mod_stack.push(current);
                },
            }
        }
    }

    fn check_item_post(&mut self, cx: &EarlyContext<'_>, item: &ast::Item) {
        if Self::all_allowed(
            cx,
            &[
                INLINE_MODULES,
                MOD_MODULE_FILES,
                PATH_ATTRIBUTE,
                SELF_NAMED_MODULE_FILES,
            ],
        ) {
            return;
        }

        if let ItemKind::Mod(.., ModKind::Loaded(_, inline_or_not, ..)) = &item.kind {
            match inline_or_not {
                Inline::Yes => {
                    self.inline_mod_stack.pop();
                },
                Inline::No { .. } => {
                    if let Some(current) = self.regular_mod_stack.pop()
                        && let Some(path) = &current.path_from_working_dir
                        && !current.has_path_attr
                    {
                        if current.contains_external {
                            check_self_named_module(cx, path, &current.mod_file);
                        }
                        check_mod_module(cx, path, &current.mod_file);
                    }
                },
            }
        }
    }
}

fn check_self_named_module(cx: &EarlyContext<'_>, path: &Path, file: &SourceFile) {
    if !path.ends_with("mod.rs") {
        let mut mod_folder = path.with_extension("");
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
    if path.ends_with("mod.rs")
        && !path
            .components()
            .filter_map(|c| if let Component::Normal(d) = c { Some(d) } else { None })
            .take_while(|&c| c != "src")
            .any(|c| c == "tests")
    {
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

fn check_inline_module<'a>(
    cx: &EarlyContext<'_>,
    path: &Path,
    mod_ident: Ident,
    mod_span: Span,
    ancestor_mods: impl Iterator<Item = &'a Ident>,
) {
    let Some(parent) = path.parent() else { return };

    span_lint_and_then(cx, INLINE_MODULES, mod_span, "inline module found", |diag| {
        let mut mod_folder = parent.to_path_buf();
        mod_folder.extend(ancestor_mods.map(Ident::as_str));
        let mod_name = mod_ident.as_str();

        let mod_file = mod_folder.join(mod_name).join("mod.rs");
        let self_named_mod_file = mod_folder.join(format!("{mod_name}.rs"));
        let outlined_mod = snippet(cx, mod_span.with_hi(mod_ident.span.hi()), "");
        diag.help(format!(
            "move the contents of the module to `{}` or `{}`, and replace this with `{outlined_mod};`",
            self_named_mod_file.display(),
            mod_file.display(),
        ));
    });
}

/// `decl_file` is the file the `mod` declaration is written in, i.e. the file *containing* the
/// `#[path]` attribute, not the file it points at. `ancestor_mods` are the modules the
/// declaration is nested in, innermost last.
fn check_path_attribute<'a>(
    cx: &EarlyContext<'_>,
    path_attr: &ast::Attribute,
    mod_ident: Ident,
    decl_file: &Path,
    ancestor_mods: impl Iterator<Item = &'a Ident>,
) {
    let Some(parent) = decl_file.parent() else { return };
    let mod_name = mod_ident.as_str();
    // `#[path]` is resolved relative to the directory the enclosing modules resolve against, so
    // this is the string the user wrote. A `None` here (a macro-generated value) only costs us
    // the more precise help.
    let declared_file = path_attr.value_str();

    let mut mod_folder = parent.to_path_buf();
    mod_folder.extend(ancestor_mods.map(Ident::as_str));

    span_lint_and_then(
        cx,
        PATH_ATTRIBUTE,
        path_attr.span,
        "the `#[path]` attribute overrides the source file of a module",
        |diag| {
            let self_named_mod_file = mod_folder.join(format!("{mod_name}.rs"));
            let mod_file_folder = mod_folder.join(mod_name).join("mod.rs");
            match declared_file {
                Some(declared_file) => diag.help(format!(
                    "move `{declared_file}` to `{}` or `{}`, and remove the `#[path]` attribute",
                    self_named_mod_file.display(),
                    mod_file_folder.display(),
                )),
                None => diag.help(format!(
                    "remove the `#[path]` attribute and declare the module in `{}` or `{}`",
                    self_named_mod_file.display(),
                    mod_file_folder.display(),
                )),
            };
        },
    );
}

fn try_trim_file_path_prefix<'a>(file: &'a SourceFile, prefix: &'a Path) -> Option<&'a Path> {
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
