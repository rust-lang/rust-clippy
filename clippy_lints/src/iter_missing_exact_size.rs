use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::paths::EXACT_SIZE_ITERATOR;
use clippy_utils::sym;
use clippy_utils::ty::{get_field_by_name, implements_trait, ty_from_hir_ty};
use rustc_hir::{
    Body, Expr, ExprKind, Impl, ImplItem, ImplItemImplKind, ImplItemKind, ItemKind, OwnerNode, QPath, StmtKind,
};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_span::symbol::kw;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for iterators where the size hint wraps around an iterator that
    /// implements `ExactSizeIterator` but do not themselves implement
    /// `ExactSizeIterator`.
    ///
    /// ### Why is this bad?
    ///
    /// When the size of an iterator is based on some other iterator that is
    /// known to have an exact size, the wrapping iterator may also have an
    /// exact size and should be marked as such.
    ///
    /// ### Known issues
    ///
    /// * The lint only checks the `size_hint()` method, which may not match
    /// what the iterator actually does.
    /// * Marking the iterator as implementing `ExactSizeIterator` may have
    /// backwards compatibility implications for exported types.
    ///
    /// ### Example
    /// ```no_run
    /// struct StringRepeater {
    ///     original: String,
    ///     range: std::ops::Range<usize>,
    /// }
    ///
    /// impl Iterator for StringRepeater {
    ///     type Item = String;
    ///     fn next(&mut self) -> Option<Self::Item> {
    ///         self.range.next().map(|i| self.original.repeat(i) )
    ///     }
    ///     fn size_hint(&self) -> (usize, Option<usize>) {
    ///         self.range.size_hint()
    ///     }
    /// }
    ///
    /// let repeater = StringRepeater { original: "Foo".to_string(), range: 1..5 };
    /// for value in repeater {
    ///     println!("{value}");
    /// }
    ///
    /// ```
    /// Use instead:
    ///
    /// ```no_run
    /// struct StringRepeater {
    ///     original: String,
    ///     range: std::ops::Range<usize>,
    /// }
    ///
    /// impl Iterator for StringRepeater {
    ///     type Item = String;
    ///     fn next(&mut self) -> Option<Self::Item> {
    ///         self.range.next().map(|i| self.original.repeat(i) )
    ///     }
    ///     fn size_hint(&self) -> (usize, Option<usize>) {
    ///         self.range.size_hint()
    ///     }
    /// }
    ///
    /// impl ExactSizeIterator for StringRepeater {}
    ///
    /// let repeater = StringRepeater { original: "Foo".to_string(), range: 1..5 };
    /// for value in repeater {
    ///     println!("{value}");
    /// }
    /// ```
    #[clippy::version = "1.99.0"]
    pub ITER_MISSING_EXACT_SIZE,
    pedantic,
    "iterator delegates to an ExactSizeIterator for its size hint but does not itself implement ExactSizeIterator"
}

declare_lint_pass!(IterMissingExactSize => [ITER_MISSING_EXACT_SIZE]);

/// Given a `Body` for the `size_hint()` function, try to get the return
/// expression, either
/// - a trailing expression when the block has no statements
/// - the singular `return` statement in a block with exactly one statement (the return) (and
///   optionally a dead-code trailing expression that we can ignore)
fn size_hint_return<'tcx>(body: &'tcx Body<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Block(block, None) = body.value.kind else {
        // Function body isn't a block?
        return None;
    };
    // Block without statements - either it has a trailing expression (and we
    // return that) or it doesn't (and we return none).
    if block.stmts.is_empty() {
        return block.expr;
    }
    if let [only_statement] = block.stmts
        && let StmtKind::Semi(statement_semi) = only_statement.kind
        && let ExprKind::Ret(returned_value) = statement_semi.kind
    {
        returned_value
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for IterMissingExactSize {
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, impl_item: &'tcx ImplItem<'tcx>) {
        // Check for this item being the size_hint() function in the
        // implementation of the iterator trait.
        if impl_item.ident.name == sym::size_hint
            && matches!(impl_item.impl_kind, ImplItemImplKind::Trait { .. })
            && let OwnerNode::Item(item) = cx
                .tcx
                .expect_hir_owner_node(cx.tcx.local_parent(impl_item.owner_id.def_id))
            && let ItemKind::Impl(Impl {
                    of_trait: Some(of_trait),
                    self_ty: current_type,
                    ..
                }) = item.kind
            && let Some(trait_id) = of_trait.trait_ref.trait_def_id()
            && cx.tcx.is_diagnostic_item(sym::Iterator, trait_id)
            // Get the body ID and convert it to the actual body
            && let ImplItemKind::Fn(_, body_id) = impl_item.kind
            && let size_hint_body = cx.tcx.hir_body(body_id)
            && let Some(size_hint_return) = size_hint_return(size_hint_body)
            // the size_hint() function just returns - check if it returns the
            // result of `self.{field}.size_hint()`
            && let ExprKind::MethodCall(method_name, receiver, [], _) = size_hint_return.kind
            && method_name.ident.name == sym::size_hint
            && let ExprKind::Field(object, field_name) = receiver.kind
            && let ExprKind::Path(QPath::Resolved(_, object_path)) = object.kind
            && let [path_segment] = object_path.segments
            && path_segment.ident.name == kw::SelfLower
            // Extract the `{field}` from `self.{field}.size_hint()`
            && let current_middle_ty = ty_from_hir_ty(cx, current_type)
            && let Some(field) = get_field_by_name(cx.tcx, current_middle_ty, field_name.name)
            // Make sure a ExactSizeIterator type is known, if not this is a no_core
            // environment
            && let Some(trait_def) = EXACT_SIZE_ITERATOR.only(cx)
            // Field type needs to implement ExactSizeIterator
            && implements_trait(cx, field, trait_def, &[])
            // Overall type needs to not already implement ExactSizeIterator
            && !implements_trait(cx, current_middle_ty, trait_def, &[])
        {
            span_lint_and_help(
                cx,
                ITER_MISSING_EXACT_SIZE,
                item.span,
                "iterator can implement `ExactSizeIterator`",
                Some(size_hint_return.span),
                "this `size_hint()` implementation delegates to to the `size_hint()` of an `ExactSizeIterator`, so the overall iterator is likely to have an exact size",
            );
        }
    }
}
