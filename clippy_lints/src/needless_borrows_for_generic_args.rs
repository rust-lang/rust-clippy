use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::mir::{PossibleBorrowerMap, enclosing_mir, expr_local, local_assignments, used_exactly_once};
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::snippet_with_context;
use clippy_utils::ty::{build_check_predicates_with_new_ty_closure, implements_trait, is_copy};
use clippy_utils::{DefinedTy, ExprUseNode, expr_use_ctxt, peel_n_hir_expr_refs};
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::{Body, Expr, ExprKind, Path, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::mir::{Rvalue, StatementKind};
use rustc_middle::ty::{self, ParamTy};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for borrow operations (`&`) that are used as a generic argument to a
    /// function when the borrowed value could be used.
    ///
    /// ### Why is this bad?
    /// Suggests that the receiver of the expression borrows
    /// the expression.
    ///
    /// ### Known problems
    /// The lint cannot tell when the implementation of a trait
    /// for `&T` and `T` do different things. Removing a borrow
    /// in such a case can change the semantics of the code.
    ///
    /// ### Example
    /// ```no_run
    /// fn f(_: impl AsRef<str>) {}
    ///
    /// let x = "foo";
    /// f(&x);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn f(_: impl AsRef<str>) {}
    ///
    /// let x = "foo";
    /// f(x);
    /// ```
    #[clippy::version = "1.74.0"]
    pub NEEDLESS_BORROWS_FOR_GENERIC_ARGS,
    style,
    "taking a reference that is going to be automatically dereferenced"
}

pub struct NeedlessBorrowsForGenericArgs<'tcx> {
    /// Stack of (body owner, `PossibleBorrowerMap`) pairs. Used by
    /// [`needless_borrow_count`] to determine when a borrowed expression can instead
    /// be moved.
    possible_borrowers: Vec<(LocalDefId, PossibleBorrowerMap<'tcx, 'tcx>)>,

    // `IntoIterator` for arrays requires Rust 1.53.
    msrv: Msrv,
}
impl_lint_pass!(NeedlessBorrowsForGenericArgs<'_> => [NEEDLESS_BORROWS_FOR_GENERIC_ARGS]);

impl NeedlessBorrowsForGenericArgs<'_> {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            possible_borrowers: Vec::new(),
            msrv: conf.msrv,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for NeedlessBorrowsForGenericArgs<'tcx> {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if matches!(expr.kind, ExprKind::AddrOf(..))
            && !expr.span.from_expansion()
            && let use_cx = expr_use_ctxt(cx, expr)
            && use_cx.same_ctxt
            && !use_cx.is_ty_unified
            && let use_node = use_cx.use_node(cx)
            && let Some(DefinedTy::Mir { def_site_def_id: _, ty }) = use_node.defined_ty(cx)
            && let ty::Param(param_ty) = *ty.skip_binder().kind()
            && let Some((hir_id, fn_id, i)) = match use_node {
                ExprUseNode::MethodArg(_, _, 0) => None,
                ExprUseNode::MethodArg(hir_id, None, i) => cx
                    .typeck_results()
                    .type_dependent_def_id(hir_id)
                    .map(|id| (hir_id, id, i)),
                ExprUseNode::FnArg(
                    &Expr {
                        kind: ExprKind::Path(ref p),
                        hir_id,
                        ..
                    },
                    i,
                ) if !path_has_args(p) => match cx.typeck_results().qpath_res(p, hir_id) {
                    Res::Def(DefKind::Fn | DefKind::Ctor(..) | DefKind::AssocFn, id) => Some((hir_id, id, i)),
                    _ => None,
                },
                _ => None,
            }
            && let count = needless_borrow_count(
                cx,
                &mut self.possible_borrowers,
                fn_id,
                cx.typeck_results().node_args(hir_id),
                i,
                param_ty,
                expr,
                self.msrv,
            )
            && count != 0
        {
            span_lint_and_then(
                cx,
                NEEDLESS_BORROWS_FOR_GENERIC_ARGS,
                expr.span,
                "the borrowed expression implements the required traits",
                |diag| {
                    let mut app = Applicability::MachineApplicable;
                    let snip_span = peel_n_hir_expr_refs(expr, count).0.span;
                    let snip = snippet_with_context(cx, snip_span, expr.span.ctxt(), "..", &mut app).0;
                    diag.span_suggestion(expr.span, "change this to", snip.into_owned(), app);
                },
            );
        }
    }

    fn check_body_post(&mut self, cx: &LateContext<'tcx>, body: &Body<'_>) {
        if self
            .possible_borrowers
            .last()
            .is_some_and(|&(local_def_id, _)| local_def_id == cx.tcx.hir_body_owner_def_id(body.id()))
        {
            self.possible_borrowers.pop();
        }
    }
}

