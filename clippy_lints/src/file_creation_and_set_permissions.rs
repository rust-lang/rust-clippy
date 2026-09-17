use std::fmt::Display;
use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeResPath as _};
use clippy_utils::visitors::for_each_expr;
use clippy_utils::{eq_expr_value, get_enclosing_block, sym};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass, Lint, declare_lint_pass};
use rustc_middle::ty::layout::HasTyCtxt as _;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for occurrences where a directory is being created and then the same
    /// file path is used to set file permissions.
    ///
    /// ### Why is this bad?
    /// Creating a directory and then setting the directory's permissions in two different
    /// syscalls can cause a path-based Time-Of-Check to Time-Of-Use (TOCTOU) race condition.
    /// Between the two calls, another process can delete and replace the created directory
    /// with a different file, which could lead to an unintended file having its permissions changed.
    /// This could be dangerous in the case of a malicious file replacing the created directory
    /// and having its privileges escalated from a set permission syscall.
    ///
    /// ### Example
    /// Creating a directory and setting permissions on that directory:
    /// ```no_run
    /// use std::path::Path;
    /// use std::fs::Permissions;
    /// use std::os::unix::fs::PermissionsExt;
    /// fn example(path: &Path) -> std::io::Result<()> {
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
    /// fn example(path: &Path) -> std::io::Result<()> {
    ///     // Atomically create and set permissions of the new directory
    ///     DirBuilder::new().mode(0o700).create(&path)?;
    ///     Ok(())
    /// }
    /// ```
    #[clippy::version = "1.100.0"]
    pub CREATE_DIR_AND_SET_PERMISSIONS,
    suspicious,
    "modifying permissions of a directory that was created through an earlier function call"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for occurrences where a regular file is being created and then the same
    /// file path is used to set file permissions.
    ///
    /// ### Why is this bad?
    /// Creating a regular file and then setting the file's permissions in two different
    /// syscalls can cause a path-based Time-Of-Check to Time-Of-Use (TOCTOU) race condition.
    /// Between the two calls, another process can delete and replace the created file,
    /// which could lead to an unintended file having its permissions changed. This
    /// could be dangerous in the case of a malicious file replacing the created file
    /// and having its privileges escalated from a set permission syscall.
    ///
    /// ### Example
    /// Creating a regular file and setting permissions on that file:
    /// ```no_run
    /// use std::path::Path;
    /// use std::fs::{File, Permissions};
    /// use std::os::unix::fs::PermissionsExt;
    /// fn example(path: &Path) -> std::io::Result<()> {
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
    /// fn example(path: &Path) -> std::io::Result<()> {
    ///     // Create regular file with default permissions
    ///     let mut f = File::options().mode(0o700).create(true).open(&path)?;
    ///     Ok(())
    /// }
    /// ```
    #[clippy::version = "1.100.0"]
    pub CREATE_REGULAR_FILE_AND_SET_PERMISSIONS,
    suspicious,
    "modifying permissions of a regular file that was created through an earlier function call"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for occurrences where a symlink is being created and then the same
    /// symlink file path is used to set file permissions.
    ///
    /// ### Why is this bad?
    /// Creating a symlink and then modifying permissions on a symlink in two different
    /// syscalls can cause a path-based Time-Of-Check to Time-Of-Use (TOCTOU) race condition.
    /// Between the two calls, another process can delete and replace the symlink with a different
    /// file or replace the target file of the symlink, which could lead to an unintended file
    /// having its permissions changed. This could be dangerous in the case of a malicious file
    /// escalating its privileges from a set permission syscall.
    ///
    /// ### Example
    /// Creating a symlink and setting permissions on that symlink:
    /// ```no_run
    /// use std::path::Path;
    /// use std::fs::Permissions;
    /// use std::os::unix::fs::{PermissionsExt, symlink};
    /// fn example(path: &Path) -> std::io::Result<()> {
    ///     // Symlink path
    ///     let symlink_path = Path::new("/path/to/symlink");
    ///     // Create with default permissions
    ///     symlink(&symlink_path, &path)?;
    ///     // Fix up permissions
    ///     std::fs::set_permissions(&path, Permissions::from_mode(0o700))?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// Creating a symlink and modifying the permissions on that symlink should be
    /// avoided altogether.
    #[clippy::version = "1.100.0"]
    pub CREATE_SYMLINK_AND_SET_PERMISSIONS,
    suspicious,
    "modifying permissions of a directory that was created through an earlier function call"
}

declare_lint_pass!(FileCreationAndSetPermissions => [
    CREATE_DIR_AND_SET_PERMISSIONS,
    CREATE_REGULAR_FILE_AND_SET_PERMISSIONS,
    CREATE_SYMLINK_AND_SET_PERMISSIONS,
]);

/// An enum containing all the different possible file types
/// to lint on
enum FileType {
    RegularFile,
    Directory,
    Symlink,
}

impl FileType {
    /// Match an enum member of `FileType` to a string representing the member
    fn file_type_as_str(&self) -> &str {
        match self {
            FileType::RegularFile => "regular file",
            FileType::Directory => "directory",
            FileType::Symlink => "symlink",
        }
    }

