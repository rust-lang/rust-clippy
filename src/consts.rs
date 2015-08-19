#![allow(cast_possible_truncation)]

use rustc::lint::Context;
use rustc::middle::const_eval::lookup_const_by_id;
use rustc::middle::def::PathResolution;
use rustc::middle::def::Def::*;
use syntax::ast::*;
use syntax::ptr::P;
use std::cmp::PartialOrd;
use std::cmp::Ordering::{self, Greater, Less, Equal};
use std::rc::Rc;
use std::ops::Deref;
use std::{u8, u16, u32, u64};
use self::Constant::*;
use self::FloatWidth::*;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum FloatWidth {
    Fw32,
    Fw64,
    FwAny
}

impl From<FloatTy> for FloatWidth {
    fn from(ty: FloatTy) -> FloatWidth {
        match ty {
            TyF32 => Fw32,
            TyF64 => Fw64,
        }
    }
}

/// a Lit_-like enum to fold constant `Expr`s into
#[derive(Eq, Debug, Clone)]
pub enum Constant {
    /// a String "abc"
    ConstantStr(String, StrStyle),
    /// a Binary String b"abc"
    ConstantBinary(Rc<Vec<u8>>),
    /// a single byte b'a'
    ConstantByte(u8),
    /// a single char 'a'
    ConstantChar(char),
    /// an integer
    ConstantInt(u64, LitIntType),
    /// a float with given type
    ConstantFloat(String, FloatWidth),
    /// true or false
    ConstantBool(bool),
    /// an array of constants
    ConstantVec(Vec<Constant>),
    /// also an array, but with only one constant, repeated N times
    ConstantRepeat(Box<Constant>, usize),
    /// a tuple of constants
    ConstantTuple(Vec<Constant>),
}

impl Constant {
    /// convert to u64 if possible
    ///
    /// # panics
    ///
    /// if the constant could not be converted to u64 losslessly
    fn as_u64(&self) -> u64 {
        if let ConstantInt(val, _) = *self {
            val // TODO we may want to check the sign if any
        } else {
            panic!("Could not convert a {:?} to u64");
        }
    }

    /// convert this constant to a f64, if possible
    #[allow(cast_precision_loss)]
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            ConstantByte(b) => Some(b as f64),
            ConstantFloat(ref s, _) => s.parse().ok(),
            ConstantInt(i, ty) => Some(if is_negative(ty) {
                -(i as f64) } else { i as f64 }),
            _ => None
        }
    }
}

impl PartialEq for Constant {
    fn eq(&self, other: &Constant) -> bool {
        match (self, other) {
            (&ConstantStr(ref ls, ref lsty), &ConstantStr(ref rs, ref rsty)) =>
                ls == rs && lsty == rsty,
            (&ConstantBinary(ref l), &ConstantBinary(ref r)) => l == r,
            (&ConstantByte(l), &ConstantByte(r)) => l == r,
            (&ConstantChar(l), &ConstantChar(r)) => l == r,
            (&ConstantInt(lv, lty), &ConstantInt(rv, rty)) => lv == rv &&
               (is_negative(lty) & (lv != 0)) == (is_negative(rty) & (rv != 0)),
            (&ConstantFloat(ref ls, lw), &ConstantFloat(ref rs, rw)) =>
                if match (lw, rw) {
                    (FwAny, _) | (_, FwAny) | (Fw32, Fw32) | (Fw64, Fw64) => true,
                    _ => false,
                } {
                    match (ls.parse::<f64>(), rs.parse::<f64>()) {
                        (Ok(l), Ok(r)) => l.eq(&r),
                        _ => false,
                    }
                } else { false },
            (&ConstantBool(l), &ConstantBool(r)) => l == r,
            (&ConstantVec(ref l), &ConstantVec(ref r)) => l == r,
            (&ConstantRepeat(ref lv, ref ls), &ConstantRepeat(ref rv, ref rs)) =>
                ls == rs && lv == rv,
            (&ConstantTuple(ref l), &ConstantTuple(ref r)) => l == r,
            _ => false, //TODO: Are there inter-type equalities?
        }
    }
}

