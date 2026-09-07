use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{eq_expr_value, get_enclosing_block, sym};
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty::layout::HasTyCtxt;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for occurrences where a file is being created and then the same
    /// file path is used to set file permissions.
    ///
    /// ### Why is this bad?
    /// Creating a file and then setting the file's permissions in two different
    /// syscalls can cause a path-based Time-Of-Check to Time-Of-Use (TOCTOU) race condition.
    /// Between the two calls, another process can add or replace the created file,
    /// which could lead to an unintended file having its permissions changed. This
    /// could be dangerous in the case of a malicious file replacing the created file
    /// and having its privileges escalated from a set permission syscall.
    ///
    /// ### Example
    /// Creating a directory and setting permissions on that directory:
    /// ```no_run
    /// use std::path::Path;
    /// fn example(path: Path) -> Result<()> {
    ///     // Create with default permissions
    ///     std::fs::create_dir(&path)?;
    ///     // Fix up permissions
    ///     std::fs::set_permissions(&path, Permissions::from_mode(0o700))?;
    ///     Ok(())
    /// }
    /// ```
    /// 
    /// Use `DirBuilderExt::mode()` (Unix-exclusive) instead:
    /// ```no_run
    /// use std::path::Path;
    /// use std::fs::DirBuilder;
    /// use std::os::unix::fs::DirBuilderExt;
    /// fn example(path: Path) -> Result<()> {
    ///     // Atomically create and set permissions of the new directory
    ///     DirBuilder::new().mode(0o700).create(&path)?;
    ///     Ok(())
    /// }
    /// ```
    /// 
    /// ### Example 2
    /// Creating a regular file and setting permissions on that file:
    /// ```no_run
    /// use std::path::Path;
    /// use std::fs::File;
    /// fn example(path: Path) -> Result<()> {
    ///     // Create regular file with default permissions
    ///     let mut f = File::create(&path)?;
    ///     // Fix up permissions
    ///     std::fs::set_permissions(&path, Permissions::from_mode(0o700))?;
    ///     Ok(())
    /// }
    /// 
    /// ```
    /// 
    /// Use `File::options` and `OpenOptionsExt::mode` (Unix-exclusive) or `OpenOptionsExt::access_mode` 
    /// (Windows exclusive) instead:
    /// ```no_run
    /// use std::path::Path;
    /// use std::fs::File;
    /// use std::os::unix::fs::OpenOptionsExt;
    /// fn example(path: Path) -> Result<()> {
    ///     // Create regular file with default permissions
    ///     let mut f = File::options().mode(0o700).create(true).open(&path)?;
    ///     Ok(())
    /// }
    /// ```
    #[clippy::version = "1.100.0"]
    pub FILE_CREATION_AND_SET_PERMISSIONS,
    suspicious,
    "modifying permissions of a file that was created through an earlier function call"
}
declare_lint_pass!(FileCreationAndSetPermissions => [FILE_CREATION_AND_SET_PERMISSIONS]);

/// An enum containing all the different possible file types
/// to lint on
enum FileType {
    Directory,
    RegularFile,
}

/// Checks if the expression is `std::fs::set_permissions` or `std::fs::set_permissions_nofollow`
fn check_expr_is_set_perm<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Call(func, [path, _]) = e.kind
        && let ExprKind::Path(ref func_qpath) = func.kind 
        && let Some(def_id) = cx.qpath_res(func_qpath, func.hir_id).opt_def_id()
        && (cx.tcx.is_diagnostic_item(sym::fs_set_permissions, def_id) ||
            cx.tcx.is_diagnostic_item(sym::fs_set_permissions_nofollow, def_id)) {
        return Some((func, path));
    }
    None
}

/// Checks if the expression is `std::fs::create_dir` or `std::fs::create_dir_all`
fn check_expr_is_create_dir<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::Call(func, [path]) = e.kind
        && let ExprKind::Path(ref func_qpath) = func.kind 
        && let Some(def_id) = cx.qpath_res(func_qpath, func.hir_id).opt_def_id()
        && (cx.tcx.is_diagnostic_item(sym::fs_create_dir, def_id) ||
            cx.tcx.is_diagnostic_item(sym::fs_create_dir_all, def_id))
             {
        return Some(path);
    }
    None
}

