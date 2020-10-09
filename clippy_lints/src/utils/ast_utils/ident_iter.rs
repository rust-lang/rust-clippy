use core::iter::{self, FusedIterator};
use rustc_ast::{
    AssocItem,
    AssocItemKind,
    AttrKind,
    Attribute,
    Block,
    Expr,
    ExprKind,
    ForeignItem,
    ForeignItemKind,
    FnDecl,
    FnRetTy,
    Generics,
    GenericBound,
    GenericBounds,
    GenericParam,
    GenericParamKind,
    Item,
    ItemKind,
    Param,
    Pat,
    PatKind,
    Path,
    PolyTraitRef,
    MacCall,
    MacCallStmt,
    MutTy,
    Stmt,
    StmtKind,
    Local,
    Ty,
    TyKind,
    UseTree,
    UseTreeKind,
    VariantData,
    Visibility,
    VisibilityKind,
    WherePredicate,
};
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
                    path_iter(path)
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
            ExprKind::Let(ref pat, ref expr) => {
                set_and_call_next!(
                    PatIdentIter::new(pat)
                        .chain(
                            ExprIdentIter::new(expr)
                        )
                )
            },
            ExprKind::If(ref condition_expr, ref block, Some(ref else_expr)) => {
                set_and_call_next!(
                    ExprIdentIter::new(condition_expr)
                        .chain(block_iter(block))
                        .chain(ExprIdentIter::new(else_expr))
                )
            },
            ExprKind::While(ref condition_expr, ref block, None)
            | ExprKind::If(ref condition_expr, ref block, None) => {
                set_and_call_next!(
                    ExprIdentIter::new(condition_expr)
                        .chain(block_iter(block))
                )
            },
            ExprKind::While(ref expr, ref block, Some(ref label)) => {
                set_and_call_next!(
                    ExprIdentIter::new(expr)
                        .chain(block_iter(block))
                        .chain(iter::once(label.ident))
                )
            },
            ExprKind::ForLoop(ref pat, ref expr, ref block, Some(ref label)) => {
                set_and_call_next!(
                    PatIdentIter::new(pat)
                        .chain(ExprIdentIter::new(expr))
                        .chain(block_iter(block))
                        .chain(iter::once(label.ident))
                )
            },
            ExprKind::ForLoop(ref pat, ref expr, ref block, None) => {
                set_and_call_next!(
                    PatIdentIter::new(pat)
                        .chain(ExprIdentIter::new(expr))
                        .chain(block_iter(block))
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

    /// This is a convenience method to help with type inference.
    fn new_p(ty: &'ty P<Ty>) -> Self {
        Self::new(ty)
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
            | TyKind::CVarArgs
            | TyKind::ImplicitSelf => None,
            TyKind::Rptr(Some(ref lifetime), MutTy{ ref ty, .. }) => {
                set_and_call_next!(
                    iter::once(lifetime.ident)
                        .chain(
                            TyIdentIter::new(ty)
                        )
                )
            },
            TyKind::Slice(ref ty)
            | TyKind::Ptr(MutTy{ ref ty, .. })
            | TyKind::Rptr(None, MutTy{ ref ty, .. })
            | TyKind::Paren(ref ty) => {
                set_and_call_next!(
                    TyIdentIter::new(ty)
                )
            },
            TyKind::Array(ref ty, ref anon_const) => {
                set_and_call_next!(
                    TyIdentIter::new(ty)
                        .chain(ExprIdentIter::new(&anon_const.value))
                )
            },
            TyKind::Tup(ref ty_vec) => {
                set_and_call_next!(
                    ty_vec.iter()
                        .flat_map(TyIdentIter::new_p)
                )
            },
            TyKind::Typeof(ref anon_const) => {
                set_and_call_next!(
                    ExprIdentIter::new(&anon_const.value)
                )
            },
            TyKind::Path(_, ref path)
            | TyKind::MacCall(MacCall{ ref path, ..}) => {
                set_and_call_next!(
                    path_iter(path)
                )
            },
            TyKind::TraitObject(ref bounds, _)
            | TyKind::ImplTrait(_, ref bounds) => {
                set_and_call_next!(generic_bounds_iter(bounds))
            },
            TyKind::BareFn(ref bare_fn_ty) => {
                set_and_call_next!(
                    bare_fn_ty.generic_params.iter()
                        .flat_map(generic_param_iter)
                        .chain(
                            fn_decl_iter(&bare_fn_ty.decl)
                        )
                )
            },
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'ty> FusedIterator for TyIdentIter<'ty> {}

struct PatIdentIter<'pat> {
    pat: &'pat Pat,
    inner: Option<IdentIter<'pat>>,
    done: bool,
}

impl <'pat> PatIdentIter<'pat> {
    fn new(pat: &'pat Pat) -> Self {
        Self {
            pat,
            inner: None,
            done: false,
        }
    }

    /// This is a convenience method to help with type inference.
    fn new_p(pat: &'pat P<Pat>) -> Self {
        Self::new(pat)
    }
}

impl <'pat> Iterator for PatIdentIter<'pat> {
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

        let output = match self.pat.kind {
            PatKind::Wild 
            | PatKind::Rest
            | PatKind::Range(None, None, _) => None,
            PatKind::Ident(_, ident, None) => {
                self.done = true;
                Some(ident)
            }
            PatKind::Ident(_, ident, Some(ref pat)) => {
                set_and_call_next!(
                    iter::once(ident)
                        .chain(PatIdentIter::new(pat))
                )
            },
            PatKind::Struct(ref path, ref field_pats, _) => {
                set_and_call_next!(
                    path_iter(path)
                        .chain(
                            field_pats.iter()
                                .flat_map(|f_p|
                                    f_p.attrs
                                        .iter()
                                        .flat_map(attribute_iter)
                                        .chain(iter::once(f_p.ident))
                                        .chain(PatIdentIter::new(&f_p.pat))
                                )
                        )
                )
            },
            PatKind::TupleStruct(ref path, ref pats) => {
                set_and_call_next!(
                    path_iter(path)
                        .chain(
                            pats.iter()
                                .flat_map(
                                    PatIdentIter::new_p
                                )
                        )
                )
            },
            PatKind::Or(ref pats)
            | PatKind::Tuple(ref pats)
            | PatKind::Slice(ref pats) => {
                set_and_call_next!(
                    pats.iter()
                        .flat_map(
                            PatIdentIter::new_p
                        )
                )
            },
            PatKind::Box(ref pat)
            | PatKind::Ref(ref pat, _)
            | PatKind::Paren(ref pat) => {
                set_and_call_next!(
                    PatIdentIter::new(pat)
                )
            },
            PatKind::Path(_, ref path)
            | PatKind::MacCall(MacCall{ ref path, ..}) => {
                set_and_call_next!(
                    path_iter(path)
                )
            },
            PatKind::Lit(ref expr)
            | PatKind::Range(Some(ref expr), None, _)
            | PatKind::Range(None, Some(ref expr), _) => {
                set_and_call_next!(
                    ExprIdentIter::new(expr)
                )
            },
            PatKind::Range(Some(ref expr1), Some(ref expr2), _) => {
                set_and_call_next!(
                    ExprIdentIter::new(expr1)
                    .chain(
                        ExprIdentIter::new(expr2)
                    )
                )
            },
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'pat> FusedIterator for PatIdentIter<'pat> {}

struct StmtIdentIter<'stmt> {
    stmt: &'stmt Stmt,
    inner: Option<IdentIter<'stmt>>,
    done: bool,
}

impl <'stmt> StmtIdentIter<'stmt> {
    fn new(stmt: &'stmt Stmt) -> Self {
        Self {
            stmt,
            inner: None,
            done: false,
        }
    }
}

impl <'stmt> Iterator for StmtIdentIter<'stmt> {
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

        let output = match self.stmt.kind {
            StmtKind::Empty => None,
            StmtKind::Expr(ref expr)
            | StmtKind::Semi(ref expr) => {
                set_and_call_next!(
                    ExprIdentIter::new(expr)
                )
            },
            StmtKind::Local(ref local) => {
                let l: &Local = local;
                match l {
                    Local{
                        ref attrs,
                        ref pat,
                        ty: Some(ref ty),
                        init: Some(ref expr),
                        ..
                    } => {
                        set_and_call_next!(
                            attrs.iter()
                                .flat_map(attribute_iter)
                                .chain(PatIdentIter::new(pat))
                                .chain(TyIdentIter::new(ty))
                                .chain(ExprIdentIter::new(expr))
                        )
                    },
                    Local{
                        ref attrs,
                        ref pat,
                        ty: Some(ref ty),
                        init: None,
                        ..
                    } => {
                        set_and_call_next!(
                            attrs.iter()
                                .flat_map(attribute_iter)
                                .chain(PatIdentIter::new(pat))
                                .chain(TyIdentIter::new(ty))
                        )
                    },
                    Local{
                        ref attrs,
                        ref pat,
                        ty: None,
                        init: Some(ref expr),
                        ..
                    } => {
                        set_and_call_next!(
                            attrs.iter()
                                .flat_map(attribute_iter)
                                .chain(PatIdentIter::new(pat))
                                .chain(ExprIdentIter::new(expr))
                        )
                    },
                    Local{
                        ref attrs,
                        ref pat,
                        ty: None,
                        init: None,
                        ..
                    } => {
                        set_and_call_next!(
                            attrs.iter()
                                .flat_map(attribute_iter)
                                .chain(PatIdentIter::new(pat))
                        )
                    }
                }
            },
            StmtKind::MacCall(ref mac_call_stmt) => {
                let mcs: &MacCallStmt = &mac_call_stmt;
                let MacCallStmt{
                    ref attrs,
                    mac: MacCall{ ref path, ..},
                    ..
                } = mcs;
                set_and_call_next!(
                    attrs.iter()
                        .flat_map(attribute_iter)
                        .chain(path_iter(path))
                )
            },
            StmtKind::Item(ref item) => {
                set_and_call_next!(
                    ItemIdentIter::new(item)
                )
            },
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'stmt> FusedIterator for StmtIdentIter<'stmt> {}

struct ItemIdentIter<'item> {
    item: &'item Item,
    inner: Option<IdentIter<'item>>,
    done: bool,
}

impl <'item> ItemIdentIter<'item> {
    fn new(item: &'item Item) -> Self {
        Self {
            item,
            inner: None,
            done: false,
        }
    }

    /// This is a convenience method to help with type inference.
    fn new_p(item: &'item P<Item>) -> Self {
        Self::new(item)
    }
}

impl <'item> Iterator for ItemIdentIter<'item> {
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

        macro_rules! set_and_call_next_with_own_idents {
            () => {
                set_and_call_next_with_own_idents!(
                    inner visibility_iter(&self.item.vis)
                        .chain(iter::once(self.item.ident))
                )
            };
            ($iter: expr) => {
                set_and_call_next_with_own_idents!(
                    inner $iter.chain(visibility_iter(&self.item.vis))
                        .chain(iter::once(self.item.ident))
                )
            };
            (inner $iter: expr) => {{
                let mut p_iter = $iter;

                let next_item = p_iter.next();

                *inner_opt = Some(Box::new(p_iter));

                next_item
            }}
        }

        let output = match self.item.kind {
            ItemKind::ExternCrate(_)
            | ItemKind::GlobalAsm(_)
            | ItemKind::MacroDef(_) => set_and_call_next_with_own_idents!(),
            ItemKind::Use(ref use_tree) => {
                set_and_call_next_with_own_idents!(
                    use_tree_iter(use_tree)
                )
            },
            ItemKind::Static(ref ty, _, None)
            | ItemKind::Const(_, ref ty, None) => {
                set_and_call_next_with_own_idents!(
                    TyIdentIter::new(ty)
                )
            },
            ItemKind::Static(ref ty, _, Some(ref expr))
            | ItemKind::Const(_, ref ty, Some(ref expr)) => {
                set_and_call_next_with_own_idents!(
                    TyIdentIter::new(ty)
                        .chain(ExprIdentIter::new(expr))
                )
            },
            ItemKind::Fn(_, ref fn_sig, ref generics, None) => {
                set_and_call_next_with_own_idents!(
                    fn_decl_iter(&fn_sig.decl)
                        .chain(generics_iter(generics))
                )
            },
            ItemKind::Fn(_, ref fn_sig, ref generics, Some(ref block)) => {
                set_and_call_next_with_own_idents!(
                    fn_decl_iter(&fn_sig.decl)
                        .chain(generics_iter(generics))
                        .chain(block_iter(block))
                )
            },
            ItemKind::Mod(ref mod_) => {
                set_and_call_next_with_own_idents!(
                    mod_.items.iter()
                        .flat_map(ItemIdentIter::new_p)
                )
            },
            ItemKind::ForeignMod(ref foreign_mod) => {
                set_and_call_next_with_own_idents!(
                    foreign_mod.items.iter()
                        .flat_map(ForeignItemIdentIter::new_p)
                )
            },
            ItemKind::TyAlias(_, ref generics, ref bounds, None)
            | ItemKind::TraitAlias(ref generics, ref bounds) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(generic_bounds_iter(bounds))
                )
            },
            ItemKind::TyAlias(_, ref generics, ref bounds, Some(ref ty)) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(generic_bounds_iter(bounds))
                        .chain(TyIdentIter::new(ty))
                )
            },
            ItemKind::Enum(ref enum_def, ref generics) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(
                            enum_def.variants.iter()
                                .flat_map(|variant| {
                                    variant.attrs.iter()
                                        .flat_map(attribute_iter)
                                        .chain(visibility_iter(&variant.vis))
                                        .chain(iter::once(variant.ident))
                                        .chain(variant_data_iter(&variant.data))
                                        .chain(
                                            variant.disr_expr
                                                .as_ref()
                                                .into_iter()
                                                .flat_map(|a_c| 
                                                    ExprIdentIter::new(&a_c.value)
                                                )
                                        )
                                })
                        )
                )
            },
            ItemKind::Struct(ref variant_data, ref generics)
            | ItemKind::Union(ref variant_data, ref generics) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(variant_data_iter(variant_data))
                )
            },
            ItemKind::Trait(_, _, ref generics, ref bounds, ref assoc_items) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(generic_bounds_iter(bounds))
                        .chain(
                            assoc_items.iter()
                                .flat_map(AssocItemIdentIter::new_p)
                        )
                )
            },
            ItemKind::MacCall(MacCall{ ref path, ..}) => {
                set_and_call_next_with_own_idents!(
                    path_iter(path)
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

impl <'item> FusedIterator for ItemIdentIter<'item> {}

struct ForeignItemIdentIter<'item> {
    item: &'item ForeignItem,
    inner: Option<IdentIter<'item>>,
    done: bool,
}

impl <'item> ForeignItemIdentIter<'item> {
    fn new(item: &'item ForeignItem) -> Self {
        Self {
            item,
            inner: None,
            done: false,
        }
    }

    fn new_p(item: &'item P<ForeignItem>) -> Self {
        Self::new(item)
    }
}

impl <'item> Iterator for ForeignItemIdentIter<'item> {
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

        macro_rules! set_and_call_next_with_own_idents {
            () => {
                set_and_call_next_with_own_idents!(
                    inner visibility_iter(&self.item.vis)
                        .chain(iter::once(self.item.ident))
                )
            };
            ($iter: expr) => {
                set_and_call_next_with_own_idents!(
                    inner $iter.chain(visibility_iter(&self.item.vis))
                        .chain(iter::once(self.item.ident))
                )
            };
            (inner $iter: expr) => {{
                let mut p_iter = $iter;

                let next_item = p_iter.next();

                *inner_opt = Some(Box::new(p_iter));

                next_item
            }}
        }

        let output = match self.item.kind {
            ForeignItemKind::Static(ref ty, _, None) => {
                set_and_call_next_with_own_idents!(
                    TyIdentIter::new(ty)
                )
            },
            ForeignItemKind::Static(ref ty, _, Some(ref expr)) => {
                set_and_call_next_with_own_idents!(
                    TyIdentIter::new(ty)
                        .chain(ExprIdentIter::new(expr))
                )
            },
            ForeignItemKind::Fn(_, ref fn_sig, ref generics, None) => {
                set_and_call_next_with_own_idents!(
                    fn_decl_iter(&fn_sig.decl)
                        .chain(generics_iter(generics))
                )
            },
            ForeignItemKind::Fn(_, ref fn_sig, ref generics, Some(ref block)) => {
                set_and_call_next_with_own_idents!(
                    fn_decl_iter(&fn_sig.decl)
                        .chain(generics_iter(generics))
                        .chain(block_iter(block))
                )
            },
            ForeignItemKind::TyAlias(_, ref generics, ref bounds, None) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(generic_bounds_iter(bounds))
                )
            },
            ForeignItemKind::TyAlias(_, ref generics, ref bounds, Some(ref ty)) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(generic_bounds_iter(bounds))
                        .chain(TyIdentIter::new(ty))
                )
            },
            ForeignItemKind::MacCall(MacCall{ ref path, ..}) => {
                set_and_call_next_with_own_idents!(
                    path_iter(path)
                )
            },
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'item> FusedIterator for ForeignItemIdentIter<'item> {}