impl PartialOrd for Constant {
    fn partial_cmp(&self, other: &Constant) -> Option<Ordering> {
        match (self, other) {
            (&ConstantStr(ref ls, ref lsty), &ConstantStr(ref rs, ref rsty)) =>
                if lsty != rsty { None } else { Some(ls.cmp(rs)) },
            (&ConstantByte(ref l), &ConstantByte(ref r)) => Some(l.cmp(r)),
            (&ConstantChar(ref l), &ConstantChar(ref r)) => Some(l.cmp(r)),
            (&ConstantInt(ref lv, lty), &ConstantInt(ref rv, rty)) =>
                Some(match (is_negative(lty) && *lv != 0,
                            is_negative(rty) && *rv != 0) {
                    (true, true) => lv.cmp(rv),
                    (false, false) => rv.cmp(lv),
                    (true, false) => Greater,
                    (false, true) => Less,
                }),
            (&ConstantFloat(ref ls, lw), &ConstantFloat(ref rs, rw)) =>
                if match (lw, rw) {
                    (FwAny, _) | (_, FwAny) | (Fw32, Fw32) | (Fw64, Fw64) => true,
                    _ => false,
                } {
                    match (ls.parse::<f64>(), rs.parse::<f64>()) {
                        (Ok(ref l), Ok(ref r)) => l.partial_cmp(r),
                        _ => None,
                    }
                } else { None },
            (&ConstantBool(ref l), &ConstantBool(ref r)) => Some(l.cmp(r)),
            (&ConstantVec(ref l), &ConstantVec(ref r)) => l.partial_cmp(&r),
            (&ConstantRepeat(ref lv, ref ls), &ConstantRepeat(ref rv, ref rs)) =>
                match lv.partial_cmp(rv) {
                    Some(Equal) => Some(ls.cmp(rs)),
                    x => x,
                },
            (&ConstantTuple(ref l), &ConstantTuple(ref r)) => l.partial_cmp(r),
             _ => None, //TODO: Are there any useful inter-type orderings?
         }
    }
}



fn lit_to_constant(lit: &Lit_) -> Constant {
    match *lit {
        LitStr(ref is, style) => ConstantStr(is.to_string(), style),
        LitBinary(ref blob) => ConstantBinary(blob.clone()),
        LitByte(b) => ConstantByte(b),
        LitChar(c) => ConstantChar(c),
        LitInt(value, ty) => ConstantInt(value, ty),
        LitFloat(ref is, ty) => ConstantFloat(is.to_string(), ty.into()),
        LitFloatUnsuffixed(ref is) => ConstantFloat(is.to_string(), FwAny),
        LitBool(b) => ConstantBool(b),
    }
}

fn constant_not(o: Constant) -> Option<Constant> {
    Some(match o {
        ConstantBool(b) => ConstantBool(!b),
        ConstantInt(value, ty) => {
            let (nvalue, nty) = match ty {
                SignedIntLit(ity, Plus) => {
                    if value == ::std::u64::MAX { return None; }
                    (value + 1, SignedIntLit(ity, Minus))
                },
                SignedIntLit(ity, Minus) => {
                    if value == 0 {
                        (1, SignedIntLit(ity, Minus))
                    } else {
                        (value - 1, SignedIntLit(ity, Plus))
                    }
                }
                UnsignedIntLit(ity) => {
                    let mask = match ity {
                        UintTy::TyU8 => ::std::u8::MAX as u64,
                        UintTy::TyU16 => ::std::u16::MAX as u64,
                        UintTy::TyU32 => ::std::u32::MAX as u64,
                        UintTy::TyU64 => ::std::u64::MAX,
                        UintTy::TyUs => { return None; }  // refuse to guess
                    };
                    (!value & mask, UnsignedIntLit(ity))
                }
                UnsuffixedIntLit(_) => { return None; }  // refuse to guess
            };
            ConstantInt(nvalue, nty)
        },
        _ => { return None; }
    })
}

fn constant_negate(o: Constant) -> Option<Constant> {
    Some(match o {
        ConstantInt(value, ty) =>
            ConstantInt(value, match ty {
                SignedIntLit(ity, sign) =>
                    SignedIntLit(ity, neg_sign(sign)),
                UnsuffixedIntLit(sign) => UnsuffixedIntLit(neg_sign(sign)),
                _ => { return None; },
            }),
        ConstantFloat(is, ty) =>
            ConstantFloat(neg_float_str(is), ty),
        _ => { return None; },
    })
}

fn neg_sign(s: Sign) -> Sign {
    match s {
        Sign::Plus => Sign::Minus,
        Sign::Minus => Sign::Plus,
    }
}

