use crate::rustc_lint::LintContext;
use crate::rustc_span::Pos;
use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{Pat, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for variable that exceeds a configurable number characters.
    ///
    /// ### Why is this bad?
    /// Long variable names can make code harder to read and thus to maintain.
    ///
    /// ### Example
    /// ```no_run
    /// let ferris_fixes_more_bugs_than_your_entire_devops_team_does = "content of a string";
    /// ```
    /// Use instead:
    /// ```no_run
    /// let ferris_fixes_more_bugs = "content of a string";
    /// ```
    #[clippy::version = "1.88.0"]
    pub LONG_VARIABLE_NAMES,
    style,
    "usage of a long variable"
}
pub struct LongVariableNames {
    pub max_variable_name_length: u32,
}

impl LongVariableNames {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            max_variable_name_length: conf.max_variable_name_length,
        }
    }
}

impl_lint_pass!(LongVariableNames => [LONG_VARIABLE_NAMES]);

impl<'tcx> LateLintPass<'tcx> for LongVariableNames {
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx Pat<'_>) {
        if let PatKind::Binding(.., ident, _) = pat.kind {
            let length_bytes = if ident.span.from_expansion() {
                // since the span can't be calculated, we can't lint this
                // so we just return
                return;
            } else {
                let higher_bound = cx.sess().source_map().lookup_char_pos(ident.span.hi()).col;
                let lower_bound = cx.sess().source_map().lookup_char_pos(ident.span.lo()).col;
                (higher_bound - lower_bound).to_u32() + 1
            };
            if length_bytes > self.max_variable_name_length {
                let variable_name_length = u32::try_from(ident.name.to_ident_string().chars().count())
                    .expect("the variable name length exceeds u32::MAX");
                if variable_name_length > self.max_variable_name_length {
                    let length_diff = variable_name_length - self.max_variable_name_length;

                    span_lint_and_help(
                        cx,
                        LONG_VARIABLE_NAMES,
                        ident.span,
                        format!(
                            "variable name is longer than the configured `max-variable-name-length` of ({} characters)",
                            self.max_variable_name_length
                        ),
                        None,
                        format!("reduce the length of the variable name with at least {length_diff} characters"),
                    );
                }
            }
        }
    }
}
