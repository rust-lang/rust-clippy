//! Utilities for manipulating and extracting information from `rustc_ast::ast`.
//!
//! - The `eq_foobar` functions test for semantic equality but ignores `NodeId`s and `Span`s.

#![allow(clippy::enum_glob_use, clippy::wildcard_imports)]
#![warn(missing_docs)]

use crate::{both, over};
use rustc_ast::attr::data_structures::CfgEntry;
use rustc_ast::{self as ast, HasAttrs, *};
use rustc_span::sym;
use rustc_span::symbol::Ident;
use std::mem;

/// Utilities for iterating over identifiers in AST nodes.
pub mod ident_iter;
pub use ident_iter::IdentIter;

/// Checks if a binary operator is useless when both operands are the same expression.
pub fn is_useless_with_eq_exprs(kind: BinOpKind) -> bool {
    use BinOpKind::*;
    matches!(
        kind,
        Sub | Div | Eq | Lt | Le | Gt | Ge | Ne | And | Or | BitXor | BitAnd | BitOr
    )
}

/// Checks if each element in the first slice is contained within the latter as per `eq_fn`.
pub fn unordered_over<X, Y>(left: &[X], right: &[Y], mut eq_fn: impl FnMut(&X, &Y) -> bool) -> bool {
    left.len() == right.len() && left.iter().all(|l| right.iter().any(|r| eq_fn(l, r)))
}

/// Checks if `l` and `r` have the same name.
pub fn eq_id(l: Ident, r: Ident) -> bool {
    l.name == r.name
}

/// Checks if two patterns are equivalent, ignoring `NodeId`s and `Span`s.
pub fn eq_pat(l: &Pat, r: &Pat) -> bool {
    use PatKind::*;
    match (&l.kind, &r.kind) {
        (Missing, _) | (_, Missing) => unreachable!(),
        (Paren(l), _) => eq_pat(l, r),
        (_, Paren(r)) => eq_pat(l, r),
        (Wild, Wild) | (Rest, Rest) => true,
        (Expr(l), Expr(r)) => eq_expr(l, r),
        (Ident(b1, i1, s1), Ident(b2, i2, s2)) => {
            b1 == b2 && eq_id(*i1, *i2) && both(s1.as_deref(), s2.as_deref(), eq_pat)
        },
        (Range(lf, lt, le), Range(rf, rt, re)) => {
            eq_expr_opt(lf.as_deref(), rf.as_deref())
                && eq_expr_opt(lt.as_deref(), rt.as_deref())
                && eq_range_end(le.node, re.node)
        },
        (Ref(l, l_pin, l_mut), Ref(r, r_pin, r_mut)) => l_pin == r_pin && l_mut == r_mut && eq_pat(l, r),
        (Tuple(l), Tuple(r)) | (Slice(l), Slice(r)) => over(l, r, eq_pat),
        (Path(lq, lp), Path(rq, rp)) => both(lq.as_deref(), rq.as_deref(), eq_qself) && eq_path(lp, rp),
        (TupleStruct(lqself, lp, lfs), TupleStruct(rqself, rp, rfs)) => {
            eq_maybe_qself(lqself.as_deref(), rqself.as_deref()) && eq_path(lp, rp) && over(lfs, rfs, eq_pat)
        },
        (Struct(lqself, lp, lfs, lr), Struct(rqself, rp, rfs, rr)) => {
            lr == rr
                && eq_maybe_qself(lqself.as_deref(), rqself.as_deref())
                && eq_path(lp, rp)
                && unordered_over(lfs, rfs, eq_field_pat)
        },
        (Or(ls), Or(rs)) => unordered_over(ls, rs, eq_pat),
        (MacCall(l), MacCall(r)) => eq_mac_call(l, r),
        _ => false,
    }
}

/// Checks if two range ends are equivalent: both are `..` (excluded), or both are `..=` (included).
fn eq_range_end(l: RangeEnd, r: RangeEnd) -> bool {
    match (l, r) {
        (RangeEnd::Excluded, RangeEnd::Excluded) => true,
        (RangeEnd::Included(l), RangeEnd::Included(r)) => {
            matches!(l, RangeSyntax::DotDotEq) == matches!(r, RangeSyntax::DotDotEq)
        },
        _ => false,
    }
}

/// Checks if two pattern fields are equivalent, comparing their placeholder status, identifier, pattern, and
/// attributes.
pub fn eq_field_pat(l: &PatField, r: &PatField) -> bool {
    l.is_placeholder == r.is_placeholder
        && eq_id(l.ident, r.ident)
        && eq_pat(&l.pat, &r.pat)
        && over(&l.attrs, &r.attrs, eq_attr)
}

/// Checks if two qualified self types are equivalent, comparing their position and type.
fn eq_qself(l: &QSelf, r: &QSelf) -> bool {
    l.position == r.position && eq_ty(&l.ty, &r.ty)
}

/// Checks if two optional qualified self types are equivalent: both `None`, or both `Some` and equivalent.
pub fn eq_maybe_qself(l: Option<&QSelf>, r: Option<&QSelf>) -> bool {
    match (l, r) {
        (Some(l), Some(r)) => eq_qself(l, r),
        (None, None) => true,
        _ => false,
    }
}

/// Checks if two paths are equivalent, comparing their segments.
pub fn eq_path(l: &Path, r: &Path) -> bool {
    over(&l.segments, &r.segments, eq_path_seg)
}

/// Checks if two path segments are equivalent, comparing their identifier and generic arguments.
fn eq_path_seg(l: &PathSegment, r: &PathSegment) -> bool {
    eq_id(l.ident, r.ident) && both(l.args.as_ref(), r.args.as_ref(), |l, r| eq_generic_args(l, r))
}