fn neg_float_str(s: String) -> String {
    if s.starts_with('-') {
        s[1..].to_owned()
    } else {
        format!("-{}", s)
    }
}

/// is the given LitIntType negative?
///
/// Examples
///
/// ```
/// assert!(is_negative(UnsuffixedIntLit(Minus)));
/// ```
pub fn is_negative(ty: LitIntType) -> bool {
    match ty {
        SignedIntLit(_, sign) | UnsuffixedIntLit(sign) => sign == Minus,
        UnsignedIntLit(_) => false,
    }
}

fn get_sign(cty: LitIntType) -> Sign {
    match cty {
        SignedIntLit(_, sign) | UnsuffixedIntLit(sign) => sign,
        _ => Plus,
    }
}

//TODO: Should negative have any bearing on the result?
fn clamp(value: u64, _negative: bool, bits: u8) -> Option<u64> {
    if bits == 64 || value <= (1 << bits) - 1 { Some(value) } else { None }
}

fn with_ty<T, F: FnOnce(&str) -> Option<T>>(ty: &Ty, f: F) -> Option<T> {
    if let TyPath(_, ref path) = ty.node {
        if path.segments.len() == 1 {
            f(&path.segments[0].identifier.name.as_str());
        }
    }
    None
}

/// unwrap parenthesis and blocks, and follow paths to constant definitions.
/// This is done repeatedly until a final expression is found and returned.
fn with_follow<F, T>(cx: Option<&Context>, e: &Expr, resolv: bool, f: F) -> T
where F: FnOnce(&Expr, bool) -> T {
    match e.node {
        ExprParen(ref inner) => with_follow(cx, inner, resolv, f),
        ExprBlock(ref block) =>
            if block.stmts.is_empty() {
                match &block.expr {
                    &Some(ref inner) => with_follow(cx, inner, resolv, f),
                    _ => f(e, resolv),
                } //TODO: add return handling?
            } else { f(e, resolv) },
        ExprPath(_, _) => {
            if let Some(lcx) = cx {
                if let Some(&PathResolution { base_def: DefConst(id), ..}) =
                        lcx.tcx.def_map.borrow().get(&e.id) {
                    if let Some(const_expr) =
                            lookup_const_by_id(lcx.tcx, id, None) {
                        return with_follow(cx, const_expr, true, f);
                    }
                }
            };
            f(e, resolv)
        },
        _ => f(e, resolv)
    }
}

fn unify_int_type(l: LitIntType, r: LitIntType, s: Sign) -> Option<LitIntType> {
    match (l, r) {
        (SignedIntLit(lty, _), SignedIntLit(rty, _)) => if lty == rty {
            Some(SignedIntLit(lty, s)) } else { None },
        (UnsignedIntLit(lty), UnsignedIntLit(rty)) =>
            if s == Plus && lty == rty {
                Some(UnsignedIntLit(lty))
            } else { None },
        (UnsuffixedIntLit(_), UnsuffixedIntLit(_)) => Some(UnsuffixedIntLit(s)),
        (SignedIntLit(lty, _), UnsuffixedIntLit(_)) => Some(SignedIntLit(lty, s)),
        (UnsignedIntLit(lty), UnsuffixedIntLit(rs)) => if rs == Plus {
            Some(UnsignedIntLit(lty)) } else { None },
        (UnsuffixedIntLit(_), SignedIntLit(rty, _)) => Some(SignedIntLit(rty, s)),
        (UnsuffixedIntLit(ls), UnsignedIntLit(rty)) => if ls == Plus {
            Some(UnsignedIntLit(rty)) } else { None },
        _ => None,
    }
}

fn add_neg_int(pos: u64, pty: LitIntType, neg: u64, nty: LitIntType) ->
        Option<Constant> {
    if neg > pos {
        unify_int_type(nty, pty, Minus).map(|ty| ConstantInt(neg - pos, ty))
    } else {
        unify_int_type(nty, pty, Plus).map(|ty| ConstantInt(pos - neg, ty))
    }
}

fn sub_int(l: u64, lty: LitIntType, r: u64, rty: LitIntType, neg: bool) ->
        Option<Constant> {
     unify_int_type(lty, rty, if neg { Minus } else { Plus }).and_then(
        |ty| l.checked_sub(r).map(|v| ConstantInt(v, ty)))
}

