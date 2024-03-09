use crate::FxHashSet;
use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_ast::UnOp;
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, CtorOf, DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{walk_block, walk_expr, walk_local, Visitor};
use rustc_hir::{Block, ByRef, Expr, ExprKind, HirId, Mutability, Node, PatKind, PathSegment, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::symbol::Ident;

declare_clippy_lint! {
    /// ### What it does
    /// Suggests replace `ref mut` with `ref` when the reference does not need to be a mutable.
    ///
    /// ### Why is this bad?
    /// This reference does not need to be a mutable since it will not change.
    /// It can be replaced by `ref` instead.
    ///
    /// ### Example
    /// ```no_run
    /// let mut s = Some(String::new());
    /// if let Some(ref mut s_ref) = s {
    ///     s_ref.as_str();
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut s = Some(String::new());
    /// if let Some(ref s_ref) = s {
    ///     s_ref.as_str();
    /// }
    /// ```
    #[clippy::version = "1.78.0"]
    pub UNNECESSARY_REF_MUT,
    restriction,
    "Removing unnecessary mutable references in patterns"
}

declare_lint_pass!(UnnecessaryRefMut => [UNNECESSARY_REF_MUT]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryRefMut {
    fn check_pat(&mut self, cx: &LateContext<'_>, pat: &'_ rustc_hir::Pat<'_>) {
        if let PatKind::Binding(annotation, _, ref_mut_ident, _) = pat.kind {
            if !(matches!(annotation.0, ByRef::Yes) && matches!(annotation.1, Mutability::Mut)) {
                return;
            }

            if let Some(block) = parent_block(cx, pat.hir_id) {
                let mut v = InspectUseMutableRefVisitor::new(cx, ref_mut_ident);
                walk_block(&mut v, block);
                if !v.used_as_mut {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_REF_MUT,
                        pat.span,
                        "unnecessary ref mut",
                        "replace with",
                        format!("ref {ref_mut_ident}"),
                        Applicability::MachineApplicable,
                    );
                }
            }
        }
    }
}

fn parent_block<'tcx>(cx: &LateContext<'tcx>, hir_id: HirId) -> Option<&'tcx Block<'tcx>> {
    for (_, node) in cx.tcx.hir().parent_iter(hir_id) {
        if let Node::Block(block) = node {
            return Some(block);
        }
    }
    None
}

struct InspectUseMutableRefVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    used_as_mut: bool,
    bindings: FxHashSet<Ident>,
}

impl<'a, 'tcx> Visitor<'tcx> for InspectUseMutableRefVisitor<'a, 'tcx> {
    fn visit_local(&mut self, local: &'tcx rustc_hir::Local<'tcx>) -> Self::Result {
        if let Some(init) = local.init
            && let Some(rhs_ident) = extract_first_ident(init)
            && self.bindings.contains(&rhs_ident)
            && let PatKind::Binding(_, _, lhs_ident, _) = local.pat.kind
        {
            self.bindings.insert(lhs_ident);
        }

        walk_local(self, local);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> Self::Result {
        match expr.kind {
            ExprKind::Let(let_expr)
                if let PatKind::Path(QPath::Resolved(_, path)) = let_expr.pat.kind
                    && let Some(lhs_segment) = path.segments.first()
                    && let Some(rhs_ident) = extract_first_ident(let_expr.init)
                    && self.bindings.contains(&rhs_ident) =>
            {
                self.bindings.insert(lhs_segment.ident);
            },
            ExprKind::Assign(lhs, rhs, _) => {
                if !self.bind_assign(lhs, rhs) {
                    self.inspect_assign(lhs, rhs);
                }
            },
            ExprKind::MethodCall(_, method_expr, args, _) => {
                self.inspect_method_call(method_expr, expr, args);
            },
            ExprKind::Call(fn_expr, args) => {
                self.inspect_call(fn_expr, args);
            },
            _ => {},
        }

        if !self.used_as_mut {
            walk_expr(self, expr);
        }
    }

    fn visit_block(&mut self, b: &'tcx Block<'tcx>) -> Self::Result {
        walk_block(self, b);
    }
}

impl<'a, 'tcx> InspectUseMutableRefVisitor<'a, 'tcx> {
    fn new(cx: &'a LateContext<'tcx>, ident: Ident) -> InspectUseMutableRefVisitor<'a, 'tcx> {
        let mut bindings = FxHashSet::default();
        bindings.insert(ident);
        Self {
            cx,
            used_as_mut: false,
            bindings,
        }
    }

