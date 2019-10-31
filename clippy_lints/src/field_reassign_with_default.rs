use rustc::lint::{EarlyLintPass, LintArray, LintPass};
use rustc::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    pub FIELD_REASSIGN_WITH_DEFAULT,
    pedantic,
    "instance initialized with Default should have its fields set in the initializer"
}

declare_lint_pass!(FieldReassignWithDefault => [FIELD_REASSIGN_WITH_DEFAULT]);

impl EarlyLintPass for FieldReassignWithDefault {}