fn uxx_of(v: u64, cty: LitIntType, tty: UintTy, tmax: u64) ->
        Option<Constant> {
    Some(ConstantInt(if is_negative(cty) { v ^ tmax } else { v } & tmax,
        UnsignedIntLit(tty)))
}

pub fn constant(lcx: &Context, e: &Expr) -> Option<(Constant, bool)> {
    let mut cx = ConstEvalContext { lcx: Some(lcx), needed_resolution: false };
    cx.expr(e).map(|cst| (cst, cx.needed_resolution))
}

pub fn constant_simple(e: &Expr) -> Option<Constant> {
    let mut cx = ConstEvalContext { lcx: None, needed_resolution: false };
    cx.expr(e)
}

struct ConstEvalContext<'c, 'cc: 'c> {
    lcx: Option<&'c Context<'c, 'cc>>,
    needed_resolution: bool
}

impl<'c, 'cc> ConstEvalContext<'c, 'cc> {
    /// simple constant folding: Insert an expression, get a constant or none.
    fn expr(&mut self, e: &Expr) -> Option<Constant> {
        match e.node {
            ExprParen(ref inner) => self.expr(inner),
            ExprPath(_, _) => with_follow(self.lcx, e, self.needed_resolution,
                |ce, res| if ce == e { None } else {
                    self.needed_resolution = res;
                    self.expr(ce)
                }),
            ExprBlock(ref block) => self.block(block),
            ExprIf(ref cond, ref then, ref otherwise) =>
                self.ifthenelse(cond, then, otherwise),
            ExprLit(ref lit) => Some(lit_to_constant(&lit.node)),
            ExprVec(ref vec) => self.multi(vec).map(ConstantVec),
            ExprTup(ref tup) => self.multi(tup).map(ConstantTuple),
            ExprRepeat(ref value, ref number) =>
                self.binop_apply(value, number, |v, n|
                    Some(ConstantRepeat(Box::new(v), n.as_u64() as usize))),
            ExprUnary(op, ref operand) => self.expr(operand).and_then(
                |o| match op {
                    UnNot => constant_not(o),
                    UnNeg => constant_negate(o),
                    UnUniq | UnDeref => Some(o),
                }),
            ExprBinary(op, ref left, ref right) => self.binop(op, left, right),
            ExprCast(ref value, ref ty) => self.cast(value, ty),
            ExprIndex(ref x, ref index) => {
                with_follow(self.lcx, x, self.needed_resolution, |vec, res|
                    if let ExprVec(ref v) = vec.node {
                        if let Some(i) = self.expr(index) {
                            self.needed_resolution = res;
                            self.index(v, i.as_u64() as usize)
                        } else { None }
                    } else { None }
                )
            },
            ExprTupField(ref x, ref index) => {
                with_follow(self.lcx, x, self.needed_resolution, |tup, res|
                    if let ExprTup(ref t) = tup.node {
                        self.needed_resolution = res;
                        self.index(t, index.node)
                    } else { None },
                )
            },
            //TODO: add other expressions
            //ExprStruct(Path, Vec<Field>, Option<P<Expr>>),
            //ExprField(P<Expr>, SpannedIdent),
            //ExprRange(Option<P<Expr>>, Option<P<Expr>>),
            //ExprBox(Option<P<Expr>>, P<Expr>),
            //ExprAddrOf(Mutability, P<Expr>),
            //ExprCall? for enum variants
            _ => None,
        }
    }

    /// A block can only yield a constant if it only has one constant expression
    fn block(&mut self, block: &Block) -> Option<Constant> {
        if block.stmts.is_empty() {
            block.expr.as_ref().and_then(|ref b| self.expr(b))
        } else { None }
    }


    fn index(&mut self, vec: &[P<Expr>], idx: usize) -> Option<Constant> {
        if idx < vec.len() { self.expr(&*vec[idx]) } else { None }
    }

    /// create `Some(Vec![..])` of all constants, unless there is any
    /// non-constant part
    fn multi<E: Deref<Target=Expr> + Sized>(&mut self, vec: &[E]) ->
            Option<Vec<Constant>> {
        vec.iter().map(|elem| self.expr(elem))
                  .collect::<Option<_>>()
    }

