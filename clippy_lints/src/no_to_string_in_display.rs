use crate::utils::{match_def_path, paths, span_lint};
use if_chain::if_chain;
use rustc_hir::{def, Expr, ExprKind, FnDecl, FnSig, ImplItem, ImplItemKind, MutTy, Node, Path, QPath, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:** Checks for uses of `to_string()` when implementing
    /// `Display` traits.
    ///
    /// **Why is this bad?** Usually `to_string` is implemented indirectly
    /// via `Display`. Hence using it while implementing `Display` would
    /// lead to infinite recursion.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// use std::fmt;
    ///
    /// struct Structure(i32);
    /// impl fmt::Display for Structure {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "{}", self.to_string())
    ///     }
    /// }
    ///
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::fmt;
    ///
    /// struct Structure(i32);
    /// impl fmt::Display for Structure {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "{}", self.0)
    ///     }
    /// }
    /// ```
    pub NO_TO_STRING_IN_DISPLAY,
    correctness,
    "to_string method used while implementing Display trait"
}

declare_lint_pass!(NoToStringInDisplay => [NO_TO_STRING_IN_DISPLAY]);

impl LateLintPass<'_, '_> for NoToStringInDisplay {
    fn check_expr(&mut self, cx: &LateContext<'_, '_>, expr: &Expr<'_>) {
        let parent_id = cx.tcx.hir().get_parent_item(expr.hir_id);
        let parent_node = cx.tcx.hir().find(parent_id);

        if_chain! {
            if let ExprKind::MethodCall(ref path, _, _) = expr.kind;
            if path.ident.as_str() == "to_string";
            if let Some(Node::ImplItem(ImplItem {ident, kind, ..})) = parent_node;
            if ident.as_str() == "fmt";
            if let ImplItemKind::Fn(FnSig {decl, ..}, _) = kind;
            if let FnDecl{inputs, ..} = decl;
            if let Ty {kind, ..} = &inputs[0];
            if let TyKind::Rptr(_, MutTy {ty, ..}) = kind;
            if let Ty {kind, ..} = ty;
            if let TyKind::Path(QPath::Resolved(_, path)) = kind;
            if let Path{res, ..} = path;
            if let def::Res::SelfTy(Some(did), _) = res;
            if match_def_path(cx, *did, &paths::DISPLAY_TRAIT);
            then {
                span_lint(
                    cx,
                    NO_TO_STRING_IN_DISPLAY,
                    expr.span,
                    "Using to_string in fmt::Display implementation might lead to infinite recursion",
                );
            }
        }
    }
}
