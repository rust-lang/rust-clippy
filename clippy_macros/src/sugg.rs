use core::cmp;
use core::fmt::Write;
use core::iter;
use proc_macro2::{Literal, Span, TokenStream, TokenTree};
use quote::{quote, ToTokens};
use syn::parse::discouraged::Speculative;
use syn::parse::{Nothing, Parse, ParseBuffer, ParseStream, Result};
use syn::{braced, bracketed, parenthesized, token, Error, Ident, Token};

trait Token: token::Token + Parse {
    fn write(&self, w: &mut String);
}
impl Token for Ident {
    fn write(&self, w: &mut String) {
        let _ = write!(w, "{}", self);
    }
}
impl Token for Literal {
    fn write(&self, w: &mut String) {
        let _ = write!(w, "{}", self);
    }
}
macro_rules! impl_token {
    ($($tokens:tt)*) => {
        impl Token for Token![$($tokens)*] {
            fn write(&self, w: &mut String) {
                w.push_str(concat!($(stringify!($tokens)),*));
            }
        }
    }
}
impl_token!(+);
impl_token!(+=);
impl_token!(-);
impl_token!(-=);
impl_token!(*);
impl_token!(*=);
impl_token!(/);
impl_token!(/=);
impl_token!(%);
impl_token!(%=);
impl_token!(&);
impl_token!(&&);
impl_token!(&=);
impl_token!(|);
impl_token!(||);
impl_token!(|=);
impl_token!(^);
impl_token!(^=);
impl_token!(<);
impl_token!(<=);
impl_token!(<<);
impl_token!(<<=);
impl_token!(>);
impl_token!(>=);
impl_token!(>>);
impl_token!(>>=);
impl_token!(.);
impl_token!(..);
impl_token!(..=);
impl_token!(!);
impl_token!(!=);
impl_token!(=);
impl_token!(==);
impl_token!(,);
impl_token!(?);
impl_token!(;);
impl_token!(:);
impl_token!(::);
impl_token!(mut);
impl_token!(const);
impl_token!(as);
impl_token!(return);
impl_token!(yield);
impl_token!(box);

macro_rules! op_precedence {
    ($($name:ident => $variant:ident $(($($args:tt)*))?,)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        enum ExprPrec {
            $($name),*
        }
        impl ToTokens for ExprPrec {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                match *self {
                    $(Self::$name => tokens.extend(
                        quote::quote!(rustc_ast::util::parser::ExprPrecedence::$variant $(($($args)*))?)
                    ),)*
                }
            }
        }
    };
}
op_precedence! {
    Closure => Closure,
    Range => Range,
    Assign => Assign,
    Or => Binary(rustc_ast::BinOpKind::Or),
    And => Binary(rustc_ast::BinOpKind::And),
    Eq => Binary(rustc_ast::BinOpKind::Eq),
    BitOr => Binary(rustc_ast::BinOpKind::BitOr),
    BitXor => Binary(rustc_ast::BinOpKind::BitXor),
    BitAnd => Binary(rustc_ast::BinOpKind::BitAnd),
    Shift => Binary(rustc_ast::BinOpKind::Shl),
    Add => Binary(rustc_ast::BinOpKind::Add),
    Mul => Binary(rustc_ast::BinOpKind::Mul),
    Cast => Cast,
    Prefix => Unary,
    // Essentially the same as `Suffix`, except method calls will take precedence over calling a field.
    Field => Field,
    Suffix => Call,
}
impl ExprPrec {
    fn merge_with(self, other: Self) -> Self {
        match (self, other) {
            (Self::Suffix | Self::Field, Self::Suffix | Self::Field) => other,
            _ => core::cmp::min(self, other),
        }
    }
}

macro_rules! op_position {
    ($($name:ident,)*) => {
        /// The position in the ast an expression can be placed. Needed to distinguish the rhs and lhs of a binary
        /// operator.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        enum ExprPos {
            $($name),*
        }
        impl quote::ToTokens for ExprPos {
            fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
                let name = match *self {
                    $(Self::$name => stringify!($name),)*
                };
                let name = proc_macro2::Ident::new(name, proc_macro2::Span::call_site());
                tokens.extend(quote::quote!(clippy_utils::_internal::ExprPosition::#name));
            }
        }
    };
}
op_position! {
    Closure,
    AssignRhs,
    AssignLhs,
    RangeLhs,
    RangeRhs,
    OrLhs,
    OrRhs,
    AndLhs,
    AndRhs,
    EqLhs,
    EqRhs,
    BitOrLhs,
    BitOrRhs,
    BitXorLhs,
    BitXorRhs,
    BitAndLhs,
    BitAndRhs,
    ShiftLhs,
    ShiftRhs,
    AddLhs,
    AddRhs,
    MulLhs,
    MulRhs,
    Cast,
    Prefix,
    Suffix,
    // Method calls take priority of field access
    Callee,
}

