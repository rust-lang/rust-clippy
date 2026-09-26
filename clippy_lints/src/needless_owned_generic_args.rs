use clippy_utils::attrs::get_builtin_attr;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::{is_copy, same_type_modulo_regions};
use clippy_utils::{last_path_segment, sym};
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Expr, ExprKind};
use rustc_infer::infer::TyCtxtInferExt as _;
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::ty::{
    self, Clause, ClauseKind, EarlyBinder, FnSig, GenericArg, GenericArgKind, GenericArgsRef, Generics, Ty,
    TypeVisitableExt as _,
};
use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt as _;
use rustc_trait_selection::traits::{Obligation, ObligationCause};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for owned, non-`Copy` values that are passed by value to a function-level
    /// generic type parameter when a shared reference to the value would satisfy all the
    /// bounds on that parameter as well. Also checks for explicit `.clone()` calls in
    /// such argument positions, where borrowing the original value would do.
    ///
    /// ### Why restrict this?
    /// Passing an owned value moves it into the callee, giving up ownership even though
    /// the callee never needed it. Keeping ownership lets the caller reuse the binding
    /// later and avoids `.clone()` calls introduced only to work around the consumed
    /// value.
    ///
    /// ### Known problems
    /// A trait implementation for `T` and one for `&T` are different implementations and
    /// may behave differently, so the suggestion can change which implementation runs.
    /// Moving a value into the callee also changes where the value is dropped, which can
    /// be observable for types whose destruction order matters. Types marked with
    /// `#[clippy::has_significant_drop]` are not linted, but other types with subtle
    /// drop-time behavior still are. This is why the lint is in the `restriction` group
    /// and the suggestion is not guaranteed to be semantics-preserving.
    ///
    /// ### Example
    /// ```no_run
    /// use std::path::{Path, PathBuf};
    ///
    /// fn read(path: impl AsRef<Path>) {}
    ///
    /// let path = PathBuf::from("/tmp");
    /// read(path);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// use std::path::{Path, PathBuf};
    ///
    /// fn read(path: impl AsRef<Path>) {}
    ///
    /// let path = PathBuf::from("/tmp");
    /// read(&path);
    /// ```
    #[clippy::version = "1.100.0"]
    pub NEEDLESS_OWNED_GENERIC_ARGS,
    restriction,
    "passing an owned value to a generic argument that could borrow it instead"
}

impl_lint_pass!(NeedlessOwnedGenericArgs => [NEEDLESS_OWNED_GENERIC_ARGS]);

pub struct NeedlessOwnedGenericArgs;

/// The kind of argument expression that triggered the lint.
#[derive(Clone, Copy)]
enum ArgKind {
    /// An owned value passed by value, e.g. `f(value)`.
    Move,
    /// An explicit clone of an owned value, e.g. `f(value.clone())`.
    Clone,
}

impl<'tcx> LateLintPass<'tcx> for NeedlessOwnedGenericArgs {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }

        let (fn_id, node_args, args, self_offset) = match expr.kind {
            ExprKind::Call(callee, args) => {
                if let ExprKind::Path(ref qpath) = callee.kind
                    && last_path_segment(qpath).args.is_none()
                    && let Res::Def(DefKind::Fn | DefKind::AssocFn, fn_id) =
                        cx.typeck_results().qpath_res(qpath, callee.hir_id)
                {
                    (fn_id, cx.typeck_results().node_args(callee.hir_id), args, 0)
                } else {
                    // Closures, function pointers and other callables do not have a
                    // recoverable generic definition to check predicates against.
                    return;
                }
            },
            ExprKind::MethodCall(name, _, args, _) => {
                if name.args.is_some() {
                    return;
                }
                if let Some(fn_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) {
                    (fn_id, cx.typeck_results().node_args(expr.hir_id), args, 1)
                } else {
                    return;
                }
            },
            _ => return,
        };

        // Cheap HIR-level filter first: only calls with at least one candidate argument pay
        // for the callee's signature and predicate queries.
        let candidates = args
            .iter()
            .enumerate()
            .filter_map(|(i, arg)| borrow_candidate(cx, arg).map(|candidate| (i + self_offset, arg, candidate)))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return;
        }

        let fn_sig = cx
            .tcx
            .fn_sig(fn_id)
            .instantiate_identity()
            .skip_norm_wip()
            .skip_binder();
        let generics = cx.tcx.generics_of(fn_id);
        let in_const_context = cx.tcx.hir_is_inside_const_context(expr.hir_id);
        let clauses = cx
            .tcx
            .param_env(fn_id)
            .caller_bounds()
            // TODO: remove filter once https://github.com/rust-lang/rust/issues/160895 is in stable
            //
            // This sends the old trait solver into unbounded recursion for `[const] Destruct` on
            // self-recursive types
            .filter(|clause| in_const_context || !matches!(clause.kind().skip_binder(), ClauseKind::HostEffect(_)))
            .collect::<Vec<_>>();

        for (input_idx, arg, candidate) in candidates {
            check_arg(
                cx, expr, fn_sig, generics, node_args, &clauses, input_idx, arg, candidate,
            );
        }
    }
}

