use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_ast::{LitKind, StrStyle};
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::sym;
use std::fmt::Write;

declare_clippy_lint! {
    /// ### What it does
    /// Looks for calls to `Path::ends_with` calls where the argument looks like a file extension.
    ///
    /// By default, Clippy has a short list of known filenames that start with a dot
    /// but aren't necessarily file extensions (e.g. the `.git` folder), which are allowed by default.
    /// The `allowed-dotfiles` configuration can be used to allow additional
    /// file extensions that Clippy should not lint.
    ///
    /// ### Why is this bad?
    /// This doesn't actually compare file extensions. Rather, `ends_with` compares the given argument
    /// to the last **component** of the path and checks if it matches exactly.
    ///
    /// ### Known issues
    /// File extensions are often at most three characters long, so this only lints in those cases
    /// in an attempt to avoid false positives.
    /// Any extension names longer than that are assumed to likely be real path components and are
    /// therefore ignored.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::path::Path;
    /// fn is_markdown(path: &Path) -> bool {
    ///     path.ends_with(".md")
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::path::Path;
    /// fn is_markdown(path: &Path) -> bool {
    ///     path.extension().is_some_and(|ext| ext == "md")
    /// }
    /// ```
    #[clippy::version = "1.74.0"]
    pub PATH_ENDS_WITH_EXT,
    suspicious,
    "attempting to compare file extensions using `Path::ends_with`"
}

pub const DEFAULT_ALLOWED_DOTFILES: &[&str] = &[
    "git", "svn", "gem", "npm", "vim", "env", "rnd", "ssh", "vnc", "smb", "nvm", "bin",
];

pub(super) fn check(
    cx: &LateContext<'_>,
    recv: &Expr<'_>,
    path: &Expr<'_>,
    expr: &Expr<'_>,
    msrv: Msrv,
    allowed_dotfiles: &FxHashSet<&'static str>,
) {
    if is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv).peel_refs(), sym::Path)
        && !path.span.from_expansion()
        && let ExprKind::Lit(lit) = path.kind
        && let LitKind::Str(path, StrStyle::Cooked) = lit.node
        && let Some(path) = path.as_str().strip_prefix('.')
        && (1..=3).contains(&path.len())
        && !allowed_dotfiles.contains(path)
        && path.chars().all(char::is_alphanumeric)
    {
        let mut sugg = snippet(cx, recv.span, "..").into_owned();
        if msrv.meets(cx, msrvs::OPTION_RESULT_IS_VARIANT_AND) {
            let _ = write!(sugg, r#".extension().is_some_and(|ext| ext == "{path}")"#);
        } else {
            let _ = write!(sugg, r#".extension().map_or(false, |ext| ext == "{path}")"#);
        }

        span_lint_and_sugg(
            cx,
            PATH_ENDS_WITH_EXT,
            expr.span,
            "this looks like a failed attempt at checking for the file extension",
            "try",
            sugg,
            Applicability::MaybeIncorrect,
        );
    }
}
