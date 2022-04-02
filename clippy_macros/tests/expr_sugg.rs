#![feature(rustc_private)]

extern crate clippy_macros;
extern crate rustc_ast;
extern crate rustc_errors;

mod rustc_lint {
    pub struct LateContext<'a>(pub &'a ());
}

mod rustc_span {
    #[derive(Clone, Copy)]
    pub struct SyntaxContext;

    #[derive(Clone, Copy)]
    pub struct Span;
    impl Span {
        pub fn ctxt(self) -> SyntaxContext {
            SyntaxContext
        }
    }
}

mod rustc_hir {
    use crate::clippy_utils::_internal::ExprPosition;
    use crate::rustc_span::Span;

    pub struct Expr<'a> {
        pub value: &'a str,
        pub pos: ExprPosition,
        pub span: Span,
    }
    impl<'a> Expr<'a> {
        pub fn new(value: &'a str, pos: ExprPosition) -> Self {
            Self { value, pos, span: Span }
        }
    }
}

mod clippy_utils {
    pub mod _internal {
        extern crate clippy_utils;
        use crate::rustc_hir::Expr;
        use crate::rustc_lint::LateContext;
        pub use clippy_utils::_internal::{needs_parens, ExprPosition};

        pub fn expr_position(_: &LateContext<'_>, e: &Expr<'_>) -> ExprPosition {
            e.pos
        }

        pub fn snip(
            _: &LateContext<'_>,
            e: &Expr<'_>,
            position: ExprPosition,
            _: crate::rustc_span::SyntaxContext,
            _app: &mut rustc_errors::Applicability,
        ) -> String {
            if position > e.pos {
                format!("({})", e.value)
            } else {
                e.value.to_string()
            }
        }
    }
}

use crate::clippy_utils::_internal::ExprPosition;
use clippy_macros::expr_sugg;
use rustc_ast::Mutability;
use rustc_hir::Expr;

#[test]
fn test() {
    let cx = &rustc_lint::LateContext(&());
    let mut app = rustc_errors::Applicability::MachineApplicable;
    let app = &mut app;
    let closure = Expr::new("", ExprPosition::Closure);
    let closure = &closure;
    let prefix = Expr::new("", ExprPosition::Prefix);
    let prefix = &prefix;
    let callee = Expr::new("", ExprPosition::Callee);
    let callee = &callee;

    assert_eq!(expr_sugg!(x)(cx, closure, app), "x");

    let arg = Expr::new("|| ()", ExprPosition::Closure);
    assert_eq!(expr_sugg!(x({}), &arg)(cx, closure, app), "x(|| ())");
    assert_eq!(expr_sugg!(x({}), &arg)(cx, prefix, app), "x(|| ())");

    let arg = Expr::new("foo", ExprPosition::Suffix);
    assert_eq!(expr_sugg!(x + {}, &arg)(cx, closure, app), "x + foo");
    assert_eq!(expr_sugg!(x + {}, &arg)(cx, prefix, app), "(x + foo)");

    let arg = Expr::new("foo + bar", ExprPosition::AddLhs);
    assert_eq!(expr_sugg!({} + x, &arg)(cx, closure, app), "foo + bar + x");
    assert_eq!(expr_sugg!(x + {}, &arg)(cx, closure, app), "x + (foo + bar)");

    assert_eq!(expr_sugg!(foo.bar)(cx, callee, app), "(foo.bar)");

    let arg = Expr::new("foo + bar", ExprPosition::AddLhs);
    assert_eq!(
        expr_sugg!({} as {}, &arg, "u32")(cx, closure, app),
        "(foo + bar) as u32"
    );

    let arg = Expr::new("0", ExprPosition::Suffix);
    assert_eq!(
        expr_sugg!(<&{mut} Foo<{}>>::bar::<*{} u32>({}), Mutability::Not, "&str", Mutability::Not, &arg)(
            cx, closure, app
        ),
        "<&Foo<&str>>::bar::<*const u32>(0)"
    )
}