    fn ifthenelse(&mut self, cond: &Expr, then: &Block, otherwise: &Option<P<Expr>>)
                  -> Option<Constant> {
        if let Some(ConstantBool(b)) = self.expr(cond) {
            if b {
                self.block(then)
            } else {
                otherwise.as_ref().and_then(|expr| self.expr(expr))
            }
        } else { None }
    }

    fn binop(&mut self, op: BinOp, left: &Expr, right: &Expr) -> Option<Constant> {
        match op.node {
            BiAdd => self.binop_apply(left, right, |l, r|
                match (l, r) {
                    (ConstantByte(l8), ConstantByte(r8)) =>
                        l8.checked_add(r8).map(ConstantByte),
                    (ConstantInt(l64, lty), ConstantInt(r64, rty)) => {
                        let (ln, rn) = (is_negative(lty), is_negative(rty));
                        if ln == rn {
                            unify_int_type(lty, rty, if ln { Minus } else { Plus })
                                .and_then(|ty| l64.checked_add(r64).map(
                                    |v| ConstantInt(v, ty)))
                        } else {
                            if ln {
                                add_neg_int(r64, rty, l64, lty)
                            } else {
                                add_neg_int(l64, lty, r64, rty)
                            }
                        }
                    },
                    // TODO: float (would need bignum library?)
                    _ => None
                }),
            BiSub => self.binop_apply(left, right, |l, r|
                match (l, r) {
                    (ConstantByte(l8), ConstantByte(r8)) => if r8 > l8 {
                        None } else { Some(ConstantByte(l8 - r8)) },
                    (ConstantInt(l64, lty), ConstantInt(r64, rty)) =>
                        match (is_negative(lty), is_negative(rty)) {
                            (false, false) => sub_int(l64, lty, r64, rty, r64 > l64),
                            (true, true) => sub_int(l64, lty, r64, rty, l64 > r64),
                            (true, false) => unify_int_type(lty, rty, Minus)
                                .and_then(|ty| l64.checked_add(r64).map(
                                    |v| ConstantInt(v, ty))),
                            (false, true) => unify_int_type(lty, rty, Plus)
                                .and_then(|ty| l64.checked_add(r64).map(
                                    |v| ConstantInt(v, ty))),
                        },
                    _ => None,
                }),
            BiMul => self.divmul(left, right, u64::checked_mul),
            BiDiv => self.divmul(left, right, u64::checked_div),
            //BiRem,
            BiAnd => self.short_circuit(left, right, false),
            BiOr => self.short_circuit(left, right, true),
            BiBitXor => self.bitop(left, right, |x, y| x ^ y),
            BiBitAnd => self.bitop(left, right, |x, y| x & y),
            BiBitOr => self.bitop(left, right, |x, y| (x | y)),
            BiShl => self.bitop(left, right, |x, y| x << y),
            BiShr => self.bitop(left, right, |x, y| x >> y),
            BiEq => self.binop_apply(left, right,
                |l, r| Some(ConstantBool(l == r))),
            BiNe => self.binop_apply(left, right,
                |l, r| Some(ConstantBool(l != r))),
            BiLt => self.cmp(left, right, Less, true),
            BiLe => self.cmp(left, right, Greater, false),
            BiGe => self.cmp(left, right, Less, false),
            BiGt => self.cmp(left, right, Greater, true),
            _ => None
        }
    }

    fn divmul<F>(&mut self, left: &Expr, right: &Expr, f: F)
            -> Option<Constant> where F: Fn(u64, u64) -> Option<u64> {
        self.binop_apply(left, right, |l, r|
            match (l, r) {
                (ConstantInt(l64, lty), ConstantInt(r64, rty)) => {
                    f(l64, r64).and_then(|value|
                        unify_int_type(lty, rty, if is_negative(lty) ==
                                is_negative(rty) { Plus } else { Minus })
                            .map(|ty| ConstantInt(value, ty)))
                },
                _ => None,
            })
    }

    fn bitop<F>(&mut self, left: &Expr, right: &Expr, f: F)
            -> Option<Constant> where F: Fn(u64, u64) -> u64 {
        self.binop_apply(left, right, |l, r| match (l, r) {
            (ConstantBool(l), ConstantBool(r)) =>
                Some(ConstantBool(f(l as u64, r as u64) != 0)),
            (ConstantByte(l8), ConstantByte(r8)) =>
                Some(ConstantByte(f(l8 as u64, r8 as u64) as u8)),
            (ConstantInt(l, lty), ConstantInt(r, rty)) =>
                unify_int_type(lty, rty, Plus).map(|ty| ConstantInt(f(l, r), ty)),
            _ => None
        })
    }

