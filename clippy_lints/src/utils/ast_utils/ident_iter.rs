use core::iter::FusedIterator;
use rustc_ast::visit::{walk_expr, Visitor};
use rustc_ast::Expr;
use rustc_span::symbol::Ident;

pub struct IdentIterator(std::vec::IntoIter<Ident>);

impl Iterator for IdentIterator {
    type Item = Ident;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl FusedIterator for IdentIterator {}

impl From<&Expr> for IdentIterator {
    fn from(expr: &Expr) -> Self {
        let byte_count = (expr.span.hi() - expr.span.lo()).0 as usize;

        // bytes / (bytes / idents) = idents
        let mut visitor = IdentCollector(Vec::with_capacity(byte_count / ESTIMATED_BYTES_OF_CODE_PER_IDENT));

        walk_expr(&mut visitor, expr);

        IdentIterator(visitor.0.into_iter())
    }
}

/// An estimate of the amount of code bytes that one should expect to look at
/// before seeing an `Ident`. This value is used to estimate how many `Ident`
/// slots to pre-allocate for a given `Span`.
const ESTIMATED_BYTES_OF_CODE_PER_IDENT: usize = 16;

struct IdentCollector(Vec<Ident>);

impl Visitor<'_> for IdentCollector {
    fn visit_ident(&mut self, ident: Ident) {
        self.0.push(ident);
    }
}