macro_rules! bin_op {
    ($($name:ident($prec:ident)[$($op_tt:tt)*],)*) => {
        struct BinOp {
            prec: ExprPrec,
            pos: ExprPos,
        }
        impl SuggBuilder {
            fn parse_bin_op(&mut self, input: ParseStream<'_>, var: Option<ExprVar>) -> Option<BinOp> {
                use ExprPos::*;
                $(if self.consume_op_token_space_prefixed::<Token![$($op_tt)*]>(
                    input,
                    var,
                    concat_idents!($prec, Lhs)
                ) {
                    self.next_string.push(' ');
                    Some(BinOp {
                        prec: ExprPrec::$prec,
                        pos: concat_idents!($prec, Rhs),
                    })
                } else)* {
                    None
                }
            }
        }
    };
}
bin_op! {
    ShlAssign(Assign)[<<=],
    ShrAssign(Assign)[>>=],
    AddAssign(Assign)[+=],
    SubAssign(Assign)[-=],
    MulAssign(Assign)[*=],
    DivAssign(Assign)[/=],
    ModAssign(Assign)[%=],
    AndAssign(Assign)[&=],
    OrAssign(Assign)[|=],
    XorAssign(Assign)[^=],
    And(And)[&&],
    Or(Or)[&&],
    Add(Add)[+],
    Sub(Add)[-],
    Mul(Mul)[*],
    Div(Mul)[/],
    Mod(Mul)[%],
    Shl(Shift)[<<],
    Shr(Shift)[>>],
    BitAnd(BitAnd)[&],
    BitOr(BitOr)[|],
    BitXor(BitXor)[^],
    LtEq(Eq)[<=],
    GtEq(Eq)[>=],
    Lt(Eq)[<],
    Gt(Eq)[>],
    Eq(Eq)[==],
    NotEq(Eq)[!=],
    Assign(Assign)[=],
    RangeIn(Range)[..=],
    RangeEx(Range)[..],
}

mod kw {
    syn::custom_keyword!(path);
    syn::custom_keyword!(expr);
    syn::custom_keyword!(ty);
    syn::custom_keyword!(ident);
}

macro_rules! var_kind {
    ($($name:ident($($path_tt:tt)*),)*) => {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum VarKind { Default $(, $name)* }
        impl Parse for VarKind {
            fn parse(input: ParseStream<'_>) -> Result<Self> {
                let kind = $(if input.peek($($path_tt)*) {
                    input.parse::<$($path_tt)*>()?;
                    Self::$name
                } else)* {
                    Self::Default
                };
                input.parse::<Nothing>().map(|_| kind)
            }
        }
    }
}
var_kind! {
    Mut(token::Mut),
    Expr(kw::expr),
    Path(kw::path),
    Ty(kw::ty),
    Ident(kw::ident),
}

struct Var {
    span: Span,
    kind: VarKind,
}
impl Parse for Var {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        Ok(Self {
            span: braced!(content in input).span,
            kind: content.parse()?,
        })
    }
}

fn parse_var(input: ParseStream) -> Option<Var> {
    parse_var_if(input, |_| true)
}

fn parse_var_if(input: ParseStream, f: impl FnOnce(&Var) -> bool) -> Option<Var> {
    let fork = input.fork();
    fork.parse().ok().filter(f).map(|x| {
        input.advance_to(&fork);
        x
    })
}

