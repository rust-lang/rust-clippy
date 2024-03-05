use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{fn_def_id, is_from_proc_macro, is_lint_allowed};
use hir::intravisit::{walk_expr, Visitor};
use hir::{ClosureKind, Expr, ExprKind, FnRetTy, FnSig, ItemKind, Node, Ty, TyKind};
use rustc_ast::Label;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::{LateContext, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_span::Span;

use super::INFINITE_LOOP;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    loop_block: &'tcx hir::Block<'_>,
    label: Option<Label>,
) {
    if is_lint_allowed(cx, INFINITE_LOOP, expr.hir_id) {
        return;
    }

    // Skip check if this loop is not in a function/method/closure. (In some weird case)
    let Some(parent_fn_ret) = get_parent_fn_ret_ty(cx, expr) else {
        return;
    };
    if parent_fn_ret.is_never() || in_external_macro(cx.sess(), expr.span) || is_from_proc_macro(cx, expr) {
        return;
    }

    let mut loop_visitor = LoopVisitor {
        cx,
        label,
        is_finite: false,
        loop_depth: 0,
    };
    loop_visitor.visit_block(loop_block);

    let is_finite_loop = loop_visitor.is_finite;

    if !is_finite_loop {
        span_lint_and_then(cx, INFINITE_LOOP, expr.span, "infinite loop detected", |diag| {
            if let Some(span) = parent_fn_ret.sugg_span() {
                diag.span_suggestion(
                    span,
                    "if this is intentional, consider specifying `!` as function return",
                    " -> !",
                    Applicability::MaybeIncorrect,
                );
            } else {
                diag.help("if this is not intended, try adding a `break` or `return` condition in the loop");
            }
        });
    }
}

fn get_parent_fn_ret_ty<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>) -> Option<RetTy<'tcx>> {
    for (_, parent_node) in cx.tcx.hir().parent_iter(expr.hir_id) {
        match parent_node {
            // Skip `Coroutine`, these are the body of `async fn`, not the async closures.
            // This is because we still need to backtrack one parent node to get the `OpaqueDef` ty.
            Node::Expr(Expr {
                kind:
                    ExprKind::Closure(hir::Closure {
                        kind: ClosureKind::Coroutine(_),
                        ..
                    }),
                ..
            }) => (),
            Node::Item(hir::Item {
                kind: ItemKind::Fn(FnSig { decl, .. }, _, _),
                ..
            })
            | Node::TraitItem(hir::TraitItem {
                kind: hir::TraitItemKind::Fn(FnSig { decl, .. }, _),
                ..
            })
            | Node::ImplItem(hir::ImplItem {
                kind: hir::ImplItemKind::Fn(FnSig { decl, .. }, _),
                ..
            })
            | Node::Expr(Expr {
                kind: ExprKind::Closure(hir::Closure { fn_decl: decl, .. }),
                ..
            }) => return Some(RetTy::from_fn_ret_ty(cx, decl.output)),
            _ => (),
        }
    }
    None
}

struct LoopVisitor<'hir, 'tcx> {
    cx: &'hir LateContext<'tcx>,
    label: Option<Label>,
    loop_depth: usize,
    is_finite: bool,
}

impl<'hir> Visitor<'hir> for LoopVisitor<'hir, '_> {
    fn visit_expr(&mut self, ex: &'hir Expr<'_>) {
        match &ex.kind {
            ExprKind::Break(hir::Destination { label, .. }, ..) => {
                // Assuming breaks the loop when `loop_depth` is 0,
                // as it could only means this `break` breaks current loop or any of its upper loop.
                // Or, the depth is not zero but the label is matched.
                if self.loop_depth == 0 || (label.is_some() && *label == self.label) {
                    self.is_finite = true;
                }
            },
            ExprKind::Ret(..) => self.is_finite = true,
            ExprKind::Loop(..) => {
                self.loop_depth += 1;
                walk_expr(self, ex);
                self.loop_depth = self.loop_depth.saturating_sub(1);
            },
            _ => {
                // Calls to a function that never return
                if let Some(did) = fn_def_id(self.cx, ex) {
                    let fn_ret_ty = self.cx.tcx.fn_sig(did).skip_binder().output().skip_binder();
                    if fn_ret_ty.is_never() {
                        self.is_finite = true;
                        return;
                    }
                }
                walk_expr(self, ex);
            },
        }
    }
}

/// Similar to [`FnRetTy`], but reveals the actual type of an `OpaqueDef`.
enum RetTy<'hir> {
    DefaultReturn(Span),
    Return(Ty<'hir>),
}

impl<'hir> RetTy<'hir> {
    fn from_fn_ret_ty(cx: &LateContext<'hir>, fn_ret_ty: FnRetTy<'hir>) -> Self {
        /// Reveal and return the related type of an `opaque`, return `None` if the
        /// given `ty` is not an `OpaqueDef`.
        fn inner_<'tcx>(cx: &LateContext<'tcx>, ty: &Ty<'tcx>) -> Option<Ty<'tcx>> {
            /// Visitor to find the type binding.
            struct BindingVisitor<'tcx> {
                res: Option<Ty<'tcx>>,
            }
            impl<'tcx> Visitor<'tcx> for BindingVisitor<'tcx> {
                fn visit_assoc_type_binding(&mut self, type_binding: &'tcx hir::TypeBinding<'tcx>) {
                    if self.res.is_some() {
                        return;
                    }
                    if let hir::TypeBindingKind::Equality {
                        term: hir::Term::Ty(ty),
                    } = type_binding.kind
                    {
                        self.res = Some(*ty);
                    }
                }
            }

            let TyKind::OpaqueDef(item_id, ..) = ty.kind else {
                return None;
            };
            let opaque_ty_item = cx.tcx.hir().item(item_id);

            // Sinces the `item_id` is from a `TyKind::OpaqueDef`,
            // therefore the `Item` related to it should always be `OpaqueTy`.
            assert!(matches!(opaque_ty_item.kind, ItemKind::OpaqueTy(_)));

            let mut vis = BindingVisitor { res: None };
            vis.visit_item(opaque_ty_item);
            vis.res
        }

        match fn_ret_ty {
            FnRetTy::DefaultReturn(span) => Self::DefaultReturn(span),
            FnRetTy::Return(ty) => Self::Return(inner_(cx, ty).unwrap_or(*ty)),
        }
    }
    /// Returns the span to where the suggestion should be.
    fn sugg_span(&self) -> Option<Span> {
        match self {
            Self::DefaultReturn(span) => Some(*span),
            Self::Return(ty) if matches!(ty.kind, TyKind::Tup(&[])) => Some(ty.span),
            Self::Return(_) => None,
        }
    }
    fn is_never(&self) -> bool {
        let Self::Return(ty) = self else { return false };
        matches!(ty.kind, TyKind::Never)
    }
}