/// Checks if two sets of generic arguments are equivalent: both angle-bracketed with equivalent args, or both
/// parenthesized with equivalent inputs and return type.
fn eq_generic_args(l: &GenericArgs, r: &GenericArgs) -> bool {
    match (l, r) {
        (AngleBracketed(l), AngleBracketed(r)) => over(&l.args, &r.args, eq_angle_arg),
        (Parenthesized(l), Parenthesized(r)) => {
            over(&l.inputs, &r.inputs, eq_param) && eq_fn_ret_ty(&l.output, &r.output)
        },
        _ => false,
    }
}

/// Checks if two angle-bracketed arguments are equivalent: both are type arguments with equivalent generics, or both
/// are constraints with equivalent associated items.
fn eq_angle_arg(l: &AngleBracketedArg, r: &AngleBracketedArg) -> bool {
    match (l, r) {
        (AngleBracketedArg::Arg(l), AngleBracketedArg::Arg(r)) => eq_generic_arg(l, r),
        (AngleBracketedArg::Constraint(l), AngleBracketedArg::Constraint(r)) => eq_assoc_item_constraint(l, r),
        _ => false,
    }
}

/// Checks if two generic arguments are equivalent: both lifetimes with the same name, both types that are equivalent,
/// or both constants with equivalent values.
fn eq_generic_arg(l: &GenericArg, r: &GenericArg) -> bool {
    match (l, r) {
        (GenericArg::Lifetime(l), GenericArg::Lifetime(r)) => eq_id(l.ident, r.ident),
        (GenericArg::Type(l), GenericArg::Type(r)) => eq_ty(l, r),
        (GenericArg::Const(l), GenericArg::Const(r)) => eq_expr(&l.value, &r.value),
        _ => false,
    }
}

/// Checks if two optional expressions are equivalent: both `None`, or both `Some` and equivalent.
fn eq_expr_opt(l: Option<&Expr>, r: Option<&Expr>) -> bool {
    both(l, r, eq_expr)
}

/// Checks if two struct rest expressions are equivalent: both are a `..base` with equivalent expressions, both are
/// `..`, or both are absent.
fn eq_struct_rest(l: &StructRest, r: &StructRest) -> bool {
    match (l, r) {
        (StructRest::Base(lb), StructRest::Base(rb)) => eq_expr(lb, rb),
        (StructRest::Rest(_), StructRest::Rest(_)) | (StructRest::None, StructRest::None) => true,
        _ => false,
    }
}

#[expect(clippy::too_many_lines, reason = "big match statement")]
/// Checks if two expressions are equivalent, ignoring `NodeId`s and `Span`s.
///
/// Parentheses are ignored, and method calls are compared by name, receiver, and arguments.
fn eq_expr(l: &Expr, r: &Expr) -> bool {
    use ExprKind::*;
    if !over(&l.attrs, &r.attrs, eq_attr) {
        return false;
    }
    match (&l.kind, &r.kind) {
        (Paren(l), _) => eq_expr(l, r),
        (_, Paren(r)) => eq_expr(l, r),
        (Err(_), Err(_)) => true,
        (Dummy, _) | (_, Dummy) => unreachable!("comparing `ExprKind::Dummy`"),
        (Try(l), Try(r)) | (Await(l, _), Await(r, _)) => eq_expr(l, r),
        (Array(l), Array(r)) => over(l, r, |l, r| eq_expr(l, r)),
        (Tup(l), Tup(r)) => over(l, r, |l, r| eq_expr(l, r)),
        (Repeat(le, ls), Repeat(re, rs)) => eq_expr(le, re) && eq_expr(&ls.value, &rs.value),
        (Call(lc, la), Call(rc, ra)) => eq_expr(lc, rc) && over(la, ra, |l, r| eq_expr(l, r)),
        (
            MethodCall(ast::MethodCall {
                seg: ls,
                receiver: lr,
                args: la,
                ..
            }),
            MethodCall(ast::MethodCall {
                seg: rs,
                receiver: rr,
                args: ra,
                ..
            }),
        ) => eq_path_seg(ls, rs) && eq_expr(lr, rr) && over(la, ra, |l, r| eq_expr(l, r)),
        (Binary(lo, ll, lr), Binary(ro, rl, rr)) => lo.node == ro.node && eq_expr(ll, rl) && eq_expr(lr, rr),
        (Unary(lo, l), Unary(ro, r)) => mem::discriminant(lo) == mem::discriminant(ro) && eq_expr(l, r),
        (Lit(l), Lit(r)) => l == r,
        (Cast(l, lt), Cast(r, rt)) | (Type(l, lt), Type(r, rt)) => eq_expr(l, r) && eq_ty(lt, rt),
        (Let(lp, le, _, _), Let(rp, re, _, _)) => eq_pat(lp, rp) && eq_expr(le, re),
        (If(lc, lt, le), If(rc, rt, re)) => {
            eq_expr(lc, rc) && eq_block(lt, rt) && eq_expr_opt(le.as_deref(), re.as_deref())
        },
        (While(lc, lt, ll), While(rc, rt, rl)) => {
            eq_label(ll.as_ref(), rl.as_ref()) && eq_expr(lc, rc) && eq_block(lt, rt)
        },
        (ForLoop(lf), ForLoop(rf)) => {
            eq_label(lf.label.as_ref(), rf.label.as_ref())
                && eq_pat(&lf.pat, &rf.pat)
                && eq_expr(&lf.iter, &rf.iter)
                && eq_block(&lf.body, &rf.body)
                && lf.kind == rf.kind
        },
        (Loop(lt, ll, _), Loop(rt, rl, _)) => eq_label(ll.as_ref(), rl.as_ref()) && eq_block(lt, rt),
        (Block(lb, ll), Block(rb, rl)) => eq_label(ll.as_ref(), rl.as_ref()) && eq_block(lb, rb),
        (TryBlock(lb, lt), TryBlock(rb, rt)) => eq_block(lb, rb) && both(lt.as_deref(), rt.as_deref(), eq_ty),
        (Yield(l), Yield(r)) => eq_expr_opt(l.expr().map(Box::as_ref), r.expr().map(Box::as_ref)) && l.same_kind(r),
        (Ret(l), Ret(r)) => eq_expr_opt(l.as_deref(), r.as_deref()),
        (Break(ll, le), Break(rl, re)) => {
            eq_label(ll.as_ref(), rl.as_ref()) && eq_expr_opt(le.as_deref(), re.as_deref())
        },
        (Continue(ll), Continue(rl)) => eq_label(ll.as_ref(), rl.as_ref()),
        (Assign(l1, l2, _), Assign(r1, r2, _)) | (Index(l1, l2, _), Index(r1, r2, _)) => {
            eq_expr(l1, r1) && eq_expr(l2, r2)
        },
        (AssignOp(lo, lp, lv), AssignOp(ro, rp, rv)) => lo.node == ro.node && eq_expr(lp, rp) && eq_expr(lv, rv),
        (Field(lp, lf), Field(rp, rf)) => eq_id(*lf, *rf) && eq_expr(lp, rp),
        (Match(ls, la, lkind), Match(rs, ra, rkind)) => (lkind == rkind) && eq_expr(ls, rs) && over(la, ra, eq_arm),
        (
            Closure(ast::Closure {
                binder: lb,
                capture_clause: lc,
                coroutine_marker: lcm,
                movability: lm,
                fn_decl: lf,
                body: le,
                ..
            }),
            Closure(ast::Closure {
                binder: rb,
                capture_clause: rc,
                coroutine_marker: rcm,
                movability: rm,
                fn_decl: rf,
                body: re,
                ..
            }),
        ) => {
            eq_closure_binder(lb, rb)
                && lc == rc
                && eq_opt_coroutine_marker(*lcm, *rcm)
                && lm == rm
                && eq_fn_decl(lf, rf)
                && eq_expr(le, re)
        },
        (Gen(lc, lb, lk, _), Gen(rc, rb, rk, _)) => lc == rc && eq_block(lb, rb) && lk == rk,
        (Range(lf, lt, ll), Range(rf, rt, rl)) => {
            ll == rl && eq_expr_opt(lf.as_deref(), rf.as_deref()) && eq_expr_opt(lt.as_deref(), rt.as_deref())
        },
        (AddrOf(lbk, lm, le), AddrOf(rbk, rm, re)) => lbk == rbk && lm == rm && eq_expr(le, re),
        (Path(lq, lp), Path(rq, rp)) => both(lq.as_deref(), rq.as_deref(), eq_qself) && eq_path(lp, rp),
        (MacCall(l), MacCall(r)) => eq_mac_call(l, r),
        (Struct(lse), Struct(rse)) => {
            eq_maybe_qself(lse.qself.as_deref(), rse.qself.as_deref())
                && eq_path(&lse.path, &rse.path)
                && eq_struct_rest(&lse.rest, &rse.rest)
                && unordered_over(&lse.fields, &rse.fields, eq_field)
        },
        _ => false,
    }
}

