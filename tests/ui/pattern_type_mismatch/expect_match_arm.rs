//@check-pass
#![warn(clippy::pattern_type_mismatch)]
#![allow(clippy::match_like_matches_macro)]

#[derive(Debug, PartialEq, Eq, Clone)]
enum Expr {
    Boolean(Box<bool>),
    Number(Box<i64>),
    Boxed(Box<Expr>),
    Other(String),
}

impl Expr {
    fn is_scalar(&self) -> bool {
        match self {
            &Expr::Boolean(_) | &Expr::Number(_) => true,
            #[expect(
                clippy::pattern_type_mismatch,
                reason = "Conflicts with clippy::needless_borrowed_references"
            )]
            Expr::Boxed(inner) => inner.is_scalar(),
            _ => false,
        }
    }

    fn is_other(&self) -> bool {
        #[expect(clippy::pattern_type_mismatch, reason = "Works on overall match")]
        match self {
            #[expect(unused_variables, reason = "Expect works on match arms")]
            Expr::Other(s) => true,
            _ => false,
        }
    }
}

fn main() {
    assert!(Expr::Boolean(Box::new(true)).is_scalar());
    assert!(Expr::Number(Box::new(5)).is_scalar());
    assert!(Expr::Boxed(Box::new(Expr::Boolean(Box::new(true)))).is_scalar());
    assert!(!Expr::Other(String::new()).is_scalar());
    assert!(Expr::Other(String::new()).is_other());
}
