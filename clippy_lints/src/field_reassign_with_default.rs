use crate::utils::span_lint_and_note;
use if_chain::if_chain;
use rustc_ast_pretty::pprust::{expr_to_string, ty_to_string};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::Symbol;
use syntax::ast::{BindingMode, Block, ExprKind, Mutability, PatKind, StmtKind};

declare_clippy_lint! {
    /// **What it does:** Checks for immediate reassignment of fields initialized
    /// with Default::default().
    ///
    /// **Why is this bad?** Fields should be set using
    /// T { field: value, ..Default::default() } syntax instead of using a mutable binding.
    ///
    /// **Known problems:** The lint does not detect calls to Default::default()
    /// if they are made via another struct implementing the Default trait. This
    /// may be corrected with a LateLintPass. If type inference stops requiring
    /// an explicit type for assignment using Default::default() this lint will
    /// not trigger for cases where the type is elided. This may also be corrected
    /// with a LateLintPass.
    ///
    /// **Example:**
    /// ```ignore
    /// // Bad
    /// let mut a: A = Default::default();
    /// a.i = 42;
    ///
    /// // Good
    /// let a = A {
    ///     i: 42,
    ///     .. Default::default()
    /// };
    /// ```
    pub FIELD_REASSIGN_WITH_DEFAULT,
    pedantic,
    "binding initialized with Default should have its fields set in the initializer"
}

declare_lint_pass!(FieldReassignWithDefault => [FIELD_REASSIGN_WITH_DEFAULT]);

impl EarlyLintPass for FieldReassignWithDefault {
    fn check_block(&mut self, cx: &EarlyContext<'_>, block: &Block) {
        // store statement index and name of binding for all statements like
        // `let mut <binding> = Default::default();`
        let binding_statements_using_default: Vec<(usize, Symbol)> = block
            .stmts
            .iter()
            .enumerate()
            .filter_map(|(idx, stmt)| {
                if_chain! {
                    // only take `let ...`
                    if let StmtKind::Local(ref local) = stmt.kind;
                    // only take `... mut <binding> ...`
                    if let PatKind::Ident(BindingMode::ByValue(Mutability::Mut), binding, _) = local.pat.kind;
                    // only when assigning `... = Default::default()`
                    if let Some(ref expr) = local.init;
                    if let ExprKind::Call(ref fn_expr, _) = &expr.kind;
                    if let ExprKind::Path(_, path) = &fn_expr.kind;
                    if path.segments.len() >= 2;
                    if path.segments[path.segments.len()-2].ident.as_str() == "Default";
                    if path.segments.last().unwrap().ident.as_str() == "default";
                    then {
                        Some((idx, binding.name))
                    }
                    else {
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        // look at all the following statements for the binding statements and see if they reassign
        // the fields of the binding
        for (stmt_idx, binding_name) in binding_statements_using_default {
            // last statement of block cannot trigger the lint
            if stmt_idx == block.stmts.len() - 1 {
                break;
            }

            // find "later statement"'s where the fields of the binding set by Default::default()
            // get reassigned
            let later_stmt = &block.stmts[stmt_idx + 1];
            if_chain! {
                // only take assignments
                if let StmtKind::Semi(ref later_expr) = later_stmt.kind;
                if let ExprKind::Assign(ref later_lhs, ref later_rhs, _) = later_expr.kind;
                // only take assignments to fields where the field refers to the same binding as the previous statement
                if let ExprKind::Field(ref binding_name_candidate, later_field_ident) = later_lhs.kind;
                if let ExprKind::Path(_, path) = &binding_name_candidate.kind;
                if let Some(second_binding_name) = path.segments.last();
                if second_binding_name.ident.name == binding_name;
                then {
                    // take the preceding statement as span
                    let stmt = &block.stmts[stmt_idx];
                    if let StmtKind::Local(preceding_local) = &stmt.kind {
                        // reorganize the latter assigment statement into `field` and `value` for the lint
                        let field = later_field_ident.name.as_str();
                        let value = expr_to_string(later_rhs);
                        if let Some(ty) = &preceding_local.ty {
                            let ty = ty_to_string(ty);

                            span_lint_and_note(
                                cx,
                                FIELD_REASSIGN_WITH_DEFAULT,
                                later_stmt.span,
                                "field assignment outside of initializer for an instance created with Default::default()",
                                preceding_local.span,
                                &format!("consider initializing the variable immutably with `{} {{ {}: {}, ..Default::default() }}`", ty, field, value),
                            );
                        }
                    }
                }
            }
        }
    }
}