/// Checks if two expression fields are equivalent, comparing their placeholder status, identifier, expression, and
/// attributes.
fn eq_field(l: &ExprField, r: &ExprField) -> bool {
    l.is_placeholder == r.is_placeholder
        && eq_id(l.ident, r.ident)
        && eq_expr(&l.expr, &r.expr)
        && over(&l.attrs, &r.attrs, eq_attr)
}

/// Checks if two match arms are equivalent, comparing their placeholder status, pattern, body, guard, and attributes.
fn eq_arm(l: &Arm, r: &Arm) -> bool {
    l.is_placeholder == r.is_placeholder
        && eq_pat(&l.pat, &r.pat)
        && eq_expr_opt(l.body.as_deref(), r.body.as_deref())
        && eq_expr_opt(l.guard.as_deref().map(|g| &g.cond), r.guard.as_deref().map(|g| &g.cond))
        && over(&l.attrs, &r.attrs, eq_attr)
}

/// Checks if `l` and `r` are both `None`, or if both are `Some` and their identifiers have the same name.
fn eq_label(l: Option<&Label>, r: Option<&Label>) -> bool {
    both(l, r, |l, r| eq_id(l.ident, r.ident))
}

/// Checks if two blocks are equivalent, comparing their rules and statements.
fn eq_block(l: &Block, r: &Block) -> bool {
    l.rules == r.rules && over(&l.stmts, &r.stmts, eq_stmt)
}

/// Checks if two statements are equivalent, comparing their kind and attributes.
fn eq_stmt(l: &Stmt, r: &Stmt) -> bool {
    use StmtKind::*;
    match (&l.kind, &r.kind) {
        (Let(l), Let(r)) => {
            eq_pat(&l.pat, &r.pat)
                && both(l.ty.as_ref(), r.ty.as_ref(), |l, r| eq_ty(l, r))
                && eq_local_kind(&l.kind, &r.kind)
                && over(&l.attrs, &r.attrs, eq_attr)
        },
        (Item(l), Item(r)) => eq_item(l, r, eq_item_kind),
        (Expr(l), Expr(r)) | (Semi(l), Semi(r)) => eq_expr(l, r),
        (Empty, Empty) => true,
        (MacCall(l), MacCall(r)) => {
            l.style == r.style && eq_mac_call(&l.mac, &r.mac) && over(&l.attrs, &r.attrs, eq_attr)
        },
        _ => false,
    }
}

/// Checks if two local kinds are equivalent: both are `Decl`, both are `Init` with equivalent expressions, or both are
/// `InitElse` with equivalent expressions and blocks.
fn eq_local_kind(l: &LocalKind, r: &LocalKind) -> bool {
    use LocalKind::*;
    match (l, r) {
        (Decl, Decl) => true,
        (Init(l), Init(r)) => eq_expr(l, r),
        (InitElse(li, le), InitElse(ri, re)) => eq_expr(li, ri) && eq_block(le, re),
        _ => false,
    }
}

