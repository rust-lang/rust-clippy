use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{AttrId, AttrKind, AttrStyle, Attribute, SyntheticAttr};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Ensures that doc comments and attributes appear before other attributes.
    ///
    /// ### Why is this bad?
    /// Documentation can be lengthy, which can cause relevant attributes to
    /// be separated from the code they affect by a significant distance.
    /// That can make it easy to miss critical attributes.
    ///
    /// This lint presently applies only to outer attributes ("`#[...`"),
    /// not inner attributes ("`#![...`"). Inner attributes, which appear at the
    /// top of modules/files, have less reason to be strictly ordered.
    /// In future, this lint might be updated to check inner attributes.
    ///
    /// ### Why does this miss some cases?
    /// Some attributes are not visible to this lint, including `#[derive...]`,
    /// any `#[cfg_attr(pred, ...)]` where the predicate evaluates to false,
    /// and any other attributes that the compiler removes before this lint runs.
    ///
    /// This lint accepts `#[cfg_attr(..., doc = "")]` mixed with documentation.
    /// This pattern is therefore allowed in documentation.  For example, this
    /// can ensure that doc tests are ignored for disabled features.
    ///
    /// ### Why this cannot be fixed automatically
    /// Some attributes, such as proc macros, might depend on specific ordering,
    /// such that changing their order attributes could affect program behavior.
    ///
    /// ### Example
    /// ```no_run
    /// #[test]
    /// /// Documentation should precede other attributes.
    /// fn check() {}
    /// ```
    /// Use instead:
    /// ```no_run
    /// /// Documentation should precede other attributes.
    /// #[test]
    /// fn check() {}
    /// ```
    #[clippy::version = "1.99.0"]
    pub DOC_ATTR_ORDERING,
    pedantic,
    "doc attribute ordering"
}

impl_lint_pass!(DocAttrOrdering => [DOC_ATTR_ORDERING]);

#[derive(Default)]
pub struct DocAttrOrdering {
    /// Statements share their attributes with the node they wrap, so
    /// `check_attributes` is run twice. The two visits are adjacent,
    /// so remembering the last scan is enough to avoid duplicates.
    last: Option<AttrId>,
}

impl DocAttrOrdering {
    fn needs_checking(&mut self, attrs: &[Attribute]) -> bool {
        if attrs.is_empty() {
            false
        } else {
            let id = attrs.first().map(|a| a.id);
            if self.last == id {
                false
            } else {
                self.last = id;
                true
            }
        }
    }
}

impl EarlyLintPass for DocAttrOrdering {
    fn check_attributes(&mut self, cx: &EarlyContext<'_>, attrs: &[Attribute]) {
        if self.needs_checking(attrs) {
            let mut first_non_doc = None;
            for attr in attrs {
                if attr.style != AttrStyle::Outer {
                    continue;
                }
                if let AttrKind::Synthetic(ref synth) = attr.kind
                    && matches!(**synth, SyntheticAttr::CfgAttrTrace(..))
                {
                    // Skip `cfg_attr` so that `#[cfg_attr(..., doc = "...")]` is accepted.
                    // Any enabled attribute(s) that are produced will be checked separately.
                    continue;
                }

                if attr.doc_str().is_some() {
                    if first_non_doc.is_none() {
                        continue;
                    }
                    span_lint_and_help(
                        cx,
                        DOC_ATTR_ORDERING,
                        attr.span,
                        "doc comments and attributes should precede other attributes",
                        first_non_doc,
                        "place any doc comments and attributes before this attribute",
                    );
                    return;
                }
                if first_non_doc.is_none() {
                    first_non_doc = Some(attr.span);
                }
            }
        }
    }
}