/// Checks one call argument `arg` that is passed as the `input_idx`th formal parameter of the
/// callee, and lints if the parameter is a function-level generic type parameter that would also
/// accept a shared reference to the argument's value.
#[expect(clippy::too_many_arguments)]
fn check_arg<'tcx>(
    cx: &LateContext<'tcx>,
    call_expr: &'tcx Expr<'tcx>,
    fn_sig: FnSig<'tcx>,
    generics: &Generics,
    node_args: GenericArgsRef<'tcx>,
    clauses: &[Clause<'tcx>],
    input_idx: usize,
    arg: &'tcx Expr<'tcx>,
    candidate: (&'tcx Expr<'tcx>, ArgKind),
) {
    // The formal parameter must be exactly a function-level type parameter, not a nested
    // occurrence (e.g. `Option<T>`) and not an impl/trait-level or `Self` parameter. The latter
    // is excluded because parent generic parameters come first in the generics list.
    let Some(&ty::Param(param)) = fn_sig.inputs().get(input_idx).map(|ty| ty.kind()) else {
        return;
    };
    if (param.index as usize) < generics.parent_count {
        return;
    }

    // The parameter must appear exactly once in the whole signature: an occurrence in another
    // argument would require a coordinated rewrite of both arguments, and an occurrence in the
    // return type would change the type of the call expression itself.
    let param_ty = param.to_ty(cx.tcx);
    if fn_sig
        .inputs_and_output
        .iter()
        .filter(|ty| ty.contains(param_ty))
        .count()
        != 1
    {
        return;
    }

    let (target, kind) = candidate;
    let ty = cx.typeck_results().expr_ty(target);
    if ty.is_ref() || ty.has_infer() || is_copy(cx, ty) {
        return;
    }
    if has_significant_drop_attr(cx, ty, &mut FxHashSet::default()) {
        return;
    }

    // The generic argument the parameter was instantiated with should be exactly the type of the
    // passed expression. Bail out if inference came to a different conclusion, e.g. through a
    // coercion.
    let Some(instantiated) = node_args.get(param.index as usize).and_then(|arg| arg.as_type()) else {
        return;
    };
    if !same_type_modulo_regions(instantiated, ty) {
        return;
    }

    // Replace `P = A` with `P = &A` in the callee's generic arguments and check that all the
    // callee's predicates still hold for the modified instantiation.
    let borrowed_ty = Ty::new_imm_ref(cx.tcx, cx.tcx.lifetimes.re_erased, ty);
    let mut new_args = node_args.to_vec();
    new_args[param.index as usize] = GenericArg::from(borrowed_ty);
    if !modified_predicates_hold(cx, clauses, &new_args) {
        return;
    }

    emit_lint(cx, call_expr, arg, target, kind);
}

/// Determines whether `arg` is an existing reusable value (a local binding or a place rooted at
/// one) that could be borrowed instead, returning the expression to borrow. Explicit calls to
/// `Clone::clone` on such a value are linted as well, in which case the receiver is returned.
fn borrow_candidate<'hir>(cx: &LateContext<'_>, arg: &'hir Expr<'hir>) -> Option<(&'hir Expr<'hir>, ArgKind)> {
    if arg.span.from_expansion() {
        return None;
    }

    if let ExprKind::MethodCall(name, recv, [], _) = arg.kind
        && name.ident.name == sym::clone
        && cx.typeck_results().expr_adjustments(arg).is_empty()
        && let Some(def_id) = cx.typeck_results().type_dependent_def_id(arg.hir_id)
        && cx.tcx.trait_of_assoc(def_id) == cx.tcx.lang_items().clone_trait()
        && is_reusable_place(cx, recv)
    {
        return Some((recv, ArgKind::Clone));
    }

    if is_reusable_place(cx, arg) && cx.typeck_results().expr_adjustments(arg).is_empty() {
        return Some((arg, ArgKind::Move));
    }

    None
}

