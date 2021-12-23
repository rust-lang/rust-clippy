//! Contains utility functions to generate suggestions.
#![deny(clippy::missing_docs_in_private_items)]

use crate::source::{
    snippet, snippet_opt, snippet_with_applicability, snippet_with_context, snippet_with_macro_callsite,
};
use crate::{get_parent_expr_for_hir, higher};
use rustc_ast::util::parser::AssocOp;
use rustc_ast::{ast, token};
use rustc_ast_pretty::pprust::token_kind_to_string;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::{ExprKind, HirId, MutTy, TyKind};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{EarlyContext, LateContext, LintContext};
use rustc_middle::hir::place::ProjectionKind;
use rustc_middle::mir::{FakeReadCause, Mutability};
use rustc_middle::ty;
use rustc_span::source_map::{BytePos, CharPos, Pos, Span, SyntaxContext};
use rustc_typeck::expr_use_visitor::{Delegate, ExprUseVisitor, PlaceBase, PlaceWithHirId};
use std::borrow::Cow;
use std::convert::TryInto;
use std::fmt::Display;
use std::iter;
use std::ops::{Add, Neg, Not, Sub};

/// A helper type to build suggestion correctly handling parentheses.
#[derive(Clone, PartialEq)]
pub enum Sugg<'a> {
    /// An expression that never needs parentheses such as `1337` or `[0; 42]`.
    NonParen(Cow<'a, str>),
    /// An expression that does not fit in other variants.
    MaybeParen(Cow<'a, str>),
    /// A binary operator expression, including `as`-casts and explicit type
    /// coercion.
    BinOp(AssocOp, Cow<'a, str>),
}

/// Literal constant `0`, for convenience.
pub const ZERO: Sugg<'static> = Sugg::NonParen(Cow::Borrowed("0"));
/// Literal constant `1`, for convenience.
pub const ONE: Sugg<'static> = Sugg::NonParen(Cow::Borrowed("1"));
/// a constant represents an empty string, for convenience.
pub const EMPTY: Sugg<'static> = Sugg::NonParen(Cow::Borrowed(""));

impl Display for Sugg<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match *self {
            Sugg::NonParen(ref s) | Sugg::MaybeParen(ref s) | Sugg::BinOp(_, ref s) => s.fmt(f),
        }
    }
}

