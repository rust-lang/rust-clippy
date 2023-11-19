use clippy_utils::consts::{constant_simple, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::PanicExpn;
use clippy_utils::source::{snippet_with_applicability, snippet_with_context};
use clippy_utils::{
    can_move_expr_to_closure, is_else_clause, is_from_proc_macro, is_never_expr, is_res_lang_ctor, path_res,
    peel_blocks, RequiresSemi, SpanlessEq,
};
use core::mem;
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{BinOpKind, Expr, ExprKind, LangItem, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::ty::{self, Ty, TypeckResults};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{sym, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual implementations of `checked_*` integer functions.
    ///
    /// ### Why is this bad?
    /// A call to one of the `checked_*` is clearer.
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.75.0"]
    pub MANUAL_CHECKED_OP,
    complexity,
    "manual implementation of a checked operator"
}

declare_lint_pass!(ManualCheckedOp => [MANUAL_CHECKED_OP]);

impl<'tcx> LateLintPass<'tcx> for ManualCheckedOp {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if let Some((cond, then, else_kind, else_)) = parse_branch_exprs(cx, e)
            && let (is_option, then) = try_unwrap_some(cx, then)
            && let ExprKind::Binary(op, lhs, rhs) = then.kind
            && let typeck = cx.typeck_results()
            && let ty = typeck.expr_ty(lhs)
            && let Some(int_ty) = IntTy::from_ty(ty)
            && let ctxt = e.span.ctxt()
            && then.span.ctxt() == ctxt
            && let (method, lhs, rhs) = match op.node {
                BinOpKind::Shl if is_checked_shift(cx, typeck, int_ty, &cond, rhs) => ("checked_shl", lhs, rhs),
                BinOpKind::Shr if is_checked_shift(cx, typeck, int_ty, &cond, rhs) => ("checked_shr", lhs, rhs),
                BinOpKind::Div if int_ty.is_unsigned() && is_checked_div(cx, typeck, &cond, rhs) => {
                    ("checked_div", lhs, rhs)
                },
                BinOpKind::Rem if int_ty.is_unsigned() && is_checked_div(cx, typeck, &cond, rhs) => {
                    ("checked_rem", lhs, rhs)
                },
                BinOpKind::Sub if is_checked_sub(cx, typeck, int_ty, &cond, lhs, rhs) => ("checked_sub", lhs, rhs),
                BinOpKind::Add if let Some((lhs, rhs)) = is_checked_add(cx, typeck, int_ty, &cond, lhs, rhs) => {
                    ("checked_add", lhs, rhs)
                },
                _ => return,
            }
            && !in_external_macro(cx.sess(), e.span)
            && !is_from_proc_macro(cx, e)
        {
            span_lint_and_then(
                cx,
                MANUAL_CHECKED_OP,
                e.span,
                &format!("manual implementation of `{method}`"),
                |diag| {
                    let mut app = Applicability::MachineApplicable;
                    let lhs_snip = snippet_with_context(cx, lhs.span, ctxt, "..", &mut app).0;
                    let rhs_snip = snippet_with_context(cx, rhs.span, ctxt, "..", &mut app).0;
                    let sugg = if is_option && matches!(else_kind, ElseKind::None) {
                        format!("{lhs_snip}.{method}({rhs_snip})")
                    } else {
                        let panic_kind = if !is_option && let Some(expn) = PanicExpn::parse_at_ctxt(cx, else_, ctxt) {
                            match expn {
                                PanicExpn::Empty => Some(None),
                                PanicExpn::Str(msg) => Some(Some(msg)),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        match panic_kind {
                            Some(None) => format!("{lhs_snip}.{method}({rhs_snip}).unwrap()"),
                            Some(Some(msg)) => {
                                let msg_snip = snippet_with_applicability(cx, msg.span, "..", &mut app);
                                format!("{lhs_snip}.{method}({rhs_snip}).expect({msg_snip})")
                            },
                            None => {
                                assert!(can_move_expr_to_closure(cx, else_).is_some());
                                let else_snip = snippet_with_context(cx, else_.span, ctxt, "..", &mut app).0;
                                if is_option {
                                    format!("{lhs_snip}.{method}({rhs_snip}).or_else(|| {else_snip})")
                                } else {
                                    format!("{lhs_snip}.{method}({rhs_snip}).unwrap_or_else(|| {else_snip})")
                                }
                            },
                        }
                    };
                    let sugg = if is_else_clause(cx.tcx, e) {
                        format!("else {{ {sugg} }}")
                    } else {
                        sugg
                    };
                    diag.span_suggestion(e.span, "try", sugg, app);
                },
            );
        }
    }
}

#[derive(Clone, Copy)]
enum IntTy {
    Int(ty::IntTy),
    Uint(ty::UintTy),
}
impl PartialEq<Ty<'_>> for IntTy {
    fn eq(&self, other: &Ty<'_>) -> bool {
        match (self, other.kind()) {
            (Self::Int(x), ty::Int(y)) => x == y,
            (Self::Uint(x), ty::Uint(y)) => x == y,
            _ => false,
        }
    }
}
impl IntTy {
    fn from_ty(ty: Ty<'_>) -> Option<Self> {
        match *ty.kind() {
            ty::Int(ty) => Some(Self::Int(ty)),
            ty::Uint(ty) => Some(Self::Uint(ty)),
            _ => None,
        }
    }

    fn name_sym(self) -> Symbol {
        match self {
            Self::Int(ty::IntTy::I8) => sym::i8,
            Self::Int(ty::IntTy::I16) => sym::i16,
            Self::Int(ty::IntTy::I32) => sym::i32,
            Self::Int(ty::IntTy::I64) => sym::i64,
            Self::Int(ty::IntTy::I128) => sym::i128,
            Self::Int(ty::IntTy::Isize) => sym::isize,
            Self::Uint(ty::UintTy::U8) => sym::u8,
            Self::Uint(ty::UintTy::U16) => sym::u16,
            Self::Uint(ty::UintTy::U32) => sym::u32,
            Self::Uint(ty::UintTy::U64) => sym::u64,
            Self::Uint(ty::UintTy::U128) => sym::u128,
            Self::Uint(ty::UintTy::Usize) => sym::usize,
        }
    }

    fn bits(self) -> Option<u8> {
        match self {
            Self::Int(ty::IntTy::I8) => Some(8),
            Self::Int(ty::IntTy::I16) => Some(16),
            Self::Int(ty::IntTy::I32) => Some(32),
            Self::Int(ty::IntTy::I64) => Some(64),
            Self::Int(ty::IntTy::I128) => Some(128),
            Self::Uint(ty::UintTy::U8) => Some(8),
            Self::Uint(ty::UintTy::U16) => Some(16),
            Self::Uint(ty::UintTy::U32) => Some(32),
            Self::Uint(ty::UintTy::U64) => Some(64),
            Self::Uint(ty::UintTy::U128) => Some(128),
            _ => None,
        }
    }

    fn min(self) -> Option<i128> {
        match self {
            Self::Int(ty::IntTy::I8) => Some(i8::MIN.into()),
            Self::Int(ty::IntTy::I16) => Some(i16::MIN.into()),
            Self::Int(ty::IntTy::I32) => Some(i32::MIN.into()),
            Self::Int(ty::IntTy::I64) => Some(i64::MIN.into()),
            Self::Int(ty::IntTy::I128) => Some(i128::MIN),
            Self::Int(ty::IntTy::Isize) => None,
            Self::Uint(_) => Some(0),
        }
    }

    fn max(self) -> Option<u128> {
        match self {
            Self::Int(ty::IntTy::I8) => Some(i8::MAX as u128),
            Self::Int(ty::IntTy::I16) => Some(i16::MAX as u128),
            Self::Int(ty::IntTy::I32) => Some(i32::MAX as u128),
            Self::Int(ty::IntTy::I64) => Some(i64::MAX as u128),
            Self::Int(ty::IntTy::I128) => Some(i128::MAX as u128),
            Self::Uint(ty::UintTy::U8) => Some(u8::MAX.into()),
            Self::Uint(ty::UintTy::U16) => Some(u16::MAX.into()),
            Self::Uint(ty::UintTy::U32) => Some(u32::MAX.into()),
            Self::Uint(ty::UintTy::U64) => Some(u64::MAX.into()),
            Self::Uint(ty::UintTy::U128) => Some(u128::MAX),
            _ => None,
        }
    }

    fn is_unsigned(self) -> bool {
        matches!(self, Self::Uint(_))
    }
}

fn is_checked_shift<'tcx>(
    cx: &LateContext<'tcx>,
    typeck: &TypeckResults<'tcx>,
    int_ty: IntTy,
    cond: &CmpOp<'tcx>,
    shift_by: &'tcx Expr<'_>,
) -> bool {
    match ConstIntOp::from_expr(cx, typeck, int_ty, cond.rhs) {
        Some(ConstIntOp::Const(c)) => match cond.op {
            OpKind::Lt if Some(c) == int_ty.bits().map(u128::from) => {},
            OpKind::Le if Some(c) == int_ty.bits().map(|x| u128::from(x - 1)) => {},
            _ => return false,
        },
        Some(ConstIntOp::IntConst(IntConst::Bits)) if matches!(cond.op, OpKind::Lt) => {},
        Some(ConstIntOp::Sub(IntConst::Bits, 1)) if matches!(cond.op, OpKind::Le) => {},
        _ => return false,
    };
    SpanlessEq::new(cx).eq_expr(cond.lhs, shift_by)
}

fn is_checked_div<'tcx>(
    cx: &LateContext<'tcx>,
    typeck: &TypeckResults<'tcx>,
    cond: &CmpOp<'tcx>,
    divisor: &'tcx Expr<'_>,
) -> bool {
    if !matches!(cond.op, OpKind::Ne) {
        return false;
    }
    let (const_, other) = if let Some(x) = constant_simple(cx, typeck, cond.lhs) {
        (x, cond.rhs)
    } else if let Some(x) = constant_simple(cx, typeck, cond.rhs) {
        (x, cond.lhs)
    } else {
        return false;
    };
    matches!(const_, Constant::Int(0)) && SpanlessEq::new(cx).eq_expr(other, divisor)
}