struct AssocItemIdentIter<'item> {
    item: &'item AssocItem,
    inner: Option<IdentIter<'item>>,
    done: bool,
}

impl <'item> AssocItemIdentIter<'item> {
    fn new(item: &'item AssocItem) -> Self {
        Self {
            item,
            inner: None,
            done: false,
        }
    }

    fn new_p(item: &'item P<AssocItem>) -> Self {
        Self::new(item)
    }
}

impl <'item> Iterator for AssocItemIdentIter<'item> {
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

        macro_rules! set_and_call_next_with_own_idents {
            () => {
                set_and_call_next_with_own_idents!(
                    inner visibility_iter(&self.item.vis)
                        .chain(iter::once(self.item.ident))
                )
            };
            ($iter: expr) => {
                set_and_call_next_with_own_idents!(
                    inner $iter.chain(visibility_iter(&self.item.vis))
                        .chain(iter::once(self.item.ident))
                )
            };
            (inner $iter: expr) => {{
                let mut p_iter = $iter;

                let next_item = p_iter.next();

                *inner_opt = Some(Box::new(p_iter));

                next_item
            }}
        }

        let output = match self.item.kind {
            AssocItemKind::Const(_, ref ty, None) => {
                set_and_call_next_with_own_idents!(
                    TyIdentIter::new(ty)
                )
            },
            AssocItemKind::Const(_, ref ty, Some(ref expr)) => {
                set_and_call_next_with_own_idents!(
                    TyIdentIter::new(ty)
                        .chain(ExprIdentIter::new(expr))
                )
            },
            AssocItemKind::Fn(_, ref fn_sig, ref generics, None) => {
                set_and_call_next_with_own_idents!(
                    fn_decl_iter(&fn_sig.decl)
                        .chain(generics_iter(generics))
                )
            },
            AssocItemKind::Fn(_, ref fn_sig, ref generics, Some(ref block)) => {
                set_and_call_next_with_own_idents!(
                    fn_decl_iter(&fn_sig.decl)
                        .chain(generics_iter(generics))
                        .chain(block_iter(block))
                )
            },
            AssocItemKind::TyAlias(_, ref generics, ref bounds, None) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(generic_bounds_iter(bounds))
                )
            },
            AssocItemKind::TyAlias(_, ref generics, ref bounds, Some(ref ty)) => {
                set_and_call_next_with_own_idents!(
                    generics_iter(generics)
                        .chain(generic_bounds_iter(bounds))
                        .chain(TyIdentIter::new(ty))
                )
            },
            AssocItemKind::MacCall(MacCall{ ref path, ..}) => {
                set_and_call_next_with_own_idents!(
                    path_iter(path)
                )
            },
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'item> FusedIterator for AssocItemIdentIter<'item> {}

