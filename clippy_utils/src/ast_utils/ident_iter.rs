use core::iter::FusedIterator;
use rustc_ast::visit::{Visitor, walk_attribute, walk_expr};
use rustc_ast::{Attribute, Expr};
use rustc_span::symbol::Ident;

/// Iterator over all identifiers of a specific node in the AST.
///
/// The iterator is created by using the `From` trait on an AST node and therefore
/// is usually created by calling `.into()`. This is made possible as the `From` trait
/// is implemented for `&Expr` and `&Attribute`.
///
/// # Examples
///
/// ```rust,ignore
/// let mut iter = IdentIter::from(expr);
/// let mut iter: IdentIter = expr.into();
/// ```
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
        let mut visitor = IdentCollector::default();

        walk_expr(&mut visitor, expr);

        IdentIter(visitor.0.into_iter())
    }
}

impl From<&Attribute> for IdentIter {
    fn from(attr: &Attribute) -> Self {
        let mut visitor = IdentCollector::default();

        walk_attribute(&mut visitor, attr);

        IdentIter(visitor.0.into_iter())
    }
}

#[derive(Default)]
struct IdentCollector(Vec<Ident>);

impl Visitor<'_> for IdentCollector {
    fn visit_ident(&mut self, ident: &Ident) {
        self.0.push(*ident);
    }
}