#[allow(clippy::wrong_self_convention)] // ok, because of the function `as_ty` method
impl<'a> Sugg<'a> {
    /// Prepare a suggestion from an expression.
    pub fn hir_opt(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> Option<Self> {
        snippet_opt(cx, expr.span).map(|snippet| {
            let snippet = Cow::Owned(snippet);
            Self::hir_from_snippet(expr, snippet)
        })
    }

    /// Convenience function around `hir_opt` for suggestions with a default
    /// text.
    pub fn hir(cx: &LateContext<'_>, expr: &hir::Expr<'_>, default: &'a str) -> Self {
        Self::hir_opt(cx, expr).unwrap_or(Sugg::NonParen(Cow::Borrowed(default)))
    }

    /// Same as `hir`, but it adapts the applicability level by following rules:
    ///
    /// - Applicability level `Unspecified` will never be changed.
    /// - If the span is inside a macro, change the applicability level to `MaybeIncorrect`.
    /// - If the default value is used and the applicability level is `MachineApplicable`, change it
    ///   to
    /// `HasPlaceholders`
    pub fn hir_with_applicability(
        cx: &LateContext<'_>,
        expr: &hir::Expr<'_>,
        default: &'a str,
        applicability: &mut Applicability,
    ) -> Self {
        if *applicability != Applicability::Unspecified && expr.span.from_expansion() {
            *applicability = Applicability::MaybeIncorrect;
        }
        Self::hir_opt(cx, expr).unwrap_or_else(|| {
            if *applicability == Applicability::MachineApplicable {
                *applicability = Applicability::HasPlaceholders;
            }
            Sugg::NonParen(Cow::Borrowed(default))
        })
    }

    /// Same as `hir`, but will use the pre expansion span if the `expr` was in a macro.
    pub fn hir_with_macro_callsite(cx: &LateContext<'_>, expr: &hir::Expr<'_>, default: &'a str) -> Self {
        let snippet = snippet_with_macro_callsite(cx, expr.span, default);

        Self::hir_from_snippet(expr, snippet)
    }

    /// Same as `hir`, but first walks the span up to the given context. This will result in the
    /// macro call, rather then the expansion, if the span is from a child context. If the span is
    /// not from a child context, it will be used directly instead.
    ///
    /// e.g. Given the expression `&vec![]`, getting a snippet from the span for `vec![]` as a HIR
    /// node would result in `box []`. If given the context of the address of expression, this
    /// function will correctly get a snippet of `vec![]`.
    pub fn hir_with_context(
        cx: &LateContext<'_>,
        expr: &hir::Expr<'_>,
        ctxt: SyntaxContext,
        default: &'a str,
        applicability: &mut Applicability,
    ) -> Self {
        let (snippet, in_macro) = snippet_with_context(cx, expr.span, ctxt, default, applicability);

        if in_macro {
            Sugg::NonParen(snippet)
        } else {
            Self::hir_from_snippet(expr, snippet)
        }
    }

    /// Generate a suggestion for an expression with the given snippet. This is used by the `hir_*`
    /// function variants of `Sugg`, since these use different snippet functions.
    fn hir_from_snippet(expr: &hir::Expr<'_>, snippet: Cow<'a, str>) -> Self {
        if let Some(range) = higher::Range::hir(expr) {
            let op = match range.limits {
                ast::RangeLimits::HalfOpen => AssocOp::DotDot,
                ast::RangeLimits::Closed => AssocOp::DotDotEq,
            };
            return Sugg::BinOp(op, snippet);
        }

        match expr.kind {
            hir::ExprKind::AddrOf(..)
            | hir::ExprKind::Box(..)
            | hir::ExprKind::If(..)
            | hir::ExprKind::Let(..)
            | hir::ExprKind::Closure(..)
            | hir::ExprKind::Unary(..)
            | hir::ExprKind::Match(..) => Sugg::MaybeParen(snippet),
            hir::ExprKind::Continue(..)
            | hir::ExprKind::Yield(..)
            | hir::ExprKind::Array(..)
            | hir::ExprKind::Block(..)
            | hir::ExprKind::Break(..)
            | hir::ExprKind::Call(..)
            | hir::ExprKind::Field(..)
            | hir::ExprKind::Index(..)
            | hir::ExprKind::InlineAsm(..)
            | hir::ExprKind::LlvmInlineAsm(..)
            | hir::ExprKind::ConstBlock(..)
            | hir::ExprKind::Lit(..)
            | hir::ExprKind::Loop(..)
            | hir::ExprKind::MethodCall(..)
            | hir::ExprKind::Path(..)
            | hir::ExprKind::Repeat(..)
            | hir::ExprKind::Ret(..)
            | hir::ExprKind::Struct(..)
            | hir::ExprKind::Tup(..)
            | hir::ExprKind::DropTemps(_)
            | hir::ExprKind::Err => Sugg::NonParen(snippet),
            hir::ExprKind::Assign(..) => Sugg::BinOp(AssocOp::Assign, snippet),
            hir::ExprKind::AssignOp(op, ..) => Sugg::BinOp(hirbinop2assignop(op), snippet),
            hir::ExprKind::Binary(op, ..) => Sugg::BinOp(AssocOp::from_ast_binop(op.node.into()), snippet),
            hir::ExprKind::Cast(..) => Sugg::BinOp(AssocOp::As, snippet),
            hir::ExprKind::Type(..) => Sugg::BinOp(AssocOp::Colon, snippet),
        }
    }

    /// Prepare a suggestion from an expression.
    pub fn ast(cx: &EarlyContext<'_>, expr: &ast::Expr, default: &'a str) -> Self {
        use rustc_ast::ast::RangeLimits;

        let snippet = if expr.span.from_expansion() {
            snippet_with_macro_callsite(cx, expr.span, default)
        } else {
            snippet(cx, expr.span, default)
        };

        match expr.kind {
            ast::ExprKind::AddrOf(..)
            | ast::ExprKind::Box(..)
            | ast::ExprKind::Closure(..)
            | ast::ExprKind::If(..)
            | ast::ExprKind::Let(..)
            | ast::ExprKind::Unary(..)
            | ast::ExprKind::Match(..) => Sugg::MaybeParen(snippet),
            ast::ExprKind::Async(..)
            | ast::ExprKind::Block(..)
            | ast::ExprKind::Break(..)
            | ast::ExprKind::Call(..)
            | ast::ExprKind::Continue(..)
            | ast::ExprKind::Yield(..)
            | ast::ExprKind::Field(..)
            | ast::ExprKind::ForLoop(..)
            | ast::ExprKind::Index(..)
            | ast::ExprKind::InlineAsm(..)
            | ast::ExprKind::LlvmInlineAsm(..)
            | ast::ExprKind::ConstBlock(..)
            | ast::ExprKind::Lit(..)
            | ast::ExprKind::Loop(..)
            | ast::ExprKind::MacCall(..)
            | ast::ExprKind::MethodCall(..)
            | ast::ExprKind::Paren(..)
            | ast::ExprKind::Underscore
            | ast::ExprKind::Path(..)
            | ast::ExprKind::Repeat(..)
            | ast::ExprKind::Ret(..)
            | ast::ExprKind::Struct(..)
            | ast::ExprKind::Try(..)
            | ast::ExprKind::TryBlock(..)
            | ast::ExprKind::Tup(..)
            | ast::ExprKind::Array(..)
            | ast::ExprKind::While(..)
            | ast::ExprKind::Await(..)
            | ast::ExprKind::Err => Sugg::NonParen(snippet),
            ast::ExprKind::Range(.., RangeLimits::HalfOpen) => Sugg::BinOp(AssocOp::DotDot, snippet),
            ast::ExprKind::Range(.., RangeLimits::Closed) => Sugg::BinOp(AssocOp::DotDotEq, snippet),
            ast::ExprKind::Assign(..) => Sugg::BinOp(AssocOp::Assign, snippet),
            ast::ExprKind::AssignOp(op, ..) => Sugg::BinOp(astbinop2assignop(op), snippet),
            ast::ExprKind::Binary(op, ..) => Sugg::BinOp(AssocOp::from_ast_binop(op.node), snippet),
            ast::ExprKind::Cast(..) => Sugg::BinOp(AssocOp::As, snippet),
            ast::ExprKind::Type(..) => Sugg::BinOp(AssocOp::Colon, snippet),
        }
    }

    /// Convenience method to create the `<lhs> && <rhs>` suggestion.
    pub fn and(self, rhs: &Self) -> Sugg<'static> {
        make_binop(ast::BinOpKind::And, &self, rhs)
    }