struct GenericBoundIdentIter<'bound> {
    bound: &'bound GenericBound,
    inner: Option<IdentIter<'bound>>,
    done: bool,
}

impl <'bound> GenericBoundIdentIter<'bound> {
    fn new(bound: &'bound GenericBound) -> Self {
        Self {
            bound,
            inner: None,
            done: false,
        }
    }
}

impl <'bound> Iterator for GenericBoundIdentIter<'bound> {
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

        let output = match self.bound {
            GenericBound::Outlives(ref lifetime) => {
                set_and_call_next!(
                    iter::once(lifetime.ident)
                )
            },
            GenericBound::Trait(
                PolyTraitRef{
                    ref bound_generic_params,
                    ref trait_ref,
                    span: _,
                },
                _
            ) => {
                set_and_call_next!(
                    bound_generic_params.iter()
                        .flat_map(generic_param_iter)
                        .chain(
                            path_iter(&trait_ref.path)
                        )
                )
            },
        };

        if output.is_none() {
            self.done = true;
        }

        output
    }
}

impl <'bound> FusedIterator for GenericBoundIdentIter<'bound> {}

fn generic_param_iter(param: &GenericParam) -> IdentIter<'_> {
    Box::new(
        param.attrs
            .iter()
            .flat_map(attribute_iter)
            .chain(iter::once(param.ident))
            .chain({
                let i_i: IdentIter<'_> = match param.kind {
                    GenericParamKind::Lifetime
                    | GenericParamKind::Type { default: None } => {
                        Box::new(iter::empty())
                    },
                    GenericParamKind::Type { default: Some(ref ty), }
                    | GenericParamKind::Const { ref ty, .. } => {
                        Box::new(TyIdentIter::new(ty))
                    },
                };

                i_i
            })
            .chain(generic_bounds_iter(&param.bounds))
    )
}