/// Checks if two items are equivalent, comparing their attributes, visibility, and kind via `eq_kind`.
fn eq_item<K>(l: &Item<K>, r: &Item<K>, mut eq_kind: impl FnMut(&K, &K) -> bool) -> bool {
    over(&l.attrs, &r.attrs, eq_attr) && eq_vis(&l.vis, &r.vis) && eq_kind(&l.kind, &r.kind)
}

#[expect(clippy::too_many_lines, reason = "big match statement")]
/// Checks if two item kinds are equivalent, comparing their structure and contents.
fn eq_item_kind(l: &ItemKind, r: &ItemKind) -> bool {
    use ItemKind::*;
    match (l, r) {
        (ExternCrate(ls, li), ExternCrate(rs, ri)) => ls == rs && eq_id(*li, *ri),
        (Use(l), Use(r)) => eq_use_tree(l, r),
        (
            Static(StaticItem {
                ident: li,
                ty: lt,
                mutability: lm,
                expr: le,
                safety: ls,
                define_opaque: _,
                eii_impl: _,
            }),
            Static(StaticItem {
                ident: ri,
                ty: rt,
                mutability: rm,
                expr: re,
                safety: rs,
                define_opaque: _,
                eii_impl: _,
            }),
        ) => eq_id(*li, *ri) && lm == rm && ls == rs && eq_ty(lt, rt) && eq_expr_opt(le.as_deref(), re.as_deref()),
        (
            Const(ConstItem {
                defaultness: ld,
                ident: li,
                generics: lg,
                ty: lt,
                body: lb,
                define_opaque: _,
            }),
            Const(ConstItem {
                defaultness: rd,
                ident: ri,
                generics: rg,
                ty: rt,

                body: rb,
                define_opaque: _,
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && eq_ty(lt, rt)
                && both(lb.as_deref(), rb.as_deref(), eq_expr)
        },
        (
            Fn(ast::Fn {
                defaultness: ld,
                sig: lf,
                ident: li,
                generics: lg,
                contract: lc,
                body: lb,
                define_opaque: _,
                eii_impl: _,
            }),
            Fn(ast::Fn {
                defaultness: rd,
                sig: rf,
                ident: ri,
                generics: rg,
                contract: rc,
                body: rb,
                define_opaque: _,
                eii_impl: _,
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_fn_sig(lf, rf)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && eq_opt_fn_contract(lc, rc)
                && both(lb.as_ref(), rb.as_ref(), |l, r| eq_block(l, r))
        },
        (Mod(ls, li, lmk), Mod(rs, ri, rmk)) => {
            ls == rs
                && eq_id(*li, *ri)
                && match (lmk, rmk) {
                    (ModKind::Loaded(litems, linline, _), ModKind::Loaded(ritems, rinline, _)) => {
                        linline == rinline && over(litems, ritems, |l, r| eq_item(l, r, eq_item_kind))
                    },
                    (ModKind::Unloaded, ModKind::Unloaded) => true,
                    _ => false,
                }
        },
        (ForeignMod(l), ForeignMod(r)) => {
            both(l.abi.as_ref(), r.abi.as_ref(), eq_str_lit)
                && over(&l.items, &r.items, |l, r| eq_item(l, r, eq_foreign_item_kind))
        },
        (
            TyAlias(ast::TyAlias {
                defaultness: ld,
                generics: lg,
                bounds: lb,
                ty: lt,
                ..
            }),
            TyAlias(ast::TyAlias {
                defaultness: rd,
                generics: rg,
                bounds: rb,
                ty: rt,
                ..
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_generics(lg, rg)
                && over(lb, rb, eq_generic_bound)
                && both(lt.as_ref(), rt.as_ref(), |l, r| eq_ty(l, r))
        },
        (Enum(li, lg, le), Enum(ri, rg, re)) => {
            eq_id(*li, *ri) && eq_generics(lg, rg) && over(&le.variants, &re.variants, eq_variant)
        },
        (Struct(li, lg, lv), Struct(ri, rg, rv)) | (Union(li, lg, lv), Union(ri, rg, rv)) => {
            eq_id(*li, *ri) && eq_generics(lg, rg) && eq_variant_data(lv, rv)
        },
        (
            Trait(ast::Trait {
                impl_restriction: liprt,
                constness: lc,
                is_auto: la,
                safety: lu,
                ident: li,
                generics: lg,
                bounds: lb,
                items: lis,
            }),
            Trait(ast::Trait {
                impl_restriction: riprt,
                constness: rc,
                is_auto: ra,
                safety: ru,
                ident: ri,
                generics: rg,
                bounds: rb,
                items: ris,
            }),
        ) => {
            eq_impl_restriction(liprt, riprt)
                && matches!(lc, ast::Const::No) == matches!(rc, ast::Const::No)
                && la == ra
                && matches!(lu, Safety::Default) == matches!(ru, Safety::Default)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && over(lb, rb, eq_generic_bound)
                && over(lis, ris, |l, r| eq_item(l, r, eq_assoc_item_kind))
        },
        (
            TraitAlias(ast::TraitAlias {
                ident: li,
                generics: lg,
                bounds: lb,
                constness: lc,
            }),
            TraitAlias(ast::TraitAlias {
                ident: ri,
                generics: rg,
                bounds: rb,
                constness: rc,
            }),
        ) => {
            matches!(lc, ast::Const::No) == matches!(rc, ast::Const::No)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && over(lb, rb, eq_generic_bound)
        },
        (
            Impl(ast::Impl {
                generics: lg,
                of_trait: lot,
                self_ty: lst,
                items: li,
                constness: lc,
            }),
            Impl(ast::Impl {
                generics: rg,
                of_trait: rot,
                self_ty: rst,
                items: ri,
                constness: rc,
            }),
        ) => {
            eq_generics(lg, rg)
                && both(lot.as_deref(), rot.as_deref(), |l, r| {
                    matches!(l.safety, Safety::Default) == matches!(r.safety, Safety::Default)
                        && matches!(l.polarity, ImplPolarity::Positive) == matches!(r.polarity, ImplPolarity::Positive)
                        && eq_defaultness(l.defaultness, r.defaultness)
                        && matches!(lc, ast::Const::No) == matches!(rc, ast::Const::No)
                        && eq_path(&l.trait_ref.path, &r.trait_ref.path)
                })
                && eq_ty(lst, rst)
                && over(li, ri, |l, r| eq_item(l, r, eq_assoc_item_kind))
        },
        (MacCall(l), MacCall(r)) => eq_mac_call(l, r),
        (MacroDef(li, ld), MacroDef(ri, rd)) => {
            eq_id(*li, *ri) && ld.macro_rules == rd.macro_rules && eq_delim_args(&ld.body, &rd.body)
        },
        _ => false,
    }
}

/// Checks if two foreign item kinds are equivalent, comparing their kind and contents.
fn eq_foreign_item_kind(l: &ForeignItemKind, r: &ForeignItemKind) -> bool {
    use ForeignItemKind::*;
    match (l, r) {
        (
            Static(StaticItem {
                ident: li,
                ty: lt,
                mutability: lm,
                expr: le,
                safety: ls,
                define_opaque: _,
                eii_impl: _,
            }),
            Static(StaticItem {
                ident: ri,
                ty: rt,
                mutability: rm,
                expr: re,
                safety: rs,
                define_opaque: _,
                eii_impl: _,
            }),
        ) => eq_id(*li, *ri) && eq_ty(lt, rt) && lm == rm && eq_expr_opt(le.as_deref(), re.as_deref()) && ls == rs,
        (
            Fn(ast::Fn {
                defaultness: ld,
                sig: lf,
                ident: li,
                generics: lg,
                contract: lc,
                body: lb,
                define_opaque: _,
                eii_impl: _,
            }),
            Fn(ast::Fn {
                defaultness: rd,
                sig: rf,
                ident: ri,
                generics: rg,
                contract: rc,
                body: rb,
                define_opaque: _,
                eii_impl: _,
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_fn_sig(lf, rf)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && eq_opt_fn_contract(lc, rc)
                && both(lb.as_ref(), rb.as_ref(), |l, r| eq_block(l, r))
        },
        (
            TyAlias(ast::TyAlias {
                defaultness: ld,
                ident: li,
                generics: lg,
                after_where_clause: lw,
                bounds: lb,
                ty: lt,
            }),
            TyAlias(ast::TyAlias {
                defaultness: rd,
                ident: ri,
                generics: rg,
                after_where_clause: rw,
                bounds: rb,
                ty: rt,
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && over(&lw.predicates, &rw.predicates, eq_where_predicate)
                && over(lb, rb, eq_generic_bound)
                && both(lt.as_ref(), rt.as_ref(), |l, r| eq_ty(l, r))
        },
        (MacCall(l), MacCall(r)) => eq_mac_call(l, r),
        _ => false,
    }
}

/// Checks if two associated item kinds are equivalent, comparing their kind and contents.
fn eq_assoc_item_kind(l: &AssocItemKind, r: &AssocItemKind) -> bool {
    use AssocItemKind::*;
    match (l, r) {
        (
            Const(ConstItem {
                defaultness: ld,
                ident: li,
                generics: lg,
                ty: lt,
                body: lb,
                define_opaque: _,
            }),
            Const(ConstItem {
                defaultness: rd,
                ident: ri,
                generics: rg,
                ty: rt,
                body: rb,
                define_opaque: _,
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && eq_ty(lt, rt)
                && both(lb.as_deref(), rb.as_deref(), eq_expr)
        },
        (
            Fn(ast::Fn {
                defaultness: ld,
                sig: lf,
                ident: li,
                generics: lg,
                contract: lc,
                body: lb,
                define_opaque: _,
                eii_impl: _,
            }),
            Fn(ast::Fn {
                defaultness: rd,
                sig: rf,
                ident: ri,
                generics: rg,
                contract: rc,
                body: rb,
                define_opaque: _,
                eii_impl: _,
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_fn_sig(lf, rf)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && eq_opt_fn_contract(lc, rc)
                && both(lb.as_ref(), rb.as_ref(), |l, r| eq_block(l, r))
        },
        (
            Type(TyAlias {
                defaultness: ld,
                ident: li,
                generics: lg,
                after_where_clause: lw,
                bounds: lb,
                ty: lt,
            }),
            Type(TyAlias {
                defaultness: rd,
                ident: ri,
                generics: rg,
                after_where_clause: rw,
                bounds: rb,
                ty: rt,
            }),
        ) => {
            eq_defaultness(*ld, *rd)
                && eq_id(*li, *ri)
                && eq_generics(lg, rg)
                && over(&lw.predicates, &rw.predicates, eq_where_predicate)
                && over(lb, rb, eq_generic_bound)
                && both(lt.as_ref(), rt.as_ref(), |l, r| eq_ty(l, r))
        },
        (MacCall(l), MacCall(r)) => eq_mac_call(l, r),
        _ => false,
    }
}

/// Checks if two variants are equivalent, comparing their placeholder status, attributes, visibility, identifier, data,
/// and discriminant.
fn eq_variant(l: &Variant, r: &Variant) -> bool {
    l.is_placeholder == r.is_placeholder
        && over(&l.attrs, &r.attrs, eq_attr)
        && eq_vis(&l.vis, &r.vis)
        && eq_id(l.ident, r.ident)
        && eq_variant_data(&l.data, &r.data)
        && both(l.disr_expr.as_ref(), r.disr_expr.as_ref(), |l, r| {
            eq_expr(&l.value, &r.value)
        })
}

/// Checks if two variant data are equivalent: both are unit, both are struct with equivalent fields, or both are tuple
/// with equivalent fields.
fn eq_variant_data(l: &VariantData, r: &VariantData) -> bool {
    use VariantData::*;
    match (l, r) {
        (Unit(_), Unit(_)) => true,
        (Struct { fields: l, .. }, Struct { fields: r, .. }) | (Tuple(l, _), Tuple(r, _)) => {
            over(l, r, eq_struct_field)
        },
        _ => false,
    }
}

/// Checks if two field definitions are equivalent, comparing their placeholder status, attributes, visibility,
/// mutability, identifier, and type.
fn eq_struct_field(l: &FieldDef, r: &FieldDef) -> bool {
    l.is_placeholder == r.is_placeholder
        && over(&l.attrs, &r.attrs, eq_attr)
        && eq_vis(&l.vis, &r.vis)
        && eq_mut_restriction(l.mut_restriction(), r.mut_restriction())
        && both(l.ident.as_ref(), r.ident.as_ref(), |l, r| eq_id(*l, *r))
        && eq_ty(&l.ty, &r.ty)
}

/// Checks if two function signatures are equivalent, comparing their declaration and header.
fn eq_fn_sig(l: &FnSig, r: &FnSig) -> bool {
    eq_fn_decl(&l.decl, &r.decl) && eq_fn_header(&l.header, &r.header)
}

/// Checks if two optional coroutine markers are equivalent: both `None`, or both `Some` with the same kind.
fn eq_opt_coroutine_marker(l: Option<CoroutineMarker>, r: Option<CoroutineMarker>) -> bool {
    match (l, r) {
        (Some(lcm), Some(rcm)) => lcm.kind == rcm.kind,
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

/// Checks if two function headers are equivalent, comparing safety, coroutine marker, constness, and extern.
fn eq_fn_header(l: &FnHeader, r: &FnHeader) -> bool {
    matches!(l.safety, Safety::Default) == matches!(r.safety, Safety::Default)
        && eq_opt_coroutine_marker(l.coroutine_marker, r.coroutine_marker)
        && matches!(l.constness, Const::No) == matches!(r.constness, Const::No)
        && eq_ext(&l.ext, &r.ext)
}

#[expect(clippy::ref_option, reason = "This is the type how it is stored in the AST")]
/// Checks if two optional function contracts are equivalent: both `None`, or both `Some` with equivalent requires and
/// ensures clauses.
fn eq_opt_fn_contract(l: &Option<Box<FnContract>>, r: &Option<Box<FnContract>>) -> bool {
    match (l, r) {
        (Some(l), Some(r)) => {
            eq_expr_opt(l.requires.as_deref(), r.requires.as_deref())
                && eq_expr_opt(l.ensures.as_deref(), r.ensures.as_deref())
        },
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

/// Checks if two generic parameter lists are equivalent, comparing their parameters and where-clause predicates.
fn eq_generics(l: &Generics, r: &Generics) -> bool {
    over(&l.params, &r.params, eq_generic_param)
        && over(&l.where_clause.predicates, &r.where_clause.predicates, |l, r| {
            eq_where_predicate(l, r)
        })
}

/// Checks if two where-predicates are equivalent: both are bound predicates with equivalent types and bounds, or both
/// are region predicates with the same lifetime and bounds.
fn eq_where_predicate(l: &WherePredicate, r: &WherePredicate) -> bool {
    use WherePredicateKind::*;
    over(&l.attrs, &r.attrs, eq_attr)
        && match (&l.kind, &r.kind) {
            (BoundPredicate(l), BoundPredicate(r)) => {
                over(&l.bound_generic_params, &r.bound_generic_params, |l, r| {
                    eq_generic_param(l, r)
                }) && eq_ty(&l.bounded_ty, &r.bounded_ty)
                    && over(&l.bounds, &r.bounds, eq_generic_bound)
            },
            (RegionPredicate(l), RegionPredicate(r)) => {
                eq_id(l.lifetime.ident, r.lifetime.ident) && over(&l.bounds, &r.bounds, eq_generic_bound)
            },
            _ => false,
        }
}

/// Checks if two use trees are equivalent, comparing their prefix and kind.
fn eq_use_tree(l: &UseTree, r: &UseTree) -> bool {
    eq_path(&l.prefix, &r.prefix) && eq_use_tree_kind(&l.kind, &r.kind)
}

/// Checks if two anonymous constants are equivalent, comparing their values.
fn eq_anon_const(l: &AnonConst, r: &AnonConst) -> bool {
    eq_expr(&l.value, &r.value)
}

/// Checks if two use tree kinds are equivalent: both glob, both simple with the same identifier, or both nested with
/// equivalent trees.
fn eq_use_tree_kind(l: &UseTreeKind, r: &UseTreeKind) -> bool {
    use UseTreeKind::*;
    match (l, r) {
        (Glob(_), Glob(_)) => true,
        (Simple(l), Simple(r)) => both(l.as_ref(), r.as_ref(), |l, r| eq_id(*l, *r)),
        (Nested { items: l, .. }, Nested { items: r, .. }) => over(l, r, |l, r| eq_use_tree(&l.inner, &r.inner)),
        _ => false,
    }
}

/// Checks if two defaultness values are equivalent: both implicit, both default, or both final.
fn eq_defaultness(l: Defaultness, r: Defaultness) -> bool {
    matches!(
        (l, r),
        (Defaultness::Implicit, Defaultness::Implicit)
            | (Defaultness::Default(_), Defaultness::Default(_))
            | (Defaultness::Final(_), Defaultness::Final(_))
    )
}

/// Checks if two visibility modifiers are equivalent: both public, both inherited, or both restricted to equivalent
/// paths.
fn eq_vis(l: &Visibility, r: &Visibility) -> bool {
    use VisibilityKind::*;
    match (&l.kind, &r.kind) {
        (Public, Public) | (Inherited, Inherited) => true,
        (Restricted { path: l, .. }, Restricted { path: r, .. }) => eq_path(l, r),
        _ => false,
    }
}

/// Checks if two impl restrictions are equivalent, comparing their restriction kinds.
fn eq_impl_restriction(l: &ImplRestriction, r: &ImplRestriction) -> bool {
    eq_restriction_kind(&l.kind, &r.kind)
}

/// Checks if two mut restrictions are equivalent, comparing their restriction kinds.
pub fn eq_mut_restriction(l: &MutRestriction, r: &MutRestriction) -> bool {
    eq_restriction_kind(&l.kind, &r.kind)
}

/// Checks if two restriction kinds are equivalent: both unrestricted, or both restricted to equivalent paths with the
/// same shorthand.
fn eq_restriction_kind(l: &RestrictionKind, r: &RestrictionKind) -> bool {
    match (l, r) {
        (RestrictionKind::Unrestricted, RestrictionKind::Unrestricted) => true,
        (
            RestrictionKind::Restricted {
                path: l_path,
                shorthand: l_short,
                id: _,
            },
            RestrictionKind::Restricted {
                path: r_path,
                shorthand: r_short,
                id: _,
            },
        ) => l_short == r_short && eq_path(l_path, r_path),
        _ => false,
    }
}

/// Checks if two function declarations are equivalent, comparing their return types and parameters.
fn eq_fn_decl(l: &FnDecl, r: &FnDecl) -> bool {
    eq_fn_ret_ty(&l.output, &r.output) && over(&l.inputs, &r.inputs, eq_param)
}

/// Checks if two parameters are equivalent, comparing their placeholder status, pattern, type, and attributes.
fn eq_param(l: &Param, r: &Param) -> bool {
    l.is_placeholder == r.is_placeholder
        && eq_pat(&l.pat, &r.pat)
        && eq_ty(&l.ty, &r.ty)
        && over(&l.attrs, &r.attrs, eq_attr)
}

/// Checks if two closure binders are equivalent: both absent, or both `for<...>` with the same number of equivalent
/// generic parameters.
fn eq_closure_binder(l: &ClosureBinder, r: &ClosureBinder) -> bool {
    match (l, r) {
        (ClosureBinder::NotPresent, ClosureBinder::NotPresent) => true,
        (ClosureBinder::For { generic_params: lp, .. }, ClosureBinder::For { generic_params: rp, .. }) => {
            lp.len() == rp.len() && std::iter::zip(lp.iter(), rp.iter()).all(|(l, r)| eq_generic_param(l, r))
        },
        _ => false,
    }
}

/// Checks if two function return types are equivalent: both default, or both explicit with equivalent types.
fn eq_fn_ret_ty(l: &FnRetTy, r: &FnRetTy) -> bool {
    match (l, r) {
        (FnRetTy::Default(_), FnRetTy::Default(_)) => true,
        (FnRetTy::Ty(l), FnRetTy::Ty(r)) => eq_ty(l, r),
        _ => false,
    }
}

/// Checks if two types are equivalent, ignoring `NodeId`s and `Span`s.
fn eq_ty(l: &Ty, r: &Ty) -> bool {
    use TyKind::*;
    match (&l.kind, &r.kind) {
        (Paren(l), _) => eq_ty(l, r),
        (_, Paren(r)) => eq_ty(l, r),
        (Never, Never) | (Infer, Infer) | (ImplicitSelf, ImplicitSelf) | (Err(_), Err(_)) | (CVarArgs, CVarArgs) => {
            true
        },
        (Slice(l), Slice(r)) => eq_ty(l, r),
        (Array(le, ls), Array(re, rs)) => eq_ty(le, re) && eq_expr(&ls.value, &rs.value),
        (Ptr(l), Ptr(r)) => l.mutbl == r.mutbl && eq_ty(&l.ty, &r.ty),
        (Ref(ll, l), Ref(rl, r)) => {
            both(ll.as_ref(), rl.as_ref(), |l, r| eq_id(l.ident, r.ident)) && l.mutbl == r.mutbl && eq_ty(&l.ty, &r.ty)
        },
        (PinnedRef(ll, l), PinnedRef(rl, r)) => {
            both(ll.as_ref(), rl.as_ref(), |l, r| eq_id(l.ident, r.ident)) && l.mutbl == r.mutbl && eq_ty(&l.ty, &r.ty)
        },
        (FnPtr(l), FnPtr(r)) => {
            l.safety == r.safety
                && eq_ext(&l.ext, &r.ext)
                && over(&l.generic_params, &r.generic_params, eq_generic_param)
                && eq_fn_decl(&l.decl, &r.decl)
        },
        (Tup(l), Tup(r)) => over(l, r, |l, r| eq_ty(l, r)),
        (Path(lq, lp), Path(rq, rp)) => both(lq.as_deref(), rq.as_deref(), eq_qself) && eq_path(lp, rp),
        (TraitObject(lg, ls), TraitObject(rg, rs)) => ls == rs && over(lg, rg, eq_generic_bound),
        (ImplTrait(_, lg), ImplTrait(_, rg)) => over(lg, rg, eq_generic_bound),
        (MacCall(l), MacCall(r)) => eq_mac_call(l, r),
        _ => false,
    }
}

/// Checks if two extern specifications are equivalent: both absent/implicit, or both explicit with equivalent string
/// literals.
fn eq_ext(l: &Extern, r: &Extern) -> bool {
    use Extern::*;
    match (l, r) {
        (None, None) | (Implicit(_), Implicit(_)) => true,
        (Explicit(l, _), Explicit(r, _)) => eq_str_lit(l, r),
        _ => false,
    }
}

/// Checks if both string literals have the same style, symbol, and suffix.
fn eq_str_lit(l: &StrLit, r: &StrLit) -> bool {
    l.style == r.style && l.symbol == r.symbol && l.suffix == r.suffix
}

/// Checks if two poly trait refs are equivalent, comparing their modifiers, trait path, and bound generic parameters.
fn eq_poly_ref_trait(l: &PolyTraitRef, r: &PolyTraitRef) -> bool {
    l.modifiers == r.modifiers
        && eq_path(&l.trait_ref.path, &r.trait_ref.path)
        && over(&l.bound_generic_params, &r.bound_generic_params, |l, r| {
            eq_generic_param(l, r)
        })
}

/// Checks if two generic parameters are equivalent, comparing their placeholder status, identifier, bounds, kind, and
/// attributes.
fn eq_generic_param(l: &GenericParam, r: &GenericParam) -> bool {
    use GenericParamKind::*;
    l.is_placeholder == r.is_placeholder
        && eq_id(l.ident, r.ident)
        && over(&l.bounds, &r.bounds, eq_generic_bound)
        && match (&l.kind, &r.kind) {
            (Lifetime, Lifetime) => true,
            (Type { default: l }, Type { default: r }) => both(l.as_ref(), r.as_ref(), |l, r| eq_ty(l, r)),
            (
                Const {
                    ty: lt,
                    default: ld,
                    span: _,
                },
                Const {
                    ty: rt,
                    default: rd,
                    span: _,
                },
            ) => eq_ty(lt, rt) && both(ld.as_ref(), rd.as_ref(), eq_anon_const),
            _ => false,
        }
        && over(&l.attrs, &r.attrs, eq_attr)
}

/// Checks if two generic bounds are equivalent: both are trait bounds with equivalent trait refs, or both are outlives
/// bounds with the same lifetime.
fn eq_generic_bound(l: &GenericBound, r: &GenericBound) -> bool {
    use GenericBound::*;
    match (l, r) {
        (Trait(ptr1), Trait(ptr2)) => eq_poly_ref_trait(ptr1, ptr2),
        (Outlives(l), Outlives(r)) => eq_id(l.ident, r.ident),
        _ => false,
    }
}

/// Checks if two terms are equivalent: both types with equivalent types, or both constants with equivalent anonymous
/// constants.
fn eq_term(l: &Term, r: &Term) -> bool {
    match (l, r) {
        (Term::Ty(l), Term::Ty(r)) => eq_ty(l, r),
        (Term::Const(l), Term::Const(r)) => eq_anon_const(l, r),
        _ => false,
    }
}

/// Checks if two associated item constraints are equivalent, comparing their identifier and kind.
fn eq_assoc_item_constraint(l: &AssocItemConstraint, r: &AssocItemConstraint) -> bool {
    use AssocItemConstraintKind::*;
    eq_id(l.ident, r.ident)
        && match (&l.kind, &r.kind) {
            (Equality { term: l }, Equality { term: r }) => eq_term(l, r),
            (Bound { bounds: l }, Bound { bounds: r }) => over(l, r, eq_generic_bound),
            _ => false,
        }
}

/// Checks if two macro calls are equivalent, comparing their path and delimiter arguments.
fn eq_mac_call(l: &MacCall, r: &MacCall) -> bool {
    eq_path(&l.path, &r.path) && eq_delim_args(&l.args, &r.args)
}

/// Checks if two attributes are equivalent, comparing their style and kind.
fn eq_attr(l: &Attribute, r: &Attribute) -> bool {
    use AttrKind::*;
    l.style == r.style
        && match (&l.kind, &r.kind) {
            (DocComment(l1, l2), DocComment(r1, r2)) => l1 == r1 && l2 == r2,
            (Normal(l), Normal(r)) => eq_path(&l.item.path, &r.item.path) && eq_attr_args(&l.item.args, &r.item.args),
            (Synthetic(..), _) | (_, Synthetic(..)) => unreachable!(),
            _ => false,
        }
}

/// Checks if two attribute arguments are equivalent: both empty, both delimited with equivalent delimiters, or both `=`
/// with equivalent expressions.
fn eq_attr_args(l: &AttrArgs, r: &AttrArgs) -> bool {
    use AttrArgs::*;
    match (l, r) {
        (Empty, Empty) => true,
        (Delimited(la), Delimited(ra)) => eq_delim_args(la, ra),
        (Eq { eq_span: _, expr: le }, Eq { eq_span: _, expr: re }) => eq_expr(le, re),
        _ => false,
    }
}

/// Checks if two delimiter arguments are equivalent, comparing their delimiter and token streams.
fn eq_delim_args(l: &DelimArgs, r: &DelimArgs) -> bool {
    l.delim == r.delim
        && l.tokens.len() == r.tokens.len()
        && l.tokens.iter().zip(r.tokens.iter()).all(|(a, b)| a.eq_unspanned(b))
}

/// Checks whether `item` is gated on `#[cfg(test)]`.
pub fn is_cfg_test(item: &impl HasAttrs) -> bool {
    item.attrs().iter().any(|attr| {
        if attr.has_name(sym::cfg)
            && let Some(item_list) = attr.meta_item_list()
            && item_list.iter().any(|item| item.has_name(sym::test))
        {
            true
        } else if let AttrKind::Synthetic(synthetic) = &attr.kind
            && let SyntheticAttr::CfgTrace(cfg) = &**synthetic
        {
            requires_test_cfg(cfg)
        } else {
            false
        }
    })
}

fn requires_test_cfg(cfg: &CfgEntry) -> bool {
    match cfg {
        CfgEntry::NameValue { name: sym::test, .. } => true,
        CfgEntry::All(subs, _) => subs.iter().any(requires_test_cfg),
        _ => false,
    }
}