/// Checks if the expression is `File::create`
fn check_expr_is_file_create<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::Call(func, [path]) = e.kind
        && let ExprKind::Path(QPath::TypeRelative(ty, segment)) = func.kind
        && ty.basic_res().is_diag_item(&cx.tcx(), sym::File)
        && (segment.ident.name == sym::create || segment.ident.name == sym::create_new || segment.ident.name == sym::create_buffered)
    {
        return Some(path);
    }
    None
}

/// This function examines all expressions up until the original `std::fs::set_permissions`
/// or `std::fs::set_permissions_nofollow` (passed in as `set_perm_expr`). If any earlier 
/// expressions indicates that it is either a `std::fs::create_dir*` or `File::create*`, the
/// path argument is compared to between the original `std::fs::set_permissions*` and the 
/// file creation function. If it is equal, then the first offending file creation function is
/// returned alongside the type of file it was.
fn check_file_was_created_earlier<'tcx>(
    cx: &LateContext<'tcx>,
    set_perm_expr: &'tcx Expr<'tcx>,
    set_perm_func: &'tcx Expr<'tcx>,
    set_perm_path: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, FileType)> {
    // Fetch the entire block that the set_permission expression belonged to
    let parent_block = if let Some(block) = get_enclosing_block(cx, set_perm_expr.hir_id) {
        block
    } else {
        return None;
    };

    // From the beginning of the block until the original set_permission expression,
    // examine each expression
    for_each_expr(cx.tcx(), parent_block, |e| {
        // Checks if current expression is a `std::fs::set_permissions*` function
        if let Some((func, path)) = check_expr_is_set_perm(cx, e) 
            && eq_expr_value(cx, e.span.ctxt(), func, set_perm_func) 
            && eq_expr_value(cx, e.span.ctxt(), path, set_perm_path) {
            return ControlFlow::Break(None);
        } 
        // Checks if current expression is a `std::fs::create_dir*` function
        // and if the path arguments are the same
        else if let Some(path) = check_expr_is_create_dir(cx, e)
            && eq_expr_value(cx, e.span.ctxt(), path, set_perm_path) {
            return ControlFlow::Break(Some((e, FileType::Directory)));
        } 
        // Checks if current expression is a `File::create*` function
        // and if the path arguments are the same
        else if let Some(path) = check_expr_is_file_create(cx, e)
            && eq_expr_value(cx, e.span.ctxt(), path, set_perm_path) {
            return ControlFlow::Break(Some((e, FileType::RegularFile)));
        }
        ControlFlow::Continue(())
    }).unwrap_or(None)
}

impl<'tcx> LateLintPass<'tcx> for FileCreationAndSetPermissions {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let Some((func,path)) = check_expr_is_set_perm(cx, expr)
            && let Some((create_file_expr, filetype)) = check_file_was_created_earlier(cx, expr, func, path)
        {
            let help = match filetype {
                FileType::Directory => {
                    "creating a directory via `std::fs::create_dir` or `std::fs::create_dir_all` and modifying \
                    permissions of the directory can lead to a TOCTOU race condition; on unix, consider using \
                    `DirBuilderExt::new().mode()` instead to atomically create a directory + set certain permissions."
               },
               FileType::RegularFile => {
                    "creating a regular file via `File::create*` and modifying the permissions of the file via its path \
                    in separate calls can lead to a TOCTOU race condition; consider using `File::options` alongside `OpenOptionsExt` \
                    to create a file with certain permissions in advance."
               }
            };
            span_lint_and_then(
                cx,
                FILE_CREATION_AND_SET_PERMISSIONS,
                expr.span,
                "modifying permissions of a file that was created through an earlier function call.",
                |diag| {
                    diag.span_note(create_file_expr.span, "create file operation occurred here.");
                    diag.span_note(expr.span, "the set permission operation occurred here.");
                    diag.help(help);
                },
            );
        }
    }   
}
