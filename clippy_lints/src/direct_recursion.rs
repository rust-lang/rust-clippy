use clippy_utils::diagnostics::span_lint;
use clippy_utils::sym;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Attribute, Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions that call themselves from their body.
    ///
    /// ### Why restrict this?
    /// In Safety Critical contexts, recursive calls can lead to catastrophic
    /// crashes if they happen to overflow the stack.
    ///
    /// In most contexts, this is not an issue, so this lint is allow-by-default.
    ///
    /// ### Notes
    ///
    /// #### Triggers
    /// There are two things that trigger this lint:
    /// - Function calls from a function (or method) to itself,
    /// - Function pointer bindings from a function (or method) to itself.
    ///
    /// #### Independent of control flow
    /// This lint triggers whenever the conditions above are met, regardless of
    /// control flow and other such constructs.
    ///
    /// #### Blessing a recursive call
    /// The user may choose to bless a recursive call or binding using the
    /// attribute #[clippy::allowed_recursion]
    ///
    /// #### Indirect calls
    /// This lint **does not** detect indirect recursive calls.
    ///
    /// ### Examples
    /// This function will trigger the lint:
    /// ```no_run
    /// fn i_call_myself_in_a_bounded_way(bound: u8) {
    ///     if bound > 0 {
    ///         // This line will trigger the lint
    ///         i_call_myself_in_a_bounded_way(bound - 1);
    ///     }
    /// }
    /// ```
    /// Using #[clippy::allowed_recursion] lets it pass:
    /// ```no_run
    /// fn i_call_myself_in_a_bounded_way(bound: u8) {
    ///     if bound > 0 {
    ///         #[clippy::allowed_recursion]
    ///         i_call_myself_in_a_bounded_way(bound - 1);
    ///     }
    /// }
    /// ```
    /// This triggers the lint when `fibo` is bound to a function pointer
    /// inside `fibo`'s body
    /// ```no_run
    /// fn fibo(a: u32) -> u32 {
    ///     if a < 2 { a } else { (a - 2..a).map(fibo).sum() }
    /// }
    /// ```
    #[clippy::version = "1.89.0"]
    pub DIRECT_RECURSION,
    restriction,
    "functions shall not call themselves directly"
}
declare_lint_pass!(DirectRecursion => [DIRECT_RECURSION]);

impl<'tcx> LateLintPass<'tcx> for DirectRecursion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // Before running the lint, we look up the attributes of this Expr.
        // If it has been marked with `clippy::allowed_recursion`, then
        // we ignore it - someone has decided that its recursive calls are fine.
        let attrs = cx.tcx.hir_attrs(expr.hir_id);

        for attr in attrs {
            if let Attribute::Unparsed(attr_item) = attr {
                for segment in &attr_item.path.segments {
                    if segment.name == sym::allowed_recursion {
                        return;
                    }
                }
            }
        }

        for paren_id in cx.tcx.hir_parent_id_iter(expr.hir_id) {
            let attrs = cx.tcx.hir_attrs(paren_id);

            for attr in attrs {
                if let Attribute::Unparsed(attr_item) = attr {
                    for segment in &attr_item.path.segments {
                        if segment.name == sym::allowed_recursion {
                            return;
                        }
                    }
                }
            }
        }

        // A Path in this context could be a path to a function or a method.
        // If it is a Path, we check if it's a Resolved path, which in this case
        // is a definition that we can already fully see, and therefore we can
        // ascertain whether or not it is a recursive call.
        if let ExprKind::Path(QPath::Resolved(_, path)) = expr.kind
            // Now we know that this Path is fully resolved. We now
            // must check if it points to a function or a method's definition.
            && let Res::Def(DefKind::Fn | DefKind::AssocFn, fn_path_id) = path.res
        {
            // 1) Now we know that the path we've found is of a function or method definition.
            // We will now check if it corresponds to the path of a function we're inside
            // of.
            //
            // 2) Since we're interested in checking for not only the function that this Path
            // occurs in, but also any function inside of which THAT function itself might
            // be nested in, we walk upwards from where we are in the AST.
            // That's what `hir_parent_iter` does!
            //
            // 3) If one of those nodes has an associated body, then we're contained in that body
            // (they are our parent, after all).
            //
            // 4) If that body's owner then, corresponds to the same path we're using in our
            // `expr`, then we know that we have a recursive loop.
            for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
                if let Some((def_id, _)) = node.associated_body()
                    && fn_path_id == def_id.into()
                {
                    span_lint(
                        cx,
                        DIRECT_RECURSION,
                        expr.span,
                        "this function contains a call to itself",
                    );
                    break;
                }
            }
        }
    }
}