trait Group {
    fn content(input: ParseStream<'_>) -> Result<ParseBuffer>;
    const OPEN: char;
    const CLOSE: char;
}
macro_rules! impl_group {
    ($ty:ty, $mac:ident, $open:literal, $close:literal) => {
        impl Group for $ty {
            fn content(input: ParseStream<'_>) -> Result<ParseBuffer> {
                let content;
                $mac!(content in input);
                Ok((content))
            }
            const OPEN: char = $open;
            const CLOSE: char = $close;
        }
    }
}
impl_group!(token::Paren, parenthesized, '(', ')');
impl_group!(token::Bracket, bracketed, '[', ']');
impl_group!(token::Brace, braced, '{', '}');

enum Output {
    Tokens(String),
    Var(Span, VarOutput),
}
enum VarOutput {
    Mut,
    PtrMut,
    Ident,
    Path,
    Ty,
    Expr(ExprPos),
}

type ExprVar = (Span, ExprPos);

#[derive(Default)]
struct SuggBuilder {
    output: Vec<Output>,
    next_string: String,
}
impl SuggBuilder {
    fn consume_token_and<T: Token>(&mut self, input: ParseStream<'_>, pre_write: impl FnOnce(&mut Self)) -> bool {
        input.parse::<T>().map_or(false, |t| {
            pre_write(self);
            t.write(&mut self.next_string);
            true
        })
    }

    fn require_token<T: Token>(&mut self, input: ParseStream<'_>, msg: &str) -> Result<()> {
        self.consume_token::<T>(input)
            .then(|| ())
            .ok_or_else(|| input.error(msg))
    }

    fn consume_token<T: Token>(&mut self, input: ParseStream<'_>) -> bool {
        self.consume_token_and::<T>(input, |_| ())
    }

    fn consume_token_space_prefixed<T: Token>(&mut self, input: ParseStream<'_>) -> bool {
        self.consume_token_and::<T>(input, |self_| self_.next_string.push(' '))
    }

    fn consume_op_token<T: Token>(&mut self, input: ParseStream<'_>, var: Option<ExprVar>, pos: ExprPos) -> bool {
        self.consume_token_and::<T>(input, |self_| self_.push_expr_var(var, pos))
    }

    fn consume_op_token_space_prefixed<T: Token>(
        &mut self,
        input: ParseStream<'_>,
        var: Option<ExprVar>,
        pos: ExprPos,
    ) -> bool {
        self.consume_token_and::<T>(input, |self_| {
            self_.push_expr_var(var, pos);
            self_.next_string.push(' ');
        })
    }