    /// Convenience method to create the `<lhs> & <rhs>` suggestion.
    pub fn bit_and(self, rhs: &Self) -> Sugg<'static> {
        make_binop(ast::BinOpKind::BitAnd, &self, rhs)
    }

    /// Convenience method to create the `<lhs> as <rhs>` suggestion.
    pub fn as_ty<R: Display>(self, rhs: R) -> Sugg<'static> {
        make_assoc(AssocOp::As, &self, &Sugg::NonParen(rhs.to_string().into()))
    }

    /// Convenience method to create the `&<expr>` suggestion.
    pub fn addr(self) -> Sugg<'static> {
        make_unop("&", self)
    }

    /// Convenience method to create the `&mut <expr>` suggestion.
    pub fn mut_addr(self) -> Sugg<'static> {
        make_unop("&mut ", self)
    }

    /// Convenience method to create the `*<expr>` suggestion.
    pub fn deref(self) -> Sugg<'static> {
        make_unop("*", self)
    }

    /// Convenience method to create the `&*<expr>` suggestion. Currently this
    /// is needed because `sugg.deref().addr()` produces an unnecessary set of
    /// parentheses around the deref.
    pub fn addr_deref(self) -> Sugg<'static> {
        make_unop("&*", self)
    }

    /// Convenience method to create the `&mut *<expr>` suggestion. Currently
    /// this is needed because `sugg.deref().mut_addr()` produces an unnecessary
    /// set of parentheses around the deref.
    pub fn mut_addr_deref(self) -> Sugg<'static> {
        make_unop("&mut *", self)
    }

    /// Convenience method to transform suggestion into a return call
    pub fn make_return(self) -> Sugg<'static> {
        Sugg::NonParen(Cow::Owned(format!("return {}", self)))
    }

    /// Convenience method to transform suggestion into a block
    /// where the suggestion is a trailing expression
    pub fn blockify(self) -> Sugg<'static> {
        Sugg::NonParen(Cow::Owned(format!("{{ {} }}", self)))
    }

    /// Convenience method to create the `<lhs>..<rhs>` or `<lhs>...<rhs>`
    /// suggestion.
    #[allow(dead_code)]
    pub fn range(self, end: &Self, limit: ast::RangeLimits) -> Sugg<'static> {
        match limit {
            ast::RangeLimits::HalfOpen => make_assoc(AssocOp::DotDot, &self, end),
            ast::RangeLimits::Closed => make_assoc(AssocOp::DotDotEq, &self, end),
        }
    }

    /// Adds parentheses to any expression that might need them. Suitable to the
    /// `self` argument of a method call
    /// (e.g., to build `bar.foo()` or `(1 + 2).foo()`).
    #[must_use]
    pub fn maybe_par(self) -> Self {
        match self {
            Sugg::NonParen(..) => self,
            // `(x)` and `(x).y()` both don't need additional parens.
            Sugg::MaybeParen(sugg) => {
                if has_enclosing_paren(&sugg) {
                    Sugg::MaybeParen(sugg)
                } else {
                    Sugg::NonParen(format!("({})", sugg).into())
                }
            },
            Sugg::BinOp(_, sugg) => {
                if has_enclosing_paren(&sugg) {
                    Sugg::NonParen(sugg)
                } else {
                    Sugg::NonParen(format!("({})", sugg).into())
                }
            },
        }
    }
}

/// Return `true` if `sugg` is enclosed in parenthesis.
fn has_enclosing_paren(sugg: impl AsRef<str>) -> bool {
    let mut chars = sugg.as_ref().chars();
    if chars.next() == Some('(') {
        let mut depth = 1;
        for c in &mut chars {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            }
            if depth == 0 {
                break;
            }
        }
        chars.next().is_none()
    } else {
        false
    }
}

/// Copied from the rust standard library, and then edited
macro_rules! forward_binop_impls_to_ref {
    (impl $imp:ident, $method:ident for $t:ty, type Output = $o:ty) => {
        impl $imp<$t> for &$t {
            type Output = $o;

            fn $method(self, other: $t) -> $o {
                $imp::$method(self, &other)
            }
        }

        impl $imp<&$t> for $t {
            type Output = $o;

            fn $method(self, other: &$t) -> $o {
                $imp::$method(&self, other)
            }
        }

        impl $imp for $t {
            type Output = $o;

            fn $method(self, other: $t) -> $o {
                $imp::$method(&self, &other)
            }
        }
    };
}