fn generic_bounds_iter(bounds: &GenericBounds) -> impl Iterator<Item = Ident> + '_ {
    bounds.iter()
        .flat_map(GenericBoundIdentIter::new)
}

fn attribute_iter(attribute: &Attribute) -> IdentIter<'_> {
    match attribute.kind {
        AttrKind::Normal(ref attr_item) => Box::new(path_iter(&attr_item.path)),
        AttrKind::DocComment(_, _) => Box::new(iter::empty()),
    }
}

fn path_iter(path: &Path) -> impl Iterator<Item = Ident> + '_ {
    path.segments.iter()
        .map(|s| s.ident)
}

fn fn_decl_iter(fn_decl: &FnDecl) -> impl Iterator<Item = Ident> + '_ {
    fn_decl.inputs
        .iter()
        .flat_map(param_iter)
        .chain({
            let i_i: IdentIter<'_> = match fn_decl.output {
                FnRetTy::Default(_) => {
                    Box::new(iter::empty())
                },
                FnRetTy::Ty(ref ty) => {
                    Box::new(TyIdentIter::new(ty))
                }
            };

            i_i
        })
}

fn param_iter(param: &Param) -> impl Iterator<Item = Ident> + '_ {
    param.attrs.iter()
        .flat_map(attribute_iter)
        .chain(
            TyIdentIter::new(&param.ty)
        )
        .chain(
            PatIdentIter::new(&param.pat)
        )
}