fn path_has_args(p: &QPath<'_>) -> bool {
    match *p {
        QPath::Resolved(_, Path { segments: [.., s], .. }) | QPath::TypeRelative(_, s) => s.args.is_some(),
        QPath::Resolved(..) => false,
    }
}

/// Checks for the number of borrow expressions which can be removed from the given expression
/// where the expression is used as an argument to a function expecting a generic type.
///
/// The following constraints will be checked:
/// * The borrowed expression meets all the generic type's constraints.
/// * The generic type appears only once in the functions signature.
/// * The borrowed value is:
///   - `Copy` itself, or
///   - the only use of a mutable reference, or
///   - not a variable (created by a function call)
#[expect(clippy::too_many_arguments)]
fn needless_borrow_count<'tcx>(
    cx: &LateContext<'tcx>,
    possible_borrowers: &mut Vec<(LocalDefId, PossibleBorrowerMap<'tcx, 'tcx>)>,
    fn_id: DefId,
    callee_args: ty::GenericArgsRef<'tcx>,
    arg_index: usize,
    param_ty: ParamTy,
    mut expr: &Expr<'tcx>,
    msrv: Msrv,
) -> usize {
    let Some(mut check_referent_ty) =
        build_check_predicates_with_new_ty_closure(cx, fn_id, callee_args, arg_index, param_ty, true, msrv)
    else {
        return 0;
    };

    let drop_trait_def_id = cx.tcx.lang_items().drop_trait();

    let mut count = 0;
    while let ExprKind::AddrOf(_, _, referent) = expr.kind {
        if let ExprKind::Field(base, _) = &referent.kind {
            let base_ty = cx.typeck_results().expr_ty(base);
            if drop_trait_def_id.is_some_and(|id| implements_trait(cx, base_ty, id, &[])) {
                break;
            }
        }

        let referent_ty = cx.typeck_results().expr_ty(referent);

        if !(is_copy(cx, referent_ty)
            || referent_ty.is_ref() && referent_used_exactly_once(cx, possible_borrowers, expr)
            || matches!(referent.kind, ExprKind::Call(..) | ExprKind::MethodCall(..)))
        {
            break;
        }

        if !check_referent_ty(referent_ty) {
            break;
        }
        expr = referent;
        count += 1;
    }
    count
}

fn referent_used_exactly_once<'tcx>(
    cx: &LateContext<'tcx>,
    possible_borrowers: &mut Vec<(LocalDefId, PossibleBorrowerMap<'tcx, 'tcx>)>,
    reference: &Expr<'tcx>,
) -> bool {
    if let Some(mir) = enclosing_mir(cx.tcx, reference.hir_id)
        && let Some(local) = expr_local(cx.tcx, reference)
        && let [location] = *local_assignments(mir, local).as_slice()
        && let block_data = &mir.basic_blocks[location.block]
        && let Some(statement) = block_data.statements.get(location.statement_index)
        && let StatementKind::Assign(box (_, Rvalue::Ref(_, _, place))) = statement.kind
        && !place.is_indirect_first_projection()
    {
        let body_owner_local_def_id = cx.tcx.hir_enclosing_body_owner(reference.hir_id);
        if possible_borrowers
            .last()
            .is_none_or(|&(local_def_id, _)| local_def_id != body_owner_local_def_id)
        {
            possible_borrowers.push((body_owner_local_def_id, PossibleBorrowerMap::new(cx, mir)));
        }
        let possible_borrower = &mut possible_borrowers.last_mut().unwrap().1;
        // If `only_borrowers` were used here, the `copyable_iterator::warn` test would fail. The reason is
        // that `PossibleBorrowerVisitor::visit_terminator` considers `place.local` a possible borrower of
        // itself. See the comment in that method for an explanation as to why.
        possible_borrower.bounded_borrowers(&[local], &[local, place.local], place.local, location)
            && used_exactly_once(mir, place.local).unwrap_or(false)
    } else {
        false
    }
}