impl Add for &Sugg<'_> {
    type Output = Sugg<'static>;
    fn add(self, rhs: &Sugg<'_>) -> Sugg<'static> {
        make_binop(ast::BinOpKind::Add, self, rhs)
    }
}

impl Sub for &Sugg<'_> {
    type Output = Sugg<'static>;
    fn sub(self, rhs: &Sugg<'_>) -> Sugg<'static> {
        make_binop(ast::BinOpKind::Sub, self, rhs)
    }
}

forward_binop_impls_to_ref!(impl Add, add for Sugg<'_>, type Output = Sugg<'static>);
forward_binop_impls_to_ref!(impl Sub, sub for Sugg<'_>, type Output = Sugg<'static>);

impl Neg for Sugg<'_> {
    type Output = Sugg<'static>;
    fn neg(self) -> Sugg<'static> {
        make_unop("-", self)
    }
}

impl Not for Sugg<'_> {
    type Output = Sugg<'static>;
    fn not(self) -> Sugg<'static> {
        make_unop("!", self)
    }
}

/// Helper type to display either `foo` or `(foo)`.
struct ParenHelper<T> {
    /// `true` if parentheses are needed.
    paren: bool,
    /// The main thing to display.
    wrapped: T,
}

impl<T> ParenHelper<T> {
    /// Builds a `ParenHelper`.
    fn new(paren: bool, wrapped: T) -> Self {
        Self { paren, wrapped }
    }
}

impl<T: Display> Display for ParenHelper<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if self.paren {
            write!(f, "({})", self.wrapped)
        } else {
            self.wrapped.fmt(f)
        }
    }
}

/// Builds the string for `<op><expr>` adding parenthesis when necessary.
///
/// For convenience, the operator is taken as a string because all unary
/// operators have the same
/// precedence.
pub fn make_unop(op: &str, expr: Sugg<'_>) -> Sugg<'static> {
    Sugg::MaybeParen(format!("{}{}", op, expr.maybe_par()).into())
}

/// Builds the string for `<lhs> <op> <rhs>` adding parenthesis when necessary.
///
/// Precedence of shift operator relative to other arithmetic operation is
/// often confusing so
/// parenthesis will always be added for a mix of these.
pub fn make_assoc(op: AssocOp, lhs: &Sugg<'_>, rhs: &Sugg<'_>) -> Sugg<'static> {
    /// Returns `true` if the operator is a shift operator `<<` or `>>`.
    fn is_shift(op: AssocOp) -> bool {
        matches!(op, AssocOp::ShiftLeft | AssocOp::ShiftRight)
    }

    /// Returns `true` if the operator is an arithmetic operator
    /// (i.e., `+`, `-`, `*`, `/`, `%`).
    fn is_arith(op: AssocOp) -> bool {
        matches!(
            op,
            AssocOp::Add | AssocOp::Subtract | AssocOp::Multiply | AssocOp::Divide | AssocOp::Modulus
        )
    }

    /// Returns `true` if the operator `op` needs parenthesis with the operator
    /// `other` in the direction `dir`.
    fn needs_paren(op: AssocOp, other: AssocOp, dir: Associativity) -> bool {
        other.precedence() < op.precedence()
            || (other.precedence() == op.precedence()
                && ((op != other && associativity(op) != dir)
                    || (op == other && associativity(op) != Associativity::Both)))
            || is_shift(op) && is_arith(other)
            || is_shift(other) && is_arith(op)
    }

    let lhs_paren = if let Sugg::BinOp(lop, _) = *lhs {
        needs_paren(op, lop, Associativity::Left)
    } else {
        false
    };

    let rhs_paren = if let Sugg::BinOp(rop, _) = *rhs {
        needs_paren(op, rop, Associativity::Right)
    } else {
        false
    };

    let lhs = ParenHelper::new(lhs_paren, lhs);
    let rhs = ParenHelper::new(rhs_paren, rhs);
    let sugg = match op {
        AssocOp::Add
        | AssocOp::BitAnd
        | AssocOp::BitOr
        | AssocOp::BitXor
        | AssocOp::Divide
        | AssocOp::Equal
        | AssocOp::Greater
        | AssocOp::GreaterEqual
        | AssocOp::LAnd
        | AssocOp::LOr
        | AssocOp::Less
        | AssocOp::LessEqual
        | AssocOp::Modulus
        | AssocOp::Multiply
        | AssocOp::NotEqual
        | AssocOp::ShiftLeft
        | AssocOp::ShiftRight
        | AssocOp::Subtract => format!(
            "{} {} {}",
            lhs,
            op.to_ast_binop().expect("Those are AST ops").to_string(),
            rhs
        ),
        AssocOp::Assign => format!("{} = {}", lhs, rhs),
        AssocOp::AssignOp(op) => format!("{} {}= {}", lhs, token_kind_to_string(&token::BinOp(op)), rhs),
        AssocOp::As => format!("{} as {}", lhs, rhs),
        AssocOp::DotDot => format!("{}..{}", lhs, rhs),
        AssocOp::DotDotEq => format!("{}..={}", lhs, rhs),
        AssocOp::Colon => format!("{}: {}", lhs, rhs),
    };

    Sugg::BinOp(op, sugg.into())
}

