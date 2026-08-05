use clippy_utils::diagnostics::span_lint_hir;
use clippy_utils::source::SpanExt as _;
use rustc_hir::{Block, ItemKind, StmtKind};
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_session::declare_lint_pass;
use rustc_span::{Pos as _, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for items declared after some statement in a block.
    ///
    /// ### Why is this bad?
    /// Items live for the entire scope they are declared
    /// in. But statements are processed in order. This might cause confusion as
    /// it's hard to figure out which item is meant in a statement.
    ///
    /// ### Example
    /// ```no_run
    /// fn foo() {
    ///     println!("cake");
    /// }
    ///
    /// fn main() {
    ///     foo(); // prints "foo"
    ///     fn foo() {
    ///         println!("foo");
    ///     }
    ///     foo(); // prints "foo"
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn foo() {
    ///     println!("cake");
    /// }
    ///
    /// fn main() {
    ///     fn foo() {
    ///         println!("foo");
    ///     }
    ///     foo(); // prints "foo"
    ///     foo(); // prints "foo"
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub ITEMS_AFTER_STATEMENTS,
    pedantic,
    "blocks where an item comes after a statement"
}

declare_lint_pass!(ItemsAfterStatements => [ITEMS_AFTER_STATEMENTS]);

/// Checks that each item is a direct child of `block`, rather than being nested inside a token tree
/// such as a `cfg_select!` arm.
///
/// `cfg_select!` splices the tokens of the selected arm into the enclosing block without applying
/// any expansion marker, so both the span and the syntax context of the resulting items are
/// indistinguishable from items written directly in the block.
///
/// This deliberately gives up on a few true positives, e.g. an item after a statement within the
/// same `cfg_select!` arm, or `some_macro! { fn f() {} }` in statement position, as a false
/// negative is preferable to a false positive here.
fn are_direct_children_of_block(cx: &LateContext<'_>, block: &Block<'_>, item_spans: &[Span]) -> Vec<bool> {
    let mut direct_children = vec![false; item_spans.len()];
    if item_spans.is_empty() {
        return direct_children;
    }

    // Only the block's own opening delimiter may be left open at an item's position. Comments and
    // string literals are single tokens, so delimiters inside them can't skew the count.
    block.span.with_source_text(cx, |src| {
        let mut item_offsets = item_spans
            .iter()
            .enumerate()
            .filter_map(|(index, item_span)| {
                (item_span.lo() >= block.span.lo() && item_span.lo() <= block.span.hi())
                    .then(|| (index, (item_span.lo() - block.span.lo()).to_usize()))
            })
            .filter(|&(_, offset)| offset <= src.len())
            .collect::<Vec<_>>();
        item_offsets.sort_unstable_by_key(|&(_, offset)| offset);

        let mut depth = 0i32;
        let mut offset = 0;
        let mut next_item = 0;
        for token in tokenize(src, FrontmatterAllowed::No) {
            while let Some(&(index, item_offset)) = item_offsets.get(next_item)
                && item_offset <= offset
            {
                direct_children[index] = depth == 1;
                next_item += 1;
            }

            match token.kind {
                TokenKind::OpenParen | TokenKind::OpenBrace | TokenKind::OpenBracket => depth += 1,
                TokenKind::CloseParen | TokenKind::CloseBrace | TokenKind::CloseBracket => depth -= 1,
                _ => {},
            }
            offset += token.len as usize;
        }

        while let Some(&(index, _)) = item_offsets.get(next_item) {
            direct_children[index] = depth == 1;
            next_item += 1;
        }
    });

    // The source isn't available, or the range spans multiple files (e.g. `include!`).
    direct_children
}

impl LateLintPass<'_> for ItemsAfterStatements {
    fn check_block(&mut self, cx: &LateContext<'_>, block: &Block<'_>) {
        if block.stmts.len() > 1 {
            let ctxt = block.span.ctxt();
            let mut in_external = None;
            let items = block
                .stmts
                .iter()
                .skip_while(|stmt| matches!(stmt.kind, StmtKind::Item(..)))
                .filter_map(|stmt| match stmt.kind {
                    StmtKind::Item(id) => Some(cx.tcx.hir_item(id)),
                    _ => None,
                })
                // Ignore macros since they can only see previously defined locals.
                .filter(|item| !matches!(item.kind, ItemKind::Macro(..)))
                // Stop linting if macros define items.
                .take_while(|item| item.span.ctxt() == ctxt)
                .collect::<Vec<_>>();
            let item_spans = items.iter().map(|item| item.span).collect::<Vec<_>>();
            let direct_children = are_direct_children_of_block(cx, block, &item_spans);
            items
                .into_iter()
                .zip(direct_children)
                .filter(|(_, direct)| *direct)
                .for_each(|(item, _)| {
                    // Only do the macro check once, but delay it until it's needed.
                    if !*in_external.get_or_insert_with(|| block.span.in_external_macro(cx.sess().source_map())) {
                        span_lint_hir(
                            cx,
                            ITEMS_AFTER_STATEMENTS,
                            item.hir_id(),
                            item.span,
                            "adding items after statements is confusing, since items exist from the \
                                start of the scope",
                        );
                    }
                });
        }
    }
}
