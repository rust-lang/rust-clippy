use core::iter::FusedIterator;
use rustc_ast::visit::{walk_attribute, walk_expr, Visitor};
use rustc_ast::{Attribute, Expr};
use rustc_span::{symbol::Ident, Span};

pub struct IdentIter(std::vec::IntoIter<Ident>);

impl Iterator for IdentIter {
    type Item = Ident;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl FusedIterator for IdentIter {}

impl From<&Expr> for IdentIter {
    fn from(expr: &Expr) -> Self {
        let mut visitor = IdentCollector::new(expr.span);

        walk_expr(&mut visitor, expr);

        IdentIter(visitor.0.into_iter())
    }
}

impl From<&Attribute> for IdentIter {
    fn from(attr: &Attribute) -> Self {
        let mut visitor = IdentCollector::new(attr.span);

        walk_attribute(&mut visitor, attr);

        IdentIter(visitor.0.into_iter())
    }
}

/// An estimate of the amount of code bytes that one should expect to look at
/// before seeing an `Ident`. This value is used to estimate how many `Ident`
/// slots to pre-allocate for a given `Span`.
const ESTIMATED_BYTES_OF_CODE_PER_IDENT: usize = 16;

struct IdentCollector(Vec<Ident>);

impl IdentCollector {
    fn new(span: Span) -> Self {
        let byte_count = (span.hi() - span.lo()).0 as usize;

        // bytes / (bytes / idents) = idents
        IdentCollector(Vec::with_capacity(byte_count / ESTIMATED_BYTES_OF_CODE_PER_IDENT))
    }
}

impl Visitor<'_> for IdentCollector {
    fn visit_ident(&mut self, ident: Ident) {
        self.0.push(ident);
    }
}