/// Convenience wrapper around `make_assoc` and `AssocOp::from_ast_binop`.
pub fn make_binop(op: ast::BinOpKind, lhs: &Sugg<'_>, rhs: &Sugg<'_>) -> Sugg<'static> {
    make_assoc(AssocOp::from_ast_binop(op), lhs, rhs)
}

#[derive(PartialEq, Eq, Clone, Copy)]
/// Operator associativity.
enum Associativity {
    /// The operator is both left-associative and right-associative.
    Both,
    /// The operator is left-associative.
    Left,
    /// The operator is not associative.
    None,
    /// The operator is right-associative.
    Right,
}

/// Returns the associativity/fixity of an operator. The difference with
/// `AssocOp::fixity` is that an operator can be both left and right associative
/// (such as `+`: `a + b + c == (a + b) + c == a + (b + c)`.
///
/// Chained `as` and explicit `:` type coercion never need inner parenthesis so
/// they are considered
/// associative.
#[must_use]
fn associativity(op: AssocOp) -> Associativity {
    use rustc_ast::util::parser::AssocOp::{
        Add, As, Assign, AssignOp, BitAnd, BitOr, BitXor, Colon, Divide, DotDot, DotDotEq, Equal, Greater,
        GreaterEqual, LAnd, LOr, Less, LessEqual, Modulus, Multiply, NotEqual, ShiftLeft, ShiftRight, Subtract,
    };

    match op {
        Assign | AssignOp(_) => Associativity::Right,
        Add | BitAnd | BitOr | BitXor | LAnd | LOr | Multiply | As | Colon => Associativity::Both,
        Divide | Equal | Greater | GreaterEqual | Less | LessEqual | Modulus | NotEqual | ShiftLeft | ShiftRight
        | Subtract => Associativity::Left,
        DotDot | DotDotEq => Associativity::None,
    }
}

/// Converts a `hir::BinOp` to the corresponding assigning binary operator.
fn hirbinop2assignop(op: hir::BinOp) -> AssocOp {
    use rustc_ast::token::BinOpToken::{And, Caret, Minus, Or, Percent, Plus, Shl, Shr, Slash, Star};

    AssocOp::AssignOp(match op.node {
        hir::BinOpKind::Add => Plus,
        hir::BinOpKind::BitAnd => And,
        hir::BinOpKind::BitOr => Or,
        hir::BinOpKind::BitXor => Caret,
        hir::BinOpKind::Div => Slash,
        hir::BinOpKind::Mul => Star,
        hir::BinOpKind::Rem => Percent,
        hir::BinOpKind::Shl => Shl,
        hir::BinOpKind::Shr => Shr,
        hir::BinOpKind::Sub => Minus,

        hir::BinOpKind::And
        | hir::BinOpKind::Eq
        | hir::BinOpKind::Ge
        | hir::BinOpKind::Gt
        | hir::BinOpKind::Le
        | hir::BinOpKind::Lt
        | hir::BinOpKind::Ne
        | hir::BinOpKind::Or => panic!("This operator does not exist"),
    })
}

/// Converts an `ast::BinOp` to the corresponding assigning binary operator.
fn astbinop2assignop(op: ast::BinOp) -> AssocOp {
    use rustc_ast::ast::BinOpKind::{
        Add, And, BitAnd, BitOr, BitXor, Div, Eq, Ge, Gt, Le, Lt, Mul, Ne, Or, Rem, Shl, Shr, Sub,
    };
    use rustc_ast::token::BinOpToken;

    AssocOp::AssignOp(match op.node {
        Add => BinOpToken::Plus,
        BitAnd => BinOpToken::And,
        BitOr => BinOpToken::Or,
        BitXor => BinOpToken::Caret,
        Div => BinOpToken::Slash,
        Mul => BinOpToken::Star,
        Rem => BinOpToken::Percent,
        Shl => BinOpToken::Shl,
        Shr => BinOpToken::Shr,
        Sub => BinOpToken::Minus,
        And | Eq | Ge | Gt | Le | Lt | Ne | Or => panic!("This operator does not exist"),
    })
}

/// Returns the indentation before `span` if there are nothing but `[ \t]`
/// before it on its line.
fn indentation<T: LintContext>(cx: &T, span: Span) -> Option<String> {
    let lo = cx.sess().source_map().lookup_char_pos(span.lo());
    lo.file
        .get_line(lo.line - 1 /* line numbers in `Loc` are 1-based */)
        .and_then(|line| {
            if let Some((pos, _)) = line.char_indices().find(|&(_, c)| c != ' ' && c != '\t') {
                // We can mix char and byte positions here because we only consider `[ \t]`.
                if lo.col == CharPos(pos) {
                    Some(line[..pos].into())
                } else {
                    None
                }
            } else {
                None
            }
        })
}

