use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_note, span_lint_and_sugg};
use itertools::Itertools;
use rustc_ast::ast;
use rustc_ast::ast::{AttrKind, Attribute, Block, Item};
use rustc_ast::AttrKind::DocComment;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::BytePos;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for empty documentation
    ///
    /// ### Why is this bad?
    /// It is unlikely there is any reason to have empty documentation for an entity
    ///
    /// ### Example
    /// ```rust
    /// ///
    /// fn returns_true() {
    ///     true
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// fn returns_true() {
    ///     true
    /// }
    /// ```
    #[clippy::version = "1.68.0"]
    pub EMPTY_DOCS,
    suspicious,
    "docstrings exist but documentation is empty"
}
declare_lint_pass!(EmptyDocs => [EMPTY_DOCS]);

const MAX_CONTEXT_LEN: BytePos = BytePos(50);

impl EarlyLintPass for EmptyDocs {
    fn check_item(&mut self, ex: &EarlyContext<'_>, item: &Item) {
        //dbg!(&item);
        if item.span.from_expansion() {
            return;
        }
        self.process_attributes(ex, item.span, &item.attrs.to_vec());
        self.process_into_item(ex, &item);
    }
}

impl EmptyDocs {
    fn process_into_item(self, ex: &EarlyContext<'_>, item: &Item) {
        match &item.kind {
            ast::ItemKind::Struct(ast::VariantData::Struct(field_list, _), _) => {
                for field_def in field_list {
                    self.process_attributes(ex, field_def.span, &field_def.attrs.to_vec());
                }
            },
            ast::ItemKind::Union(ast::VariantData::Struct(field_list, _), _) => {
                for field_def in field_list {
                    self.process_attributes(ex, field_def.span, &field_def.attrs.to_vec());
                }
            },
            ast::ItemKind::Enum(enum_def, _) => {
                for variant in &enum_def.variants {
                    self.process_attributes(ex, variant.span, &variant.attrs.to_vec());
                }
            },
            ast::ItemKind::Fn(fn_def) => {
                if let Some(block) = &fn_def.body {
                    self.process_into_block(ex, &block);
                }
            },
            _ => {},
        }
    }

    fn process_into_block(self, ex: &EarlyContext<'_>, block: &Block) {
        for statement in &block.stmts {
            let statement_span = statement.span;
            match &statement.kind {
                ast::StmtKind::Local(local) => {
                    self.process_attributes(ex, statement_span, &local.attrs.to_vec());
                },
                _ => {},
            }
        }
    }

    fn process_attributes(self, ex: &EarlyContext<'_>, parent_span: Span, attributes: &Vec<Attribute>) {
        for (is_doc_comment, doc_string_group) in &attributes.iter().group_by(|a| match a.kind {
            DocComment(..) => true,
            _ => false,
        }) {
            let doc_string_group = doc_string_group.collect::<Vec<_>>();
            if is_doc_comment {
                let empty_attributes = &doc_string_group
                    .iter()
                    .filter(|a| match a.kind {
                        DocComment(_, comment_text) => comment_text.as_str().trim().is_empty(),
                        _ => {
                            panic!("should always be able to unwrap DocComment")
                        },
                    })
                    .collect::<Vec<_>>();
                if empty_attributes.len() == doc_string_group.len() {
                    if !empty_attributes.iter().any(|a| a.span.from_expansion()) {
                        let lo_span = empty_attributes
                            .get(0)
                            .expect("should always have an element")
                            .span
                            .data();
                        let hi_help_span = empty_attributes
                            .get(empty_attributes.len() - 1)
                            .expect("should always have an element")
                            .span
                            .data();
                        let hi_span_adjusted = std::cmp::min(parent_span.data().hi, hi_help_span.hi + MAX_CONTEXT_LEN);
                        let span = Span::new(lo_span.lo, hi_span_adjusted, lo_span.ctxt, lo_span.parent);
                        let help_span = Span::new(lo_span.lo, hi_help_span.hi, lo_span.ctxt, lo_span.parent);
                        span_lint_and_help(
                            ex,
                            EMPTY_DOCS,
                            span,
                            "documentation is empty",
                            Some(help_span),
                            "consider adding documentation or removing the docstring(s)",
                        );
                    }
                }
            }
        }
    }
}