fn block_iter(block: &Block) -> impl Iterator<Item = Ident> + '_ {
    block.stmts
        .iter()
        .flat_map(StmtIdentIter::new)
}

fn use_tree_iter(use_tree: &UseTree) -> IdentIter<'_> {
    match use_tree.kind {
        UseTreeKind::Simple(Some(ident), _, _) => {
            Box::new(
                path_iter(&use_tree.prefix)
                    .chain(iter::once(ident))
            )
        },
        UseTreeKind::Nested(ref nested_trees) => {
            Box::new(
                path_iter(&use_tree.prefix)
                    .chain(
                        nested_trees.iter()
                            .flat_map(|(ref u_t, _)| use_tree_iter(u_t))
                    )
            )
        },
        UseTreeKind::Glob
        | UseTreeKind::Simple(None, _, _) => {
            Box::new(path_iter(&use_tree.prefix))
        },
    }
}

fn generics_iter(generics: &Generics) -> impl Iterator<Item = Ident> + '_ {
    generics.params
        .iter()
        .flat_map(generic_param_iter)
        .chain(
            generics.where_clause
                .predicates
                .iter()
                .flat_map(|pred| {
                    let i_i: IdentIter<'_> = match pred {
                        WherePredicate::BoundPredicate(ref bound) => {
                            Box::new(
                                bound.bound_generic_params.iter()
                                    .flat_map(generic_param_iter)
                                    .chain(TyIdentIter::new(&bound.bounded_ty))
                                    .chain(generic_bounds_iter(&bound.bounds))
                            )
                        },
                        WherePredicate::RegionPredicate(ref region) => {
                            Box::new(
                                iter::once(region.lifetime.ident)
                                    .chain(generic_bounds_iter(&region.bounds))
                            )
                        },
                        WherePredicate::EqPredicate(ref eq) => {
                            Box::new(
                                TyIdentIter::new(&eq.lhs_ty)
                                    .chain(TyIdentIter::new(&eq.rhs_ty))
                            )
                        },
                    };

                    i_i
                })
        )
}

fn variant_data_iter(data: &VariantData) -> IdentIter<'_> {
    match data {
        VariantData::Struct(ref struct_fields, _)
        | VariantData::Tuple(ref struct_fields, _) => {
            Box::new(
                struct_fields.iter()
                    .flat_map(|s_f| {
                        s_f.attrs.iter()
                            .flat_map(attribute_iter)
                            .chain(visibility_iter(&s_f.vis))
                            .chain(s_f.ident.into_iter())
                            .chain(TyIdentIter::new(&s_f.ty))
                    })
            )
        },
        VariantData::Unit(_) => Box::new(iter::empty()),
    }
}

fn visibility_iter(vis: &Visibility) -> IdentIter<'_> {
    match vis.kind {
        VisibilityKind::Public
        | VisibilityKind::Inherited
        | VisibilityKind::Crate(_) => {
            Box::new(iter::empty())
        },
        VisibilityKind::Restricted {
            ref path,
            ..
        } => {
            Box::new(path_iter(path))
        },
    }
}