/// Convenience extension trait for `DiagnosticBuilder`.
pub trait DiagnosticBuilderExt<T: LintContext> {
    /// Suggests to add an attribute to an item.
    ///
    /// Correctly handles indentation of the attribute and item.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// diag.suggest_item_with_attr(cx, item, "#[derive(Default)]");
    /// ```
    fn suggest_item_with_attr<D: Display + ?Sized>(
        &mut self,
        cx: &T,
        item: Span,
        msg: &str,
        attr: &D,
        applicability: Applicability,
    );

    /// Suggest to add an item before another.
    ///
    /// The item should not be indented (except for inner indentation).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// diag.suggest_prepend_item(cx, item,
    /// "fn foo() {
    ///     bar();
    /// }");
    /// ```
    fn suggest_prepend_item(&mut self, cx: &T, item: Span, msg: &str, new_item: &str, applicability: Applicability);

    /// Suggest to completely remove an item.
    ///
    /// This will remove an item and all following whitespace until the next non-whitespace
    /// character. This should work correctly if item is on the same indentation level as the
    /// following item.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// diag.suggest_remove_item(cx, item, "remove this")
    /// ```
    fn suggest_remove_item(&mut self, cx: &T, item: Span, msg: &str, applicability: Applicability);
}

impl<T: LintContext> DiagnosticBuilderExt<T> for rustc_errors::DiagnosticBuilder<'_> {
    fn suggest_item_with_attr<D: Display + ?Sized>(
        &mut self,
        cx: &T,
        item: Span,
        msg: &str,
        attr: &D,
        applicability: Applicability,
    ) {
        if let Some(indent) = indentation(cx, item) {
            let span = item.with_hi(item.lo());

            self.span_suggestion(span, msg, format!("{}\n{}", attr, indent), applicability);
        }
    }

    fn suggest_prepend_item(&mut self, cx: &T, item: Span, msg: &str, new_item: &str, applicability: Applicability) {
        if let Some(indent) = indentation(cx, item) {
            let span = item.with_hi(item.lo());

            let mut first = true;
            let new_item = new_item
                .lines()
                .map(|l| {
                    if first {
                        first = false;
                        format!("{}\n", l)
                    } else {
                        format!("{}{}\n", indent, l)
                    }
                })
                .collect::<String>();

            self.span_suggestion(span, msg, format!("{}\n{}", new_item, indent), applicability);
        }
    }

    fn suggest_remove_item(&mut self, cx: &T, item: Span, msg: &str, applicability: Applicability) {
        let mut remove_span = item;
        let hi = cx.sess().source_map().next_point(remove_span).hi();
        let fmpos = cx.sess().source_map().lookup_byte_offset(hi);

        if let Some(ref src) = fmpos.sf.src {
            let non_whitespace_offset = src[fmpos.pos.to_usize()..].find(|c| c != ' ' && c != '\t' && c != '\n');

            if let Some(non_whitespace_offset) = non_whitespace_offset {
                remove_span = remove_span
                    .with_hi(remove_span.hi() + BytePos(non_whitespace_offset.try_into().expect("offset too large")));
            }
        }

        self.span_suggestion(remove_span, msg, String::new(), applicability);
    }
}

/// Suggestion results for handling closure
/// args dereferencing and borrowing
pub struct DerefClosure {
    /// confidence on the built suggestion
    pub applicability: Applicability,
    /// gradually built suggestion
    pub suggestion: String,
}

/// Build suggestion gradually by handling closure arg specific usages,
/// such as explicit deref and borrowing cases.
/// Returns `None` if no such use cases have been triggered in closure body
///
/// note: this only works on single line immutable closures with exactly one input parameter.
pub fn deref_closure_args<'tcx>(cx: &LateContext<'_>, closure: &'tcx hir::Expr<'_>) -> Option<DerefClosure> {
    if let hir::ExprKind::Closure(_, fn_decl, body_id, ..) = closure.kind {
        let closure_body = cx.tcx.hir().body(body_id);
        // is closure arg a type annotated double reference (i.e.: `|x: &&i32| ...`)
        // a type annotation is present if param `kind` is different from `TyKind::Infer`
        let closure_arg_is_type_annotated_double_ref = if let TyKind::Rptr(_, MutTy { ty, .. }) = fn_decl.inputs[0].kind
        {
            matches!(ty.kind, TyKind::Rptr(_, MutTy { .. }))
        } else {
            false
        };

        let mut visitor = DerefDelegate {
            cx,
            closure_span: closure.span,
            closure_arg_is_type_annotated_double_ref,
            next_pos: closure.span.lo(),
            suggestion_start: String::new(),
            applicability: Applicability::MaybeIncorrect,
        };

        let fn_def_id = cx.tcx.hir().local_def_id(closure.hir_id);
        cx.tcx.infer_ctxt().enter(|infcx| {
            ExprUseVisitor::new(&mut visitor, &infcx, fn_def_id, cx.param_env, cx.typeck_results())
                .consume_body(closure_body);
        });

        if !visitor.suggestion_start.is_empty() {
            return Some(DerefClosure {
                applicability: visitor.applicability,
                suggestion: visitor.finish(),
            });
        }
    }
    None
}