    fn cmp(&mut self, left: &Expr, right: &Expr, ordering: Ordering, b: bool) -> Option<Constant> {
        self.binop_apply(left, right, |l, r| l.partial_cmp(&r).map(|o|
            ConstantBool(b == (o == ordering))))
    }

    fn binop_apply<F>(&mut self, left: &Expr, right: &Expr, op: F) -> Option<Constant>
    where F: Fn(Constant, Constant) -> Option<Constant> {
        if let (Some(lc), Some(rc)) = (self.expr(left), self.expr(right)) {
            op(lc, rc)
        } else { None }
    }

    fn short_circuit(&mut self, left: &Expr, right: &Expr, b: bool) -> Option<Constant> {
        self.expr(left).and_then(|left|
            if let ConstantBool(lbool) = left {
                if lbool == b {
                    Some(left)
                } else {
                    self.expr(right).and_then(|right|
                        if let ConstantBool(_) = right {
                            Some(right)
                        } else { None }
                    )
                }
            } else { None }
        )
    }

    fn cast(&mut self, value: &Expr, ty: &Ty) -> Option<Constant> {
        self.expr(value).and_then(|v| with_ty(ty, |ty_i|
            match (v, ty_i) {
            (ConstantBool(b), "u8") =>
                Some(ConstantInt(b as u64, UnsignedIntLit(TyU8))),
            (ConstantBool(b), "u16") =>
                Some(ConstantInt(b as u64, UnsignedIntLit(TyU16))),
            (ConstantBool(b), "u32") =>
                Some(ConstantInt(b as u64, UnsignedIntLit(TyU32))),
            (ConstantBool(b), "u64") =>
                Some(ConstantInt(b as u64, UnsignedIntLit(TyU64))),
            (ConstantBool(b), "usize") => // no need to guess here
                Some(ConstantInt(b as u64, UnsignedIntLit(TyUs))),

            (ConstantBool(b), "i8") =>
                Some(ConstantInt(b as u64, SignedIntLit(TyI8, Plus))),
            (ConstantBool(b), "i16") =>
                Some(ConstantInt(b as u64, SignedIntLit(TyI16, Plus))),
            (ConstantBool(b), "i32") =>
                Some(ConstantInt(b as u64, SignedIntLit(TyI32, Plus))),
            (ConstantBool(b), "i64") =>
                Some(ConstantInt(b as u64, SignedIntLit(TyI64, Plus))),
            (ConstantBool(b), "isize") => // no need to guess here
                Some(ConstantInt(b as u64, SignedIntLit(TyIs, Plus))),

            (ConstantInt(v, t), "u8") => uxx_of(v, t, TyU8, u8::MAX as u64),
            (ConstantInt(v, t), "u16") => uxx_of(v, t, TyU16, u16::MAX as u64),
            (ConstantInt(v, t), "u32") => uxx_of(v, t, TyU32, u32::MAX as u64),
            (ConstantInt(v, t), "u64") => uxx_of(v, t, TyU64, u64::MAX),
            (ConstantInt(v, t), "usize") =>
                // if it fits in 32 bits, OK, else refuse to guess
                if is_negative(t) || v > ::std::u32::MAX as u64 {
                    None
                } else { Some(ConstantInt(v, UnsignedIntLit(TyUs))) },

            (ConstantInt(v, cty), "i8") => Some(ConstantInt(
                v & 0xFF, SignedIntLit(TyI8, get_sign(cty)))),
            (ConstantInt(v, cty), "i16") => Some(ConstantInt(
                v & 0xFFFF, SignedIntLit(TyI16, get_sign(cty)))),
            (ConstantInt(v, cty), "i32") => Some(ConstantInt(
                v & 0xFFFFFFFF, SignedIntLit(TyI32, get_sign(cty)))),
            (ConstantInt(v, cty), "i64") => Some(ConstantInt(
                v, SignedIntLit(TyI64, get_sign(cty)))),
            (ConstantInt(v, cty), "isize") => clamp(v, is_negative(cty),
                32).map(|v| ConstantInt(v, SignedIntLit(TyIs, get_sign(cty)))),
            _ => None, //TODO: is it better to ignore casts?
        }))
    }
}
