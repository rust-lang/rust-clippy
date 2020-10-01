use core::iter::{self, FusedIterator};
use rustc_ast::{Expr, ExprKind, MacCall, Ty, TyKind};
use rustc_ast::ptr::P;
use rustc_span::symbol::Ident;

pub type IdentIter<'a> = Box<dyn Iterator<Item = Ident> + 'a>;

pub fn from_expr<'expr>(expr: &'expr Expr) -> IdentIter<'expr> {
    Box::new(ExprIdentIter::new(expr))
}

pub fn from_ty<'ty>(ty: &'ty Ty) -> IdentIter<'ty> {
    Box::new(TyIdentIter::new(ty))
}

struct ExprIdentIter<'expr> {
    expr: &'expr Expr,
    inner: Option<IdentIter<'expr>>,
    done: bool,
}

impl <'expr> ExprIdentIter<'expr> {
    fn new(expr: &'expr Expr) -> Self {
        Self {
            expr,
            inner: None,
            done: false,
        }
    }

    /// This is a convenience method to help with type inference.
    fn new_p(expr: &'expr P<Expr>) -> Self {
        Self::new(expr)
    }
}

impl <'expr> Iterator for ExprIdentIter<'expr> {
    type Item = Ident;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let inner_opt = &mut self.inner;

        if let Some(mut inner) = inner_opt.take() {
            let output = inner.next();

            if output.is_some() {
                *inner_opt = Some(inner);
                return output;
            }
        }

        macro_rules! set_and_call_next {
            ($iter: expr) => {{
                let mut p_iter = $iter;

                let next_item = p_iter.next();

                *inner_opt = Some(Box::new(p_iter));

                next_item
            }}
        }

        let output = match self.expr.kind {
            ExprKind::Lit(_)|ExprKind::Err => None,
            ExprKind::Path(_, ref path)
            | ExprKind::MacCall(MacCall{ ref path, ..}) => {
                set_and_call_next!(
                    path.segments.iter()
                        .map(|s| s.ident)
                )
            },
            ExprKind::Box(ref expr)
            | ExprKind::Unary(_, ref expr) => {
                set_and_call_next!(
                    ExprIdentIter::new(expr)
                )
            },
            ExprKind::Array(ref exprs)|ExprKind::Tup(ref exprs) => {
                set_and_call_next!(
                    exprs.iter()
                        .flat_map(ExprIdentIter::new_p)
                )
            },
            ExprKind::Call(ref func, ref args) => {
                set_and_call_next!(
                    ExprIdentIter::new(func)
                        .chain(
                            args.iter()
                                .flat_map(ExprIdentIter::new_p)
                        )
                )
            },
            ExprKind::MethodCall(ref method_name, ref args, _) => {
                set_and_call_next!(
                    iter::once(method_name.ident)
                        .chain(
                            args.iter()
                                .flat_map(ExprIdentIter::new_p)
                        )
                )
            },
            ExprKind::Binary(_, ref left, ref right) => {
                set_and_call_next!(
                    ExprIdentIter::new(left)
                        .chain(
                            ExprIdentIter::new(right)
                        )
                )
            },
            ExprKind::Cast(ref expr, ref ty)
            | ExprKind::Type(ref expr, ref ty) => {
                set_and_call_next!(
                    ExprIdentIter::new(expr)
                        .chain(
                            TyIdentIter::new(ty)
                        )
                )
            },
            _ => todo!(),
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'expr> FusedIterator for ExprIdentIter<'expr> {}

struct TyIdentIter<'ty> {
    ty: &'ty Ty,
    inner: Option<IdentIter<'ty>>,
    done: bool,
}

impl <'ty> TyIdentIter<'ty> {
    fn new(ty: &'ty Ty) -> Self {
        Self {
            ty,
            inner: None,
            done: false,
        }
    }
}

impl <'ty> Iterator for TyIdentIter<'ty> {
    type Item = Ident;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let inner_opt = &mut self.inner;

        if let Some(mut inner) = inner_opt.take() {
            let output = inner.next();

            if output.is_some() {
                *inner_opt = Some(inner);
                return output;
            }
        }

        macro_rules! set_and_call_next {
            ($iter: expr) => {{
                let mut p_iter = $iter;

                let next_item = p_iter.next();

                *inner_opt = Some(Box::new(p_iter));

                next_item
            }}
        }

        let output = match self.ty.kind {
            TyKind::Never
            | TyKind::Infer
            | TyKind::Err
            | TyKind::CVarArgs => None,
            TyKind::Slice(ref ty) => {
                set_and_call_next!(
                    TyIdentIter::new(ty)
                )
            },
            _ => todo!(),
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'expr> FusedIterator for TyIdentIter<'expr> {}