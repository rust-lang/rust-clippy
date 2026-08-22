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

/// `cfg_select!` splices the tokens of the selected arm into the enclosing block without applying
/// any expansion marker, so both the span and the syntax context of the resulting items are
/// indistinguishable from items written directly in the block.
fn are_direct_children_of_block(cx: &LateContext<'_>, block: &Block<'_>, stmt_spans: &[Span]) -> Vec<bool> {
    if stmt_spans.is_empty() {
        return vec![];
    }
    let mut direct_children = vec![true; stmt_spans.len()];

    // Only the block's own opening delimiter may be left open at a statement's position. Comments and
    // string literals are single tokens, so delimiters inside them can't skew the count.
    block.span.with_source_text(cx, |src| {
        let Some(stmt_offsets) = stmt_spans
            .iter()
            .map(|stmt_span| {
                (stmt_span.lo() >= block.span.lo() && stmt_span.lo() <= block.span.hi())
                    .then(|| (stmt_span.lo() - block.span.lo()).to_usize())
                    .filter(|&offset| offset <= src.len())
            })
            .collect::<Option<Vec<_>>>()
        else {
            return;
        };
        if !stmt_offsets.is_sorted() {
            return;
        }

        let mut depth = 0i32;
        let mut offset = 0;
        let mut next_item = 0;
        for token in tokenize(src, FrontmatterAllowed::No) {
            while let Some(&stmt_offset) = stmt_offsets.get(next_item)
                && stmt_offset <= offset
            {
                direct_children[next_item] = depth == 1;
                next_item += 1;
            }

            match token.kind {
                TokenKind::OpenParen | TokenKind::OpenBrace | TokenKind::OpenBracket => depth += 1,
                TokenKind::CloseParen | TokenKind::CloseBrace | TokenKind::CloseBracket => depth -= 1,
                _ => {},
            }
            offset += token.len as usize;
        }

        while stmt_offsets.get(next_item).is_some() {
            direct_children[next_item] = depth == 1;
            next_item += 1;
        }
    });

    // If source text isn't available or the spans can't be mapped, preserve the lint.
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
                    StmtKind::Item(id) => Some((stmt.span, cx.tcx.hir_item(id))),
                    _ => None,
                })
                // Ignore macros since they can only see previously defined locals.
                .filter(|(_, item)| !matches!(item.kind, ItemKind::Macro(..)))
                // Stop linting if macros define items.
                .take_while(|(_, item)| item.span.ctxt() == ctxt)
                .collect::<Vec<_>>();
            let stmt_spans = items.iter().map(|(stmt_span, _)| *stmt_span).collect::<Vec<_>>();
            let direct_children = are_direct_children_of_block(cx, block, &stmt_spans);
            items
                .into_iter()
                .zip(direct_children)
                .filter(|(_, direct)| *direct)
                .for_each(|((_, item), _)| {
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