    fn consume_group<G: Group>(
        &mut self,
        input: ParseStream<'_>,
        pre_write: impl FnOnce(&mut Self),
        parse: impl FnOnce(&mut Self, ParseStream) -> Result<()>,
    ) -> Result<bool> {
        if let Ok(content) = G::content(input) {
            pre_write(self);
            self.next_string.push(G::OPEN);
            parse(self, &content)?;
            content.parse::<Nothing>()?;
            self.next_string.push(G::CLOSE);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn push_expr_var(&mut self, var: Option<ExprVar>, pos: ExprPos) {
        if let Some((span, pos2)) = var {
            self.push_var(span, VarOutput::Expr(cmp::max(pos, pos2)));
        }
    }

    fn require<T>(
        &mut self,
        input: ParseStream<'_>,
        f: impl FnOnce(&mut Self, ParseStream<'_>) -> Result<Option<T>>,
        msg: &str,
    ) -> Result<T> {
        match f(self, input) {
            Ok(Some(x)) => Ok(x),
            Ok(None) => Err(input.error(msg)),
            Err(e) => Err(e),
        }
    }

    fn parse_list<T>(
        &mut self,
        input: ParseStream<'_>,
        f: impl Fn(&mut Self, ParseStream<'_>) -> Result<Option<T>>,
    ) -> Result<()> {
        while f(self, input)?.is_some() && self.consume_token::<Token![,]>(input) {
            self.next_string.push(' ');
        }
        Ok(())
    }

    fn push_var(&mut self, span: Span, var: VarOutput) {
        if !self.next_string.is_empty() {
            self.output.push(Output::Tokens(self.next_string.clone()));
            self.next_string.clear();
        }
        self.output.push(Output::Var(span, var));
    }

    fn parse_ty(&mut self, input: ParseStream) -> Result<Option<()>> {
        if self.consume_token::<Token![&]>(input) {
            self.parse_mutability(input);
        } else if self.consume_token::<Token![*]>(input) {
            self.parse_ptr_mutability(input)?;
        } else {
            return self.parse_ty_body(input);
        }

        self.require(input, Self::parse_ty, "expected a type").map(Some)
    }

    #[allow(clippy::blocks_in_if_conditions)]
    fn parse_ty_body(&mut self, input: ParseStream) -> Result<Option<()>> {
        if let Some(ends_with_ident) = self.parse_path_root(input)? {
            if ends_with_ident && self.consume_token::<Token![<]>(input) {
                self.parse_list(input, Self::parse_ty)?;
                self.require_token::<Token![>]>(input, "expected `>`")?;
            }
        } else if self.consume_group::<token::Bracket>(
            input,
            |_| (),
            |self_, input| {
                self_.require(input, Self::parse_ty, "expected a type")?;
                if self_.consume_token::<Token![;]>(input) {
                    self_.next_string.push(' ');
                    self_.require(input, Self::parse_expr, "expected an expression")?;
                }
                Ok(())
            },
        )? || self.consume_group::<token::Paren>(
            input,
            |_| (),
            |self_, input| self_.parse_list(input, Self::parse_ty),
        )? {
            // Nothing to do
        } else if let Some(var) = parse_var(input) {
            if matches!(var.kind, VarKind::Ty | VarKind::Default) {
                self.push_var(var.span, VarOutput::Ty);
            } else {
                return Err(Error::new(var.span, "expected a `ty`, `ident` or `path` variable"));
            }
        } else {
            return Ok(None);
        }

        Ok(Some(()))
    }

    fn parse_path(&mut self, input: ParseStream) -> Result<bool> {
        let ends_with_ident = if self.consume_token::<Ident>(input) {
            true
        } else if self.consume_token::<Token![<]>(input) {
            self.parse_list(input, Self::parse_ty)?;
            self.require_token::<Token![>]>(input, "expected `>`")?;
            false
        } else if let Some(var) = parse_var(input) {
            if matches!(var.kind, VarKind::Default | VarKind::Path | VarKind::Ident) {
                self.push_var(var.span, VarOutput::Path);
            } else {
                return Err(Error::new(var.span, "expected a `path` or `ident` variable"));
            }
            true
        } else {
            return Err(input.error("expected an ident, generic arguments, or a `path` or `ident` variable"));
        };

        if self.consume_token::<Token![::]>(input) {
            self.parse_path(input)
        } else {
            Ok(ends_with_ident)
        }
    }

    fn parse_mutability(&mut self, input: ParseStream<'_>) {
        if self.consume_token::<Token![mut]>(input) {
            self.next_string.push(' ');
        } else if let Some(var) = parse_var_if(input, |var| var.kind == VarKind::Mut) {
            self.push_var(var.span, VarOutput::Mut);
        }
    }

    fn parse_ptr_mutability(&mut self, input: ParseStream<'_>) -> Result<()> {
        if self.consume_token::<Token![mut]>(input) || self.consume_token::<Token![const]>(input) {
            self.next_string.push(' ');
        } else if let Some(var) = parse_var(input) {
            if matches!(var.kind, VarKind::Mut | VarKind::Default) {
                self.push_var(var.span, VarOutput::PtrMut);
            } else {
                return Err(Error::new(var.span, "expected a `mut` variable"));
            }
        } else {
            return Err(input.error("expected `mut`, `const` or a `mut` variable"));
        }
        Ok(())
    }

    fn parse_expr(&mut self, input: ParseStream<'_>) -> Result<Option<ExprPrec>> {
        self.parse_expr_prefix(input, ExprPos::Closure, ExprPrec::Suffix)
    }

    fn parse_expr_prefix(&mut self, input: ParseStream<'_>, pos: ExprPos, prec: ExprPrec) -> Result<Option<ExprPrec>> {
        let (pos, prec) = if self.consume_token::<Token![&]>(input) {
            self.parse_mutability(input);
            (ExprPos::Prefix, prec.merge_with(ExprPrec::Prefix))
        } else if self.consume_token::<Token![*]>(input) || self.consume_token::<Token![!]>(input) {
            (ExprPos::Prefix, prec.merge_with(ExprPrec::Prefix))
        } else if self.consume_token::<Token![box]>(input) {
            self.next_string.push(' ');
            (ExprPos::Prefix, prec.merge_with(ExprPrec::Prefix))
        } else if self.consume_token::<Token![return]>(input) || self.consume_token::<Token![yield]>(input) {
            self.next_string.push(' ');
            (ExprPos::Closure, ExprPrec::Closure)
        } else if self.consume_token::<Token![|]>(input) {
            self.parse_list(input, Self::parse_closure_arg)?;
            self.require_token::<Token![|]>(input, "expected `|`")?;
            (ExprPos::Closure, ExprPrec::Closure)
        } else {
            return self.parse_expr_body(input, pos, prec);
        };

        self.require(
            input,
            |self_, input| self_.parse_expr_prefix(input, pos, prec),
            "expected an expression",
        )
        .map(Some)
    }

    fn parse_expr_body(&mut self, input: ParseStream<'_>, pos: ExprPos, prec: ExprPrec) -> Result<Option<ExprPrec>> {
        if self.consume_token::<Literal>(input)
            || self.consume_group::<token::Paren>(
                input,
                |_| (),
                |self_, input| self_.parse_list(input, Self::parse_expr).map(|_| ()),
            )?
            || self.parse_path_root(input)?.is_some()
        {
            // Nothing to do
        } else if let Some(var) = parse_var(input) {
            return if matches!(var.kind, VarKind::Expr | VarKind::Default) {
                self.parse_expr_suffix(input, prec, Some((var.span, pos))).map(Some)
            } else {
                Err(Error::new(var.span, "expected an `expr`, `ident` or `path` variable"))
            };
        } else {
            return Ok(None);
        };

        self.parse_expr_suffix(input, prec, None).map(Some)
    }

    fn parse_expr_suffix(&mut self, input: ParseStream<'_>, prec: ExprPrec, var: Option<ExprVar>) -> Result<ExprPrec> {
        let prec = if let Some(bin_op) = self.parse_bin_op(input, var) {
            return self.require(
                input,
                |self_, input| self_.parse_expr_prefix(input, bin_op.pos, prec.merge_with(bin_op.prec)),
                "expected an expression",
            );
        } else if self.consume_op_token::<Token![:]>(input, var, ExprPos::Cast)
            || self.consume_op_token_space_prefixed::<Token![as]>(input, var, ExprPos::Cast)
        {
            self.next_string.push(' ');
            self.require(input, Self::parse_ty, "expected a type")?;
            prec.merge_with(ExprPrec::Cast)
        } else if self.consume_op_token::<Token![?]>(input, var, ExprPos::Suffix)
            || self.consume_group::<token::Bracket>(
                input,
                |self_| self_.push_expr_var(var, ExprPos::Suffix),
                |self_, input| {
                    self_
                        .require(input, Self::parse_expr, "expected an expression")
                        .map(|_| ())
                },
            )?
            || self.consume_group::<token::Paren>(
                input,
                |self_| self_.push_expr_var(var, ExprPos::Callee),
                |self_, input| self_.parse_list(input, Self::parse_expr).map(|_| ()),
            )?
        {
            prec.merge_with(ExprPrec::Suffix)
        } else if self.consume_op_token::<Token![.]>(input, var, ExprPos::Suffix) {
            if self.consume_token::<Ident>(input) {
                // Nothing to do
            } else if let Some(var) = parse_var(input) {
                if matches!(var.kind, VarKind::Default | VarKind::Ident) {
                    self.push_var(var.span, VarOutput::Ident);
                } else {
                    return Err(Error::new(var.span, "expected an `ident` variable"));
                }
            } else {
                return Err(input.error("expected an identifier or an `ident` variable"));
            }
            if self.consume_token::<Token![::]>(input) {
                self.require_token::<Token![<]>(input, "expected `<`")?;
                self.parse_list(input, Self::parse_ty)?;
                self.require_token::<Token![>]>(input, "expected `>`")?;
            }
            prec.merge_with(ExprPrec::Field)
        } else {
            self.push_expr_var(var, ExprPos::Closure);
            return Ok(prec);
        };

        self.parse_expr_suffix(input, prec, None)
    }

    fn parse_closure_arg(&mut self, input: ParseStream<'_>) -> Result<Option<()>> {
        if self.consume_token::<Ident>(input) {
            // Nothing to do
        } else if let Some(var) = parse_var(input) {
            if matches!(var.kind, VarKind::Default | VarKind::Ident) {
                self.push_var(var.span, VarOutput::Ident);
            } else {
                return Err(Error::new(var.span, "expected an `ident` variable"));
            }
        } else {
            return Ok(None);
        }
        if self.consume_token::<Token![:]>(input) {
            self.require(input, Self::parse_ty, "expected a type")?;
        }
        Ok(Some(()))
    }

    fn parse_path_root(&mut self, input: ParseStream<'_>) -> Result<Option<bool>> {
        let parsed = if self.consume_token::<Ident>(input) {
            true
        } else if self.consume_token::<Token![<]>(input) {
            self.require(input, Self::parse_ty, "expected a type")?;
            if self.consume_token_space_prefixed::<Token![as]>(input) {
                self.next_string.push(' ');
                self.require(input, Self::parse_ty, "expected a type")?;
            }
            self.require_token::<Token![>]>(input, "expected `>`")?;
            self.require_token::<Token![::]>(input, "expected `::`")?;
            return Ok(Some(self.parse_path(input)?));
        } else if let Some(var) = parse_var_if(input, |var| matches!(var.kind, VarKind::Ident | VarKind::Path)) {
            self.push_var(var.span, VarOutput::Path);
            true
        } else {
            false
        };

        if self.consume_token::<Token![::]>(input) {
            self.parse_path(input).map(Some)
        } else {
            Ok(parsed.then(|| true))
        }
    }

    fn build(&self, prec: ExprPrec, args: &[TokenStream]) -> Result<TokenStream> {
        let mut args = args.iter();
        let mut body = TokenStream::new();
        for part in &self.output {
            match part {
                Output::Tokens(x) => body.extend(iter::once(quote!(sugg.push_str(#x);))),
                &Output::Var(span, ref kind) => {
                    let Some(arg) = args.next() else {
                        return Err(Error::new(span, "no argument given for variable"));
                    };
                    match kind {
                        VarOutput::Mut => body.extend(iter::once(quote!(match #arg {
                            rustc_ast::ast::Mutability::Mut => sugg.push_str("mut "),
                            rustc_ast::ast::Mutability::Not => (),
                        }))),
                        VarOutput::PtrMut => body.extend(iter::once(quote!(match #arg {
                            rustc_ast::ast::Mutability::Mut => sugg.push_str("mut "),
                            rustc_ast::ast::Mutability::Not => sugg.push_str("const "),
                        }))),
                        VarOutput::Expr(pos) => body.extend(iter::once(quote!(
                            sugg.push_str(&clippy_utils::_internal::snip(cx, #arg, #pos, ctxt, app));
                        ))),
                        _ => body.extend(iter::once(quote!(sugg.push_str(&format!("{}", #arg));))),
                    }
                },
            }
        }
        if !self.next_string.is_empty() {
            let s = &self.next_string;
            body.extend(iter::once(quote!(sugg.push_str(#s);)));
        }
        if prec == ExprPrec::Suffix {
            body.extend(iter::once(quote!(sugg)));
        } else {
            body.extend(iter::once(quote!(
                if clippy_utils::_internal::needs_parens(#prec, clippy_utils::_internal::expr_position(cx, e)) {
                    format!("({})", sugg)
                } else {
                    sugg
                }
            )));
        }
        Ok(
            quote!(|cx: &rustc_lint::LateContext<'_>, e: &rustc_hir::Expr<'_>, app: &mut rustc_errors::Applicability| {
                let ctxt = e.span.ctxt();
                let mut sugg = String::new();
                #body
            }),
        )
    }
}

fn split_args(input: ParseStream) -> Result<Vec<TokenStream>> {
    let mut args = Vec::new();

    loop {
        let mut arg = TokenStream::default();
        while !input.peek(Token![,]) {
            if let Ok(tt) = input.parse::<TokenTree>() {
                arg.extend(iter::once(tt));
            } else {
                if !arg.is_empty() {
                    args.push(arg);
                }
                return Ok(args);
            }
        }
        if arg.is_empty() {
            return Err(input.error("expected an argument"));
        }
        input.parse::<Token![,]>()?;
        args.push(arg);
    }
}

#[allow(clippy::module_name_repetitions)]
pub struct ExprSugg(pub TokenStream);
impl Parse for ExprSugg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut builder = SuggBuilder::default();
        let prec = builder.require(input, SuggBuilder::parse_expr, "expected an expression")?;
        let args = match input.parse::<Token![,]>() {
            Ok(_) => split_args(input)?,
            Err(_) => Vec::new(),
        };
        builder.build(prec, &args).map(Self)
    }
}