    /// Match an enum member of `FileType` to the lint it should emit
    fn lint_type(&self) -> &'static Lint {
        match self {
            FileType::RegularFile => CREATE_REGULAR_FILE_AND_SET_PERMISSIONS,
            FileType::Directory => CREATE_DIR_AND_SET_PERMISSIONS,
            FileType::Symlink => CREATE_SYMLINK_AND_SET_PERMISSIONS,
        }
    }

    /// Match an enum member of `FileType` to a message on alternate option
    /// or route for a user to take (exception for symlink)
    fn consider_message(&self) -> &'static str {
        match self {
            FileType::RegularFile => {
                "consider using `File::options` alongside `OpenOptionsExt` \
                to create a file with certain permissions in advance"
            },
            FileType::Directory => {
                "on unix, consider using `DirBuilderExt::new().mode()` instead \
                to atomically create a directory and set certain permissions"
            },
            FileType::Symlink => {
                "creating a symlink and modifying the permissions of that symlink should \
                be avoided altogether"
            },
        }
    }
}

impl Display for FileType {
    /// Displays the lint message that describes why the following code
    /// is bad.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file_type = self.file_type_as_str();
        write!(
            f,
            "creating a {file_type} and setting its permissions in two \
            separate steps leaves a brief window where the {file_type} is \
            exposed with world-readable permissions. Another process could \
            replace this file with a different file and cause privilege \
            escalation on the new file"
        )
    }
}

/// Checks if the expression is `std::fs::set_permissions` or `std::fs::set_permissions_nofollow`
fn check_expr_is_set_perm<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Call(func, [path, _]) = e.kind
        && let ExprKind::Path(ref func_qpath) = func.kind
        && let Some(def_id) = cx.qpath_res(func_qpath, func.hir_id).opt_def_id()
        && matches!(
            cx.tcx.get_diagnostic_name(def_id),
            Some(sym::fs_set_permissions | sym::fs_set_permissions_nofollow)
        )
    {
        return Some((func, path));
    }
    None
}

/// Checks if the expression is `std::fs::create_dir` or `std::fs::create_dir_all`
fn check_expr_is_create_dir<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, FileType)> {
    if let ExprKind::Call(func, [path]) = e.kind
        && let ExprKind::Path(ref func_qpath) = func.kind
        && let Some(def_id) = cx.qpath_res(func_qpath, func.hir_id).opt_def_id()
        && matches!(
            cx.tcx.get_diagnostic_name(def_id),
            Some(sym::fs_create_dir | sym::fs_create_dir_all)
        )
    {
        return Some((path, FileType::Directory));
    }
    None
}

/// Checks if the expression is `File::create*`
fn check_expr_is_file_create<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, FileType)> {
    if let ExprKind::Call(func, [path]) = e.kind
        && let ExprKind::Path(QPath::TypeRelative(ty, segment)) = func.kind
        && ty.basic_res().is_diag_item(&cx.tcx(), sym::File)
        && matches!(segment.ident.name, sym::create | sym::create_new | sym::create_buffered)
    {
        return Some((path, FileType::RegularFile));
    }
    None
}

/// Checks if the expression is `symlink*`.
fn check_expr_is_symlink<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, FileType)> {
    if let ExprKind::Call(func, [original, _link]) = e.kind
        && let ExprKind::Path(ref func_qpath) = func.kind
        && let Some(def_id) = cx.qpath_res(func_qpath, func.hir_id).opt_def_id()
        && matches!(
            cx.tcx.get_diagnostic_name(def_id),
            Some(sym::fs_symlink | sym::fs_symlink_dir | sym::fs_symlink_file | sym::fs_symlink_path)
        )
    {
        return Some((original, FileType::Symlink));
    }
    None
}

/// Finds the earliest `std::fs::create_dir*`, `File::create*`, `symlink*`
/// expression matching `set_perm_path` prior to `set_perm_expr`.
///
/// Returns the creation expression and the [`FileType`] created.
fn check_file_was_created_earlier<'tcx>(
    cx: &LateContext<'tcx>,
    set_perm_expr: &'tcx Expr<'tcx>,
    set_perm_func: &'tcx Expr<'tcx>,
    set_perm_path: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, FileType)> {
    let parent_block = get_enclosing_block(cx, set_perm_expr.hir_id)?;

    for_each_expr(cx.tcx(), parent_block, |e| {
        if let Some((func, path)) = check_expr_is_set_perm(cx, e)
            && eq_expr_value(cx, e.span.ctxt(), func, set_perm_func)
            && eq_expr_value(cx, e.span.ctxt(), path, set_perm_path)
        {
            return ControlFlow::Break(None);
        } else if let Some((path, filetype)) = check_expr_is_create_dir(cx, e)
            .or(check_expr_is_file_create(cx, e))
            .or(check_expr_is_symlink(cx, e))
            && eq_expr_value(cx, e.span.ctxt(), path, set_perm_path)
        {
            return ControlFlow::Break(Some((e, filetype)));
        }
        ControlFlow::Continue(())
    })
    .unwrap_or(None)
}

impl<'tcx> LateLintPass<'tcx> for FileCreationAndSetPermissions {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let Some((func, path)) = check_expr_is_set_perm(cx, expr)
            && let Some((create_file_expr, filetype)) = check_file_was_created_earlier(cx, expr, func, path)
        {
            let (warning_msg, consider_msg) = (format!("{filetype}"), filetype.consider_message());

            span_lint_and_then(cx, filetype.lint_type(), expr.span, warning_msg, |diag| {
                diag.span_note(create_file_expr.span, "create file operation occurred here");
                diag.span_note(expr.span, "the set permission operation occurred here");
                diag.help(consider_msg);
            });
        }
    }
}
