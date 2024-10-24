use std::borrow::Cow;
use std::collections::BTreeMap;

use clippy_config::Conf;
use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use clippy_utils::path_def_id;
use clippy_utils::ty::get_inherent_method;
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects calls to `std::process::Command::new` that can easily be replaced with Rust code.
    ///
    /// ### Why is this bad?
    ///
    /// "Shelling out" is slow, non-portable, and generally unnecessary (especially to these programs).
    ///
    /// ### Example
    /// ```no_run
    /// use std::io;
    /// use std::process::Command;
    ///
    /// fn list_files() -> io::Result<Vec<String>> {
    ///     let output = Command::new("ls").output()?;
    ///     if !output.status.success() {
    ///         return Err(io::Error::new(
    ///             io::ErrorKind::Other,
    ///             String::from_utf8_lossy(&output.stderr)
    ///         ));
    ///     }
    ///
    ///     let stdout = std::str::from_utf8(&output.stdout).expect("should be UTF-8 output");
    ///     Ok(stdout.split_whitespace().map(String::from).collect())
    /// }
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn list_files() -> std::io::Result<Vec<String>> {
    ///     let mut buf = Vec::new();
    ///     for entry_res in std::fs::read_dir(".")? {
    ///         let path = entry_res?.path();
    ///         let os_name = path.into_os_string();
    ///         buf.push(os_name.into_string().expect("should be UTF-8 paths"))
    ///     }
    ///
    ///     Ok(buf)
    /// }
    /// ```
    #[clippy::version = "1.84.0"]
    pub UNNECESSARY_SHELL_COMMAND,
    pedantic,
    "using the simple shell utilities instead of Rust code"
}

pub struct UnnecessaryShellCommand {
    std_process_command_new: Option<DefId>,
    unnecessary_commands: &'static BTreeMap<Box<str>, Option<Cow<'static, str>>>,
}

impl UnnecessaryShellCommand {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            std_process_command_new: None,
            unnecessary_commands: &conf.unnecessary_commands,
        }
    }
}

impl_lint_pass!(UnnecessaryShellCommand => [UNNECESSARY_SHELL_COMMAND]);

impl LateLintPass<'_> for UnnecessaryShellCommand {
    fn check_crate(&mut self, cx: &LateContext<'_>) {
        if let Some(command_did) = cx.tcx.get_diagnostic_item(sym::Command)
            && let Some(fn_item) = get_inherent_method(cx, command_did, sym::new)
        {
            self.std_process_command_new = Some(fn_item.def_id);
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let Some(std_process_command_new) = self.std_process_command_new else {
            return;
        };

        if let ExprKind::Call(func, [command]) = expr.kind
            && path_def_id(cx, func) == Some(std_process_command_new)
            && let Some(Constant::Str(command_lit)) = ConstEvalCtxt::new(cx).eval(command)
            && let command_lit = command_lit.strip_suffix(".exe").unwrap_or(&command_lit)
            && let Some(help) = self.unnecessary_commands.get(command_lit)
        {
            let lint = UNNECESSARY_SHELL_COMMAND;
            let msg = "unnecessarily shelling out for trivial operation";
            if let Some(help) = help.as_deref() {
                span_lint_and_help(cx, lint, command.span, msg, None, help);
            } else {
                span_lint(cx, lint, command.span, msg);
            }
        }
    }
}