/// Visitor struct used for tracking down
/// dereferencing and borrowing of closure's args
struct DerefDelegate<'a, 'tcx> {
    /// The late context of the lint
    cx: &'a LateContext<'tcx>,
    /// The span of the input closure to adapt
    closure_span: Span,
    /// Indicates if the arg of the closure is a type annotated double reference
    closure_arg_is_type_annotated_double_ref: bool,
    /// last position of the span to gradually build the suggestion
    next_pos: BytePos,
    /// starting part of the gradually built suggestion
    suggestion_start: String,
    /// confidence on the built suggestion
    applicability: Applicability,
}

impl DerefDelegate<'_, 'tcx> {
    /// build final suggestion:
    /// - create the ending part of suggestion
    /// - concatenate starting and ending parts
    /// - potentially remove needless borrowing
    pub fn finish(&mut self) -> String {
        let end_span = Span::new(self.next_pos, self.closure_span.hi(), self.closure_span.ctxt(), None);
        let end_snip = snippet_with_applicability(self.cx, end_span, "..", &mut self.applicability);
        let sugg = format!("{}{}", self.suggestion_start, end_snip);
        if self.closure_arg_is_type_annotated_double_ref {
            sugg.replacen('&', "", 1)
        } else {
            sugg
        }
    }

    /// indicates whether the function from `parent_expr` takes its args by double reference
    fn func_takes_arg_by_double_ref(&self, parent_expr: &'tcx hir::Expr<'_>, cmt_hir_id: HirId) -> bool {
        let (call_args, inputs) = match parent_expr.kind {
            ExprKind::MethodCall(_, _, call_args, _) => {
                if let Some(method_did) = self.cx.typeck_results().type_dependent_def_id(parent_expr.hir_id) {
                    (call_args, self.cx.tcx.fn_sig(method_did).skip_binder().inputs())
                } else {
                    return false;
                }
            },
            ExprKind::Call(func, call_args) => {
                let typ = self.cx.typeck_results().expr_ty(func);
                (call_args, typ.fn_sig(self.cx.tcx).skip_binder().inputs())
            },
            _ => return false,
        };

        iter::zip(call_args, inputs)
            .any(|(arg, ty)| arg.hir_id == cmt_hir_id && matches!(ty.kind(), ty::Ref(_, inner, _) if inner.is_ref()))
    }
}