fn is_checked_sub<'tcx>(
    cx: &LateContext<'tcx>,
    typeck: &TypeckResults<'tcx>,
    int_ty: IntTy,
    cond: &CmpOp<'tcx>,
    lhs: &'tcx Expr<'_>,
    rhs: &'tcx Expr<'_>,
) -> bool {
    if !matches!(cond.op, OpKind::Lt) {
        return false;
    }
    let limit_eq = if let Some(Constant::Int(rhs_const)) = constant_simple(cx, typeck, rhs)
        && let Some(Constant::Int(cond_const)) = constant_simple(cx, typeck, cond.rhs)
    {
        if int_ty.is_unsigned() {
            rhs_const == cond_const
        } else if rhs_const as i128 > 0
            && let Some(min) = int_ty.min()
        {
            rhs_const as i128 + min == cond_const as i128
        } else {
            return false;
        }
    } else if int_ty.is_unsigned() {
        SpanlessEq::new(cx).eq_expr(cond.rhs, rhs)
    } else {
        return false;
    };
    limit_eq && SpanlessEq::new(cx).eq_expr(cond.lhs, lhs)
}

fn is_checked_add<'tcx>(
    cx: &LateContext<'tcx>,
    typeck: &TypeckResults<'tcx>,
    int_ty: IntTy,
    cond: &CmpOp<'tcx>,
    lhs: &'tcx Expr<'_>,
    rhs: &'tcx Expr<'_>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    let (lhs, rhs, op_const) = if let Some(op_const) = constant_simple(cx, typeck, rhs) {
        (lhs, rhs, op_const)
    } else if let Some(op_const) = constant_simple(cx, typeck, lhs) {
        (rhs, lhs, op_const)
    } else {
        return None;
    };
    let Constant::Int(op_const) = op_const else {
        return None;
    };

    let (cond_const, cond_other, cond_const_is_rhs) = if let Some(cond_const) = constant_simple(cx, typeck, cond.rhs) {
        (cond_const, cond.lhs, true)
    } else if let Some(cond_const) = constant_simple(cx, typeck, cond.lhs) {
        (cond_const, cond.rhs, false)
    } else {
        return None;
    };
    let Constant::Int(cond_const) = cond_const else {
        return None;
    };
    let max = int_ty.max()?;

    let limit_eq = match cond.op {
        OpKind::Lt if cond_const_is_rhs && (int_ty.is_unsigned() || op_const as i128 > 0) => {
            max - op_const == cond_const
        },
        OpKind::Ne => max == cond_const && op_const == 1,
        _ => return None,
    };
    (limit_eq && SpanlessEq::new(cx).eq_expr(cond_other, lhs)).then_some((lhs, rhs))
}

