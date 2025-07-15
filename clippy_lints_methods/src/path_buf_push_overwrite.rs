use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::symbol::sym;
use std::path::{Component, Path};

declare_clippy_lint! {
    /// ### What it does
    ///* Checks for [push](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.push)
    /// calls on `PathBuf` that can cause overwrites.
    ///
    /// ### Why is this bad?
    /// Calling `push` with a root path at the start can overwrite the
    /// previous defined path.
    ///
    /// ### Example
    /// ```no_run
    /// use std::path::PathBuf;
    ///
    /// let mut x = PathBuf::from("/foo");
    /// x.push("/bar");
    /// assert_eq!(x, PathBuf::from("/bar"));
    /// ```
    /// Could be written:
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    ///
    /// let mut x = PathBuf::from("/foo");
    /// x.push("bar");
    /// assert_eq!(x, PathBuf::from("/foo/bar"));
    /// ```
    #[clippy::version = "1.36.0"]
    pub PATH_BUF_PUSH_OVERWRITE,
    nursery,
    "calling `push` with file system root on `PathBuf` can overwrite it"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, arg: &'tcx Expr<'_>) {
    if let Some(method_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_method(method_id)
        && is_type_diagnostic_item(cx, cx.tcx.type_of(impl_id).instantiate_identity(), sym::PathBuf)
        && let ExprKind::Lit(lit) = arg.kind
        && let LitKind::Str(ref path_lit, _) = lit.node
        && let pushed_path = Path::new(path_lit.as_str())
        && let Some(pushed_path_lit) = pushed_path.to_str()
        && pushed_path.has_root()
        && let Some(root) = pushed_path.components().next()
        && root == Component::RootDir
    {
        span_lint_and_sugg(
            cx,
            PATH_BUF_PUSH_OVERWRITE,
            lit.span,
            "calling `push` with '/' or '\\' (file system root) will overwrite the previous path definition",
            "try",
            format!("\"{}\"", pushed_path_lit.trim_start_matches(['/', '\\'])),
            Applicability::MaybeIncorrect,
        );
    }
}