impl<'tcx> Delegate<'tcx> for DerefDelegate<'_, 'tcx> {
    fn consume(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    #[allow(clippy::too_many_lines)]
    fn borrow(&mut self, cmt: &PlaceWithHirId<'tcx>, _: HirId, _: ty::BorrowKind) {
        if let PlaceBase::Local(id) = cmt.place.base {
            let map = self.cx.tcx.hir();
            let span = map.span(cmt.hir_id);
            let start_span = Span::new(self.next_pos, span.lo(), span.ctxt(), None);
            let mut start_snip = snippet_with_applicability(self.cx, start_span, "..", &mut self.applicability);

            // identifier referring to the variable currently triggered (i.e.: `fp`)
            let ident_str = map.name(id).to_string();
            // full identifier that includes projection (i.e.: `fp.field`)
            let ident_str_with_proj = snippet(self.cx, span, "..").to_string();

            if cmt.place.projections.is_empty() {
                // handle item without any projection, that needs an explicit borrowing
                // i.e.: suggest `&x` instead of `x`
                self.suggestion_start.push_str(&format!("{}&{}", start_snip, ident_str));
            } else {
                // cases where a parent `Call` or `MethodCall` is using the item
                // i.e.: suggest `.contains(&x)` for `.find(|x| [1, 2, 3].contains(x)).is_none()`
                //
                // Note about method calls:
                // - compiler automatically dereference references if the target type is a reference (works also for
                //   function call)
                // - `self` arguments in the case of `x.is_something()` are also automatically (de)referenced, and
                //   no projection should be suggested
                if let Some(parent_expr) = get_parent_expr_for_hir(self.cx, cmt.hir_id) {
                    match &parent_expr.kind {
                        // given expression is the self argument and will be handled completely by the compiler
                        // i.e.: `|x| x.is_something()`
                        ExprKind::MethodCall(_, _, [self_expr, ..], _) if self_expr.hir_id == cmt.hir_id => {
                            self.suggestion_start
                                .push_str(&format!("{}{}", start_snip, ident_str_with_proj));
                            self.next_pos = span.hi();
                            return;
                        },
                        // item is used in a call
                        // i.e.: `Call`: `|x| please(x)` or `MethodCall`: `|x| [1, 2, 3].contains(x)`
                        ExprKind::Call(_, [call_args @ ..]) | ExprKind::MethodCall(_, _, [_, call_args @ ..], _) => {
                            let expr = self.cx.tcx.hir().expect_expr(cmt.hir_id);
                            let arg_ty_kind = self.cx.typeck_results().expr_ty(expr).kind();

                            if matches!(arg_ty_kind, ty::Ref(_, _, Mutability::Not)) {
                                // suggest ampersand if call function is taking args by double reference
                                let takes_arg_by_double_ref =
                                    self.func_takes_arg_by_double_ref(parent_expr, cmt.hir_id);

                                // compiler will automatically dereference field or index projection, so no need
                                // to suggest ampersand, but full identifier that includes projection is required
                                let has_field_or_index_projection =
                                    cmt.place.projections.iter().any(|proj| {
                                        matches!(proj.kind, ProjectionKind::Field(..) | ProjectionKind::Index)
                                    });

                                // no need to bind again if the function doesn't take arg by double ref
                                // and if the item is already a double ref
                                let ident_sugg = if !call_args.is_empty()
                                    && !takes_arg_by_double_ref
                                    && (self.closure_arg_is_type_annotated_double_ref || has_field_or_index_projection)
                                {
                                    let ident = if has_field_or_index_projection {
                                        ident_str_with_proj
                                    } else {
                                        ident_str
                                    };
                                    format!("{}{}", start_snip, ident)
                                } else {
                                    format!("{}&{}", start_snip, ident_str)
                                };
                                self.suggestion_start.push_str(&ident_sugg);
                                self.next_pos = span.hi();
                                return;
                            }

                            self.applicability = Applicability::Unspecified;
                        },
                        _ => (),
                    }
                }

                let mut replacement_str = ident_str;
                let mut projections_handled = false;
                cmt.place.projections.iter().enumerate().for_each(|(i, proj)| {
                    match proj.kind {
                        // Field projection like `|v| v.foo`
                        // no adjustment needed here, as field projections are handled by the compiler
                        ProjectionKind::Field(..) => match cmt.place.ty_before_projection(i).kind() {
                            ty::Adt(..) | ty::Tuple(_) => {
                                replacement_str = ident_str_with_proj.clone();
                                projections_handled = true;
                            },
                            _ => (),
                        },
                        // Index projection like `|x| foo[x]`
                        // the index is dropped so we can't get it to build the suggestion,
                        // so the span is set-up again to get more code, using `span.hi()` (i.e.: `foo[x]`)
                        // instead of `span.lo()` (i.e.: `foo`)
                        ProjectionKind::Index => {
                            let start_span = Span::new(self.next_pos, span.hi(), span.ctxt(), None);
                            start_snip = snippet_with_applicability(self.cx, start_span, "..", &mut self.applicability);
                            replacement_str.clear();
                            projections_handled = true;
                        },
                        // note: unable to trigger `Subslice` kind in tests
                        ProjectionKind::Subslice => (),
                        ProjectionKind::Deref => {
                            // Explicit derefs are typically handled later on, but
                            // some items do not need explicit deref, such as array accesses,
                            // so we mark them as already processed
                            // i.e.: don't suggest `*sub[1..4].len()` for `|sub| sub[1..4].len() == 3`
                            if let ty::Ref(_, inner, _) = cmt.place.ty_before_projection(i).kind() {
                                if matches!(inner.kind(), ty::Ref(_, innermost, _) if innermost.is_array()) {
                                    projections_handled = true;
                                }
                            }
                        },
                    }
                });

                // handle `ProjectionKind::Deref` by removing one explicit deref
                // if no special case was detected (i.e.: suggest `*x` instead of `**x`)
                if !projections_handled {
                    let last_deref = cmt
                        .place
                        .projections
                        .iter()
                        .rposition(|proj| proj.kind == ProjectionKind::Deref);

                    if let Some(pos) = last_deref {
                        let mut projections = cmt.place.projections.clone();
                        projections.truncate(pos);

                        for item in projections {
                            if item.kind == ProjectionKind::Deref {
                                replacement_str = format!("*{}", replacement_str);
                            }
                        }
                    }
                }

                self.suggestion_start
                    .push_str(&format!("{}{}", start_snip, replacement_str));
            }
            self.next_pos = span.hi();
        }
    }

    fn mutate(&mut self, _: &PlaceWithHirId<'tcx>, _: HirId) {}

    fn fake_read(&mut self, _: rustc_typeck::expr_use_visitor::Place<'tcx>, _: FakeReadCause, _: HirId) {}
}

#[cfg(test)]
mod test {
    use super::Sugg;

    use rustc_ast::util::parser::AssocOp;
    use std::borrow::Cow;

    const SUGGESTION: Sugg<'static> = Sugg::NonParen(Cow::Borrowed("function_call()"));

    #[test]
    fn make_return_transform_sugg_into_a_return_call() {
        assert_eq!("return function_call()", SUGGESTION.make_return().to_string());
    }

    #[test]
    fn blockify_transforms_sugg_into_a_block() {
        assert_eq!("{ function_call() }", SUGGESTION.blockify().to_string());
    }

    #[test]
    fn binop_maybe_par() {
        let sugg = Sugg::BinOp(AssocOp::Add, "(1 + 1)".into());
        assert_eq!("(1 + 1)", sugg.maybe_par().to_string());

        let sugg = Sugg::BinOp(AssocOp::Add, "(1 + 1) + (1 + 1)".into());
        assert_eq!("((1 + 1) + (1 + 1))", sugg.maybe_par().to_string());
    }
}