fn parse_branch_exprs<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
) -> Option<(CmpOp<'tcx>, &'tcx Expr<'tcx>, ElseKind, &'tcx Expr<'tcx>)> {
    if let ExprKind::If(cond, then, Some(else_)) = e.kind
        && let Some(mut cond) = CmpOp::parse_expr(cond.peel_drop_temps())
    {
        let then = peel_blocks(then);
        let else_ = peel_blocks(else_);
        if let Some(else_kind) = ElseKind::parse_expr(cx, else_) {
            Some((cond, then, else_kind, else_))
        } else if let Some(else_kind) = ElseKind::parse_expr(cx, then) {
            cond.inv();
            Some((cond, else_, else_kind, then))
        } else {
            None
        }
    } else {
        None
    }
}

enum ElseKind {
    None,
    Diverge(RequiresSemi),
}
impl ElseKind {
    fn parse_expr<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) -> Option<Self> {
        if let Some(semi) = is_never_expr(cx, e) {
            Some(Self::Diverge(semi))
        } else if is_res_lang_ctor(cx, path_res(cx, e), LangItem::OptionNone) {
            Some(Self::None)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
enum OpKind {
    Eq,
    Ne,
    Lt,
    Le,
}
pub struct CmpOp<'tcx> {
    op: OpKind,
    lhs: &'tcx Expr<'tcx>,
    rhs: &'tcx Expr<'tcx>,
}
impl<'tcx> CmpOp<'tcx> {
    fn parse_expr(e: &'tcx Expr<'_>) -> Option<Self> {
        if let ExprKind::Binary(op, lhs, rhs) = e.kind {
            match op.node {
                BinOpKind::Eq => Some(Self {
                    op: OpKind::Eq,
                    lhs,
                    rhs,
                }),
                BinOpKind::Ne => Some(Self {
                    op: OpKind::Ne,
                    lhs,
                    rhs,
                }),
                BinOpKind::Lt => Some(Self {
                    op: OpKind::Lt,
                    lhs,
                    rhs,
                }),
                BinOpKind::Le => Some(Self {
                    op: OpKind::Le,
                    lhs,
                    rhs,
                }),
                BinOpKind::Gt => Some(Self {
                    op: OpKind::Lt,
                    lhs: rhs,
                    rhs: lhs,
                }),
                BinOpKind::Ge => Some(Self {
                    op: OpKind::Le,
                    lhs: rhs,
                    rhs: lhs,
                }),
                _ => None,
            }
        } else {
            None
        }
    }

    fn inv(&mut self) {
        match self.op {
            OpKind::Eq => self.op = OpKind::Ne,
            OpKind::Ne => self.op = OpKind::Eq,
            OpKind::Lt => {
                self.op = OpKind::Le;
                mem::swap(&mut self.lhs, &mut self.rhs);
            },
            OpKind::Le => {
                self.op = OpKind::Lt;
                mem::swap(&mut self.lhs, &mut self.rhs);
            },
        }
    }
}

#[derive(Clone, Copy)]
enum IntConst {
    Bits,
    Min,
    Max,
}
impl IntConst {
    fn from_path<'tcx>(
        cx: &LateContext<'tcx>,
        typeck: &TypeckResults<'tcx>,
        ty: IntTy,
        p: &'tcx QPath<'_>,
    ) -> Option<Self> {
        match p {
            QPath::Resolved(None, path)
                if let Res::Def(DefKind::Const, did) = path.res
                    && let path = cx.get_def_path(did)
                    && let &[sym::core, ty_name, item_name] = &*path
                    && ty_name == ty.name_sym() =>
            {
                match item_name.as_str() {
                    "MIN" => Some(Self::Min),
                    "MAX" => Some(Self::Max),
                    _ => None,
                }
            },
            QPath::TypeRelative(path_ty, name) if typeck.node_type_opt(path_ty.hir_id).is_some_and(|x| ty == x) => {
                match name.ident.as_str() {
                    "MIN" => Some(Self::Min),
                    "MAX" => Some(Self::Max),
                    "BITS" => Some(Self::Bits),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    fn from_expr<'tcx>(
        cx: &LateContext<'tcx>,
        typeck: &TypeckResults<'tcx>,
        ty: IntTy,
        e: &'tcx Expr<'_>,
    ) -> Option<Self> {
        if let ExprKind::Path(p) = &e.kind {
            Self::from_path(cx, typeck, ty, p)
        } else {
            None
        }
    }
}

enum ConstIntOp {
    Const(u128),
    IntConst(IntConst),
    Add(IntConst, u128),
    Sub(IntConst, u128),
}
impl ConstIntOp {
    fn from_expr<'tcx>(
        cx: &LateContext<'tcx>,
        typeck: &TypeckResults<'tcx>,
        ty: IntTy,
        e: &'tcx Expr<'_>,
    ) -> Option<Self> {
        if let Some(c) = constant_simple(cx, typeck, e) {
            if let Constant::Int(c) = c {
                Some(Self::Const(c))
            } else {
                None
            }
        } else {
            match e.kind {
                ExprKind::Path(ref p) if let Some(c) = IntConst::from_path(cx, typeck, ty, p) => {
                    Some(Self::IntConst(c))
                },
                ExprKind::Binary(op, lhs, rhs) if matches!(op.node, BinOpKind::Add) => {
                    if let Some(lhs) = IntConst::from_expr(cx, typeck, ty, lhs) {
                        if let Some(Constant::Int(rhs)) = constant_simple(cx, typeck, rhs) {
                            Some(Self::Add(lhs, rhs))
                        } else {
                            None
                        }
                    } else if let Some(rhs) = IntConst::from_expr(cx, typeck, ty, rhs)
                        && let Some(Constant::Int(lhs)) = constant_simple(cx, typeck, lhs)
                    {
                        Some(Self::Add(rhs, lhs))
                    } else {
                        None
                    }
                },
                ExprKind::Binary(op, lhs, rhs)
                    if matches!(op.node, BinOpKind::Sub)
                        && let Some(lhs) = IntConst::from_expr(cx, typeck, ty, lhs)
                        && let Some(Constant::Int(rhs)) = constant_simple(cx, typeck, rhs) =>
                {
                    Some(Self::Sub(lhs, rhs))
                },
                _ => None,
            }
        }
    }
}

fn try_unwrap_some<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) -> (bool, &'tcx Expr<'tcx>) {
    if let ExprKind::Call(p, [e]) = e.kind
        && let res = path_res(cx, p)
        && is_res_lang_ctor(cx, res, LangItem::OptionSome)
    {
        (true, peel_blocks(e))
    } else {
        (false, e)
    }
}
