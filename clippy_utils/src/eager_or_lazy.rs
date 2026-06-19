//! Utilities for evaluating whether eagerly evaluated expressions can be made lazy and vice versa.
//!
//! Things to consider:
//!  - does the expression have side-effects?
//!  - is the expression computationally expensive?
//!
//! See lints:
//!  - unnecessary-lazy-evaluations
//!  - or-fun-call
//!  - option-if-let-else

use crate::consts::ConstEvalCtxt;
use crate::sym;
use crate::ty::all_predicates_of;
use crate::visitors::is_const_evaluatable;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{BinOpKind, Block, Expr, ExprKind, QPath, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::adjustment::{Adjust, DerefAdjustKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;
use std::{cmp, ops};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EagernessSuggestion {
    // The expression is cheap and should be evaluated eagerly
    Eager,
    // The expression may be cheap, so don't suggested lazy evaluation; or the expression may not be safe to switch to
    // eager evaluation.
    NoChange,
    // The expression is likely expensive and should be evaluated lazily.
    Lazy,
    // The expression cannot be placed into a closure.
    ForceNoChange,
}
impl ops::BitOr for EagernessSuggestion {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        cmp::max(self, rhs)
    }
}
impl ops::BitOrAssign for EagernessSuggestion {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

/// Determine the eagerness of the given function call.
fn fn_eagerness(tcx: TyCtxt<'_>, fn_id: DefId, name: Symbol, have_one_arg: bool) -> EagernessSuggestion {
    use EagernessSuggestion::{Eager, Lazy, NoChange};

    let ty = match tcx.impl_of_assoc(fn_id) {
        Some(id) => tcx.type_of(id).instantiate_identity().skip_norm_wip(),
        None => return Lazy,
    };

    if (matches!(name, sym::is_empty | sym::len) || name.as_str().starts_with("as_")) && have_one_arg {
        if matches!(
            tcx.crate_name(fn_id.krate),
            sym::std | sym::core | sym::alloc | sym::proc_macro
        ) {
            Eager
        } else {
            NoChange
        }
    } else if let ty::Adt(def, subs) = ty.kind() {
        // Types where the only fields are generic types (or references to) with no trait bounds other
        // than marker traits.
        // Due to the limited operations on these types functions should be fairly cheap.
        if def.variants().iter().flat_map(|v| v.fields.iter()).any(|x| {
            matches!(
                tcx.type_of(x.did)
                    .instantiate_identity()
                    .skip_norm_wip()
                    .peel_refs()
                    .kind(),
                ty::Param(_)
            )
        }) && all_predicates_of(tcx, fn_id).all(|(pred, _)| match pred.kind().skip_binder() {
            ty::ClauseKind::Trait(pred) => tcx.trait_def(pred.trait_ref.def_id).is_marker,
            _ => true,
        }) && subs.types().all(|x| matches!(x.peel_refs().kind(), ty::Param(_)))
        {
            // Limit the function to either `(self) -> bool` or `(&self) -> bool`
            match &**tcx
                .fn_sig(fn_id)
                .instantiate_identity()
                .skip_norm_wip()
                .skip_binder()
                .inputs_and_output
            {
                [arg, res] if !arg.is_mutable_ptr() && arg.peel_refs() == ty && res.is_bool() => NoChange,
                _ => Lazy,
            }
        } else {
            Lazy
        }
    } else {
        Lazy
    }
}

fn res_has_significant_drop(res: Res, ecx: &ConstEvalCtxt<'_>, e: &Expr<'_>) -> bool {
    if let Res::Def(DefKind::Ctor(..) | DefKind::Variant | DefKind::Enum | DefKind::Struct, _)
    | Res::SelfCtor(_)
    | Res::SelfTyAlias { .. } = res
    {
        ecx.typeck.expr_ty(e).has_significant_drop(ecx.tcx, ecx.typing_env)
    } else {
        false
    }
}

#[expect(clippy::too_many_lines)]
fn expr_eagerness<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) -> EagernessSuggestion {
    struct V<'tcx> {
        ecx: ConstEvalCtxt<'tcx>,
        eagerness: EagernessSuggestion,
    }

    impl<'tcx> Visitor<'tcx> for V<'tcx> {
        fn visit_expr(&mut self, e: &'tcx Expr<'_>) {
            use EagernessSuggestion::{ForceNoChange, Lazy, NoChange};
            if self.eagerness == ForceNoChange {
                return;
            }

            // Autoderef through a user-defined `Deref` impl can have side-effects,
            // so don't suggest changing it.
            if self
                .ecx
                .typeck
                .expr_adjustments(e)
                .iter()
                .any(|adj| matches!(adj.kind, Adjust::Deref(DerefAdjustKind::Overloaded(_))))
            {
                self.eagerness |= NoChange;
                return;
            }

            match e.kind {
                ExprKind::Call(
                    &Expr {
                        kind: ExprKind::Path(ref path),
                        hir_id,
                        ..
                    },
                    args,
                ) => match self.ecx.typeck.qpath_res(path, hir_id) {
                    res @ (Res::Def(DefKind::Ctor(..) | DefKind::Variant, _) | Res::SelfCtor(_)) => {
                        if res_has_significant_drop(res, &self.ecx, e) {
                            self.eagerness = ForceNoChange;
                            return;
                        }
                    },
                    Res::Def(_, id) if self.ecx.tcx.is_promotable_const_fn(id) => (),
                    // No need to walk the arguments here, `is_const_evaluatable` already did
                    Res::Def(..) if is_const_evaluatable(self.ecx.tcx, self.ecx.typeck, e) => {
                        self.eagerness |= NoChange;
                        return;
                    },
                    Res::Def(_, id) => match path {
                        QPath::Resolved(_, p) => {
                            self.eagerness |= fn_eagerness(
                                self.ecx.tcx,
                                id,
                                p.segments.last().unwrap().ident.name,
                                !args.is_empty(),
                            );
                        },
                        QPath::TypeRelative(_, name) => {
                            self.eagerness |= fn_eagerness(self.ecx.tcx, id, name.ident.name, !args.is_empty());
                        },
                    },
                    _ => self.eagerness = Lazy,
                },
                // No need to walk the arguments here, `is_const_evaluatable` already did
                ExprKind::MethodCall(..) if is_const_evaluatable(self.ecx.tcx, self.ecx.typeck, e) => {
                    self.eagerness |= NoChange;
                    return;
                },
                #[expect(clippy::match_same_arms)] // arm pattern can't be merged due to `ref`, see rust#105778
                ExprKind::Struct(path, ..) => {
                    if res_has_significant_drop(self.ecx.typeck.qpath_res(path, e.hir_id), &self.ecx, e) {
                        self.eagerness = ForceNoChange;
                        return;
                    }
                },
                ExprKind::Path(ref path) => {
                    if res_has_significant_drop(self.ecx.typeck.qpath_res(path, e.hir_id), &self.ecx, e) {
                        self.eagerness = ForceNoChange;
                        return;
                    }
                },
                ExprKind::MethodCall(name, ..) => {
                    self.eagerness |= self
                        .ecx
                        .typeck
                        .type_dependent_def_id(e.hir_id)
                        .map_or(Lazy, |id| fn_eagerness(self.ecx.tcx, id, name.ident.name, true));
                },
                ExprKind::Index(_, e, _) => {
                    let ty = self.ecx.typeck.expr_ty_adjusted(e);
                    if self.ecx.tcx.type_is_copy_modulo_regions(self.ecx.typing_env, ty) && !ty.is_ref() {
                        self.eagerness |= NoChange;
                    } else {
                        self.eagerness = Lazy;
                    }
                },

                // Both binary and unary operations have cases which panic for integer types.
                // For such cases we will only suggest eager evaluation if the overflow would be
                // caught by rustc's `arithmetic_overflow` or `unconditional_panic` lints.
                ExprKind::Unary(op, e) => {
                    let ty = self.ecx.typeck.expr_ty(e).kind();
                    match op {
                        UnOp::Neg => match *ty {
                            ty::Int(_) if self.ecx.eval(e).is_none() => self.eagerness |= NoChange,
                            ty::Int(_) | ty::Float(_) => {},
                            _ => self.eagerness = Lazy,
                        },
                        UnOp::Deref => match *ty {
                            ty::Adt(def, _) if def.is_box() => {},
                            ty::Ref(..) => {},
                            // Raw pointer dereferences have validity invariants which may not be
                            // met if moved earlier. Everything else is a custom deref which should be
                            // cheap, but we don't know for sure.
                            _ => self.eagerness |= NoChange,
                        },
                        UnOp::Not => match *ty {
                            ty::Int(_) | ty::Uint(_) | ty::Bool => {},
                            _ => self.eagerness |= NoChange,
                        },
                    }
                },
                ExprKind::Binary(op, lhs, rhs) => {
                    let lhs_ty = self.ecx.typeck.expr_ty(lhs);
                    let rhs_ty = self.ecx.typeck.expr_ty(rhs);
                    let same_ty = lhs_ty == rhs_ty;
                    match op.node {
                        BinOpKind::Shl | BinOpKind::Shr => match (lhs_ty.kind(), rhs_ty.kind()) {
                            (ty::Int(_) | ty::Uint(_), ty::Int(_) | ty::Uint(_)) if self.ecx.eval(rhs).is_none() => {
                                self.eagerness |= NoChange;
                            },
                            (ty::Int(_) | ty::Uint(_), ty::Int(_) | ty::Uint(_)) => {},
                            _ => self.eagerness = Lazy,
                        },
                        BinOpKind::Div | BinOpKind::Rem => match *lhs_ty.kind() {
                            ty::Uint(_) if same_ty && self.ecx.eval(rhs).is_none() => self.eagerness |= NoChange,
                            ty::Int(ty)
                                if same_ty
                                    && self.ecx.eval(rhs).is_none_or(|rhs| {
                                        rhs.to_int(self.ecx.tcx, ty) == Some(-1) && self.ecx.eval(lhs).is_none()
                                    }) =>
                            {
                                self.eagerness |= NoChange;
                            },
                            ty::Uint(_) | ty::Int(_) | ty::Float(_) if same_ty => {},
                            _ => self.eagerness = Lazy,
                        },
                        BinOpKind::Add | BinOpKind::Sub | BinOpKind::Mul => match *lhs_ty.kind() {
                            ty::Int(_) | ty::Uint(_)
                                if same_ty && (self.ecx.eval(lhs).is_none() || self.ecx.eval(rhs).is_none()) =>
                            {
                                self.eagerness |= NoChange;
                            },
                            ty::Int(_) | ty::Uint(_) | ty::Float(_) if same_ty => {},
                            _ => self.eagerness = Lazy,
                        },
                        BinOpKind::BitAnd
                        | BinOpKind::BitOr
                        | BinOpKind::BitXor
                        | BinOpKind::Eq
                        | BinOpKind::Ne
                        | BinOpKind::Lt
                        | BinOpKind::Le
                        | BinOpKind::Gt
                        | BinOpKind::Ge
                        | BinOpKind::Or
                        | BinOpKind::And => match *lhs_ty.kind() {
                            ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) if same_ty => {},
                            ty::RawPtr(..) if rhs_ty.is_raw_ptr() => {},
                            _ => self.eagerness = Lazy,
                        },
                    }
                },

                // Can't be moved into a closure
                ExprKind::Break(..)
                | ExprKind::Continue(_)
                | ExprKind::Ret(_)
                | ExprKind::Become(_)
                | ExprKind::InlineAsm(_)
                | ExprKind::Yield(..)
                | ExprKind::Err(_) => {
                    self.eagerness = ForceNoChange;
                    return;
                },

                ExprKind::Loop(..) | ExprKind::Call(..) => {
                    self.eagerness = Lazy;
                },

                ExprKind::ConstBlock(_)
                | ExprKind::Array(_)
                | ExprKind::Tup(_)
                | ExprKind::Use(..)
                | ExprKind::Lit(_)
                | ExprKind::Cast(..)
                | ExprKind::Type(..)
                | ExprKind::DropTemps(_)
                | ExprKind::Let(..)
                | ExprKind::If(..)
                | ExprKind::Match(..)
                | ExprKind::Closure { .. }
                | ExprKind::Field(..)
                | ExprKind::AddrOf(..)
                | ExprKind::Repeat(..)
                | ExprKind::Block(Block { stmts: [], .. }, _)
                | ExprKind::OffsetOf(..)
                | ExprKind::UnsafeBinderCast(..) => (),

                // Assignment might be to a local defined earlier, so don't eagerly evaluate.
                // Blocks with multiple statements might be expensive, so don't eagerly evaluate.
                // TODO: Actually check if either of these are true here.
                ExprKind::Assign(..) | ExprKind::AssignOp(..) | ExprKind::Block(..) => self.eagerness |= NoChange,
            }
            walk_expr(self, e);
        }
    }

    let mut v = V {
        ecx: ConstEvalCtxt::new(cx),
        eagerness: EagernessSuggestion::Eager,
    };
    v.visit_expr(e);
    v.eagerness
}

/// Whether the given expression should be changed to evaluate eagerly
pub fn switch_to_eager_eval<'tcx>(cx: &'_ LateContext<'tcx>, expr: &'tcx Expr<'_>) -> bool {
    expr_eagerness(cx, expr) == EagernessSuggestion::Eager
}

/// Whether the given expression should be changed to evaluate lazily
pub fn switch_to_lazy_eval<'tcx>(cx: &'_ LateContext<'tcx>, expr: &'tcx Expr<'_>) -> bool {
    expr_eagerness(cx, expr) == EagernessSuggestion::Lazy
}