/// Checks whether `expr` denotes a reusable place: a local variable, or field projections of one.
/// Temporaries such as `String::new()` or `make_value()` are not reusable places.
fn is_reusable_place(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Path(ref qpath) => matches!(cx.typeck_results().qpath_res(qpath, expr.hir_id), Res::Local(_)),
        ExprKind::Field(base, _) => is_reusable_place(cx, base),
        _ => false,
    }
}

/// Checks whether all of the callee's predicates hold for the modified generic arguments, where
/// the linted parameter was replaced by a shared reference to the argument's type.
fn modified_predicates_hold<'tcx>(cx: &LateContext<'tcx>, clauses: &[Clause<'tcx>], args: &[GenericArg<'tcx>]) -> bool {
    let infcx = cx.tcx.infer_ctxt().build(cx.typing_mode());
    clauses.iter().all(|&clause| {
        let clause = EarlyBinder::bind(cx.tcx, clause)
            .instantiate(cx.tcx, args)
            .skip_norm_wip();
        let obligation = Obligation::new(cx.tcx, ObligationCause::dummy(), cx.param_env, clause);
        infcx.predicate_must_hold_modulo_regions(&obligation)
    })
}

/// Checks whether the type or any of its components is marked with
/// `#[clippy::has_significant_drop]`, like `MutexGuard` is. Moving such a value into the callee
/// changes where it is dropped, which is more likely to matter than the ownership transfer itself.
fn has_significant_drop_attr<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, seen: &mut FxHashSet<Ty<'tcx>>) -> bool {
    if let Some(adt) = ty.ty_adt_def()
        && get_builtin_attr(
            #[allow(deprecated)]
            cx.tcx.get_all_attrs(adt.did()),
            sym::has_significant_drop,
        )
        .next()
        .is_some()
    {
        return true;
    }

    if !seen.insert(ty) {
        return false;
    }

    match ty.kind() {
        ty::Adt(adt, args) => {
            // If some field has a significant drop, e.g. `struct S { guard: MutexGuard<'static, ()> }`,
            adt.all_fields()
                .map(|field| field.ty(cx.tcx, args))
                .any(|ty| has_significant_drop_attr(cx, ty.skip_norm_wip(), seen))
            // or if there is no generic lifetime and some generic parameter has a significant drop,
            // e.g. `Box<MutexGuard<'_, ()>>`
            || (args.iter().all(|arg| !matches!(arg.kind(), GenericArgKind::Lifetime(_)))
                && args
                    .iter()
                    .filter_map(GenericArg::as_type)
                    .any(|ty| has_significant_drop_attr(cx, ty, seen)))
        },
        ty::Tuple(tys) => tys.iter().any(|ty| has_significant_drop_attr(cx, ty, seen)),
        ty::Array(ty, _) | ty::Slice(ty) => has_significant_drop_attr(cx, *ty, seen),
        _ => false,
    }
}

fn emit_lint(cx: &LateContext<'_>, call_expr: &Expr<'_>, arg: &Expr<'_>, target: &Expr<'_>, kind: ArgKind) {
    let (msg, help) = match kind {
        ArgKind::Move => (
            "owned value is moved into a generic argument that can borrow it",
            "consider borrowing the value instead",
        ),
        ArgKind::Clone => (
            "cloning this value is unnecessary because the generic argument can borrow it",
            "consider borrowing the original value instead",
        ),
    };
    span_lint_and_then(cx, NEEDLESS_OWNED_GENERIC_ARGS, arg.span, msg, |diag| {
        let mut app = Applicability::MaybeIncorrect;
        let sugg = Sugg::hir_with_context(cx, target, call_expr.span.ctxt(), "..", &mut app);
        diag.span_suggestion(arg.span, help, format!("&{}", sugg.maybe_paren()), app);
    });
}