    fn bind_assign(&mut self, lhs: &'tcx Expr<'tcx>, rhs: &'tcx Expr<'tcx>) -> bool {
        if self.set_use_as_mut_if_bind_to_static(lhs) {
            return true;
        }

        if let Some(rhs_ident) = extract_first_ident(rhs)
            && self.bindings.contains(&rhs_ident)
            && let Some(lhs_segment) = extract_first_segment(lhs)
        {
            self.bindings.insert(lhs_segment.ident);
            true
        } else {
            false
        }
    }

    fn set_use_as_mut_if_bind_to_static(&mut self, lhs: &'tcx Expr<'tcx>) -> bool {
        if let ExprKind::Path(QPath::Resolved(_, path)) = lhs.kind
            && let Res::Def(DefKind::Static(Mutability::Mut), _) = path.res
        {
            self.used_as_mut = true;
            true
        } else {
            false
        }
    }

    fn inspect_assign(&mut self, lhs: &'tcx Expr<'tcx>, rhs: &'tcx Expr<'tcx>) {
        if let ExprKind::Unary(UnOp::Deref, lhs_assign) = lhs.kind
            && let Some(lhs_ident) = extract_first_ident(lhs_assign)
        {
            if self.bindings.contains(&lhs_ident) {
                self.used_as_mut = true;
                return;
            }

            if let Some(lhs_ident) = extract_first_ident(lhs_assign)
                && self.bindings.contains(&lhs_ident)
            {
                self.used_as_mut = true;
                return;
            }
        }

        if let ExprKind::Unary(UnOp::Deref, rhs_assign) = rhs.kind
            && let Some(rhs_segment) = extract_first_segment(rhs_assign)
            && self.bindings.contains(&rhs_segment.ident)
        {
            self.used_as_mut = true;
        }
    }

    fn inspect_method_call(&mut self, method_expr: &'tcx Expr<'tcx>, expr: &'tcx Expr<'tcx>, args: &[Expr<'tcx>]) {
        let Some(method_def_id) = self.cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
            return;
        };

        self.inspect_fn_call(method_def_id, args, true);
        if self.used_as_mut {
            return;
        }

        if let Some(ident) = extract_first_ident(method_expr)
            && self.bindings.contains(&ident)
        {
            let method = self.cx.tcx.fn_sig(method_def_id).instantiate_identity();
            let receiver = method.input(0).skip_binder();

            if matches!(receiver.ref_mutability(), Some(Mutability::Mut)) {
                self.used_as_mut = true;
            }
        }
    }

    fn inspect_call(&mut self, fn_expr: &Expr<'tcx>, args: &[Expr<'tcx>]) {
        let ExprKind::Path(ref path) = fn_expr.kind else {
            return;
        };
        let Some(fn_def_id) = self.cx.qpath_res(path, fn_expr.hir_id).opt_def_id() else {
            return;
        };
        self.inspect_fn_call(fn_def_id, args, false);
    }

    fn position_args_index(&self, args: &[Expr<'tcx>]) -> Option<usize> {
        args.iter().position(|arg| match arg.kind {
            ExprKind::Path(QPath::Resolved(_, path)) => path
                .segments
                .iter()
                .any(|segment| self.bindings.contains(&segment.ident)),
            _ => false,
        })
    }

    fn inspect_fn_call(&mut self, fn_def_id: DefId, args: &[Expr<'tcx>], is_method: bool) {
        let Some(arg_index) = self.position_args_index(args) else {
            return;
        };

        let fn_sig = self.cx.tcx.fn_sig(fn_def_id).instantiate_identity();
        let ty_kind = fn_sig.input(arg_index + usize::from(is_method)).skip_binder().kind();
        if matches!(ty_kind, rustc_middle::ty::Ref(_, _, Mutability::Mut)) {
            self.used_as_mut = true;
        }
    }
}

fn extract_first_segment<'tcx>(expr: &Expr<'tcx>) -> Option<&'tcx PathSegment<'tcx>> {
    match expr.kind {
        ExprKind::Path(QPath::Resolved(_, path)) if let Some(segment) = path.segments.first() => Some(segment),
        ExprKind::Call(call, args)
            if let ExprKind::Path(QPath::Resolved(_, path)) = call.kind
                && let Res::Def(DefKind::Ctor(CtorOf::Variant, CtorKind::Fn), _) = path.res
                && !args.is_empty() =>
        {
            extract_first_segment(&args[0])
        },
        _ => None,
    }
}

fn extract_first_ident(expr: &Expr<'_>) -> Option<Ident> {
    extract_first_segment(expr).map(|segment| segment.ident)
}
