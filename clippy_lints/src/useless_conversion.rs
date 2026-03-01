use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg, span_lint_and_then};
use clippy_utils::res::{MaybeDef, MaybeQPath, MaybeResPath, MaybeTypeckRes};
use clippy_utils::source::{snippet, snippet_with_context};
use clippy_utils::sugg::{DiagExt as _, Sugg};
use clippy_utils::ty::{is_copy, same_type_modulo_regions};
use clippy_utils::{get_parent_expr, higher, is_ty_alias, peel_blocks, sym};
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_hir::{BindingMode, Expr, ExprKind, HirId, Mutability, Node, PatKind};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_infer::traits::Obligation;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::traits::ObligationCause;
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability};
use rustc_middle::ty::{self, EarlyBinder, GenericArg, GenericArgsRef, Ty, TypeVisitableExt};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `Into`, `TryInto`, `From`, `TryFrom`, or `IntoIter` calls
    /// which uselessly convert to the same type.
    ///
    /// ### Why is this bad?
    /// Redundant code.
    ///
    /// ### Example
    /// ```no_run
    /// // format!() returns a `String`
    /// let s: String = format!("hello").into();
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let s: String = format!("hello");
    /// ```
    #[clippy::version = "1.45.0"]
    pub USELESS_CONVERSION,
    complexity,
    "calls to `Into`, `TryInto`, `From`, `TryFrom`, or `IntoIter` which perform useless conversions to the same type"
}

#[derive(Default)]
pub struct UselessConversion {
    try_desugar_arm: Vec<HirId>,
    expn_depth: u32,
}

impl_lint_pass!(UselessConversion => [USELESS_CONVERSION]);

enum MethodOrFunction {
    Method,
    Function,
}

impl MethodOrFunction {
    /// Maps the argument position in `pos` to the parameter position.
    /// For methods, `self` is skipped.
    fn param_pos(self, pos: usize) -> usize {
        match self {
            MethodOrFunction::Method => pos + 1,
            MethodOrFunction::Function => pos,
        }
    }
}
/// Returns `true` if `def_id` is an associated item of the `From` trait.
fn is_from_trait_item(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx
        .trait_of_assoc(def_id)
        .is_some_and(|trait_id| cx.tcx.is_diagnostic_item(sym::From, trait_id))
}

/// Returns `true` if `expr` resolves to the closure parameter bound to `local_id`.
fn is_local_binding(expr: &Expr<'_>, local_id: HirId) -> bool {
    matches!(
        peel_blocks(expr).kind,
        ExprKind::Path(rustc_hir::QPath::Resolved(_, path)) if path.res == Res::Local(local_id)
    )
}

/// Checks if the argument to `map_err` is a `From::from` or `Into::into` conversion.
///
/// This detects three patterns:
/// - Direct function paths: `From::from`, `MyError::from` (verified via `trait_of_assoc`)
/// - Closures calling `From::from`: `|e| MyError::from(e)`
/// - Closures calling `.into()`: `|e| e.into()`
///
/// Uses `trait_of_assoc` rather than name-based matching to avoid false positives
/// from inherent `from()` methods that are not the `From` trait.
fn is_from_or_into_conversion<'tcx>(cx: &LateContext<'tcx>, arg: &'tcx Expr<'_>) -> bool {
    let arg = peel_blocks(arg);

    // Direct function path: `From::from` or `MyError::from`
    if let Some(def_id) = arg.res(cx).opt_def_id()
        && is_from_trait_item(cx, def_id)
    {
        return true;
    }

    // Single-parameter closure whose body is a From/Into call on that parameter
    if let ExprKind::Closure(closure) = arg.kind
        && let body = cx.tcx.hir_body(closure.body)
        && let [param] = body.params
        && let PatKind::Binding(_, local_id, ..) = param.pat.kind
    {
        let value = peel_blocks(body.value);

        // `|e| MyError::from(e)`
        if let ExprKind::Call(func, [from_arg]) = value.kind
            && let Some(def_id) = func.res(cx).opt_def_id()
            && is_from_trait_item(cx, def_id)
            && is_local_binding(from_arg, local_id)
        {
            return true;
        }

        // `|e| e.into()`
        if let ExprKind::MethodCall(name, recv, [], _) = value.kind
            && name.ident.name == sym::into
            && cx.ty_based_def(value).opt_parent(cx).is_diag_item(cx, sym::Into)
            && is_local_binding(recv, local_id)
        {
            return true;
        }
    }

    false
}

/// Returns the span of the `IntoIterator` trait bound in the function pointed to by `fn_did`,
/// iff all of the bounds also hold for the type of the `.into_iter()` receiver.
/// ```ignore
/// pub fn foo<I>(i: I)
/// where I: IntoIterator<Item=i32> + ExactSizeIterator
///                                   ^^^^^^^^^^^^^^^^^ this extra bound stops us from suggesting to remove `.into_iter()` ...
/// {
///     assert_eq!(i.len(), 3);
/// }
///
/// pub fn bar() {
///     foo([1, 2, 3].into_iter());
///                  ^^^^^^^^^^^^ ... here, because `[i32; 3]` is not `ExactSizeIterator`
/// }
/// ```
fn into_iter_bound<'tcx>(
    cx: &LateContext<'tcx>,
    fn_did: DefId,
    into_iter_did: DefId,
    into_iter_receiver: Ty<'tcx>,
    param_index: u32,
    node_args: GenericArgsRef<'tcx>,
) -> Option<Span> {
    let mut into_iter_span = None;

    for (pred, span) in cx.tcx.explicit_predicates_of(fn_did).predicates {
        if let ty::ClauseKind::Trait(tr) = pred.kind().skip_binder()
            && tr.self_ty().is_param(param_index)
        {
            if tr.def_id() == into_iter_did {
                into_iter_span = Some(*span);
            } else {
                let tr = cx.tcx.erase_and_anonymize_regions(tr);
                if tr.has_escaping_bound_vars() {
                    return None;
                }

                // Substitute generics in the predicate and replace the IntoIterator type parameter with the
                // `.into_iter()` receiver to see if the bound also holds for that type.
                let args = cx.tcx.mk_args_from_iter(node_args.iter().enumerate().map(|(i, arg)| {
                    if i == param_index as usize {
                        GenericArg::from(into_iter_receiver)
                    } else {
                        arg
                    }
                }));

                let predicate = EarlyBinder::bind(tr).instantiate(cx.tcx, args);
                let obligation = Obligation::new(cx.tcx, ObligationCause::dummy(), cx.param_env, predicate);
                if !cx
                    .tcx
                    .infer_ctxt()
                    .build(cx.typing_mode())
                    .predicate_must_hold_modulo_regions(&obligation)
                {
                    return None;
                }
            }
        }
    }

    into_iter_span
}

/// Extracts the receiver of a `.into_iter()` method call.
fn into_iter_call<'hir>(cx: &LateContext<'_>, expr: &'hir Expr<'hir>) -> Option<&'hir Expr<'hir>> {
    if let ExprKind::MethodCall(name, recv, [], _) = expr.kind
        && cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::IntoIterator)
        && name.ident.name == sym::into_iter
    {
        Some(recv)
    } else {
        None
    }
}

/// Same as [`into_iter_call`], but tries to look for the innermost `.into_iter()` call, e.g.:
/// `foo.into_iter().into_iter()`
///  ^^^  we want this expression
fn into_iter_deep_call<'hir>(cx: &LateContext<'_>, mut expr: &'hir Expr<'hir>) -> (&'hir Expr<'hir>, usize) {
    let mut depth = 0;
    while let Some(recv) = into_iter_call(cx, expr) {
        expr = recv;
        depth += 1;
    }
    (expr, depth)
}

/// Checks for `recv.map_err(From::from)?` where the `map_err` is redundant
/// because `?` already performs the same `From` conversion.
///
/// Given `recv.map_err(f)?` in a function returning `Result<T, RetErr>`:
///
/// - `recv` has type `Result<T, E1>`
/// - `map_err(f)` converts to `Result<T, E2>`
/// - `?` then converts `E2` -> `RetErr` via `From`
///
/// If `E2 == RetErr`, the `From` conversion in `?` is the identity and the
/// `map_err` could have been omitted. `?` alone would convert `E1` -> `RetErr`
/// directly (since `From<E1> for E2` exists, and `E2 == RetErr`).
///
/// We skip the case where `E1 == E2` (same input/output error type) because
/// that is already caught by the general same-type `map_err` arm.
fn check_map_err_before_try<'tcx>(cx: &LateContext<'tcx>, try_desugar: &higher::TryDesugar<'tcx>) {
    // Early return if the `try_desugar` doesn't match the expected `recv.map_err(arg)?` pattern.
    let ExprKind::MethodCall(path, recv, [arg], _) = try_desugar.scrutinee.kind else {
        return;
    };

    // Only look at `map_err` calls.
    if path.ident.name != sym::map_err {
        return;
    }

    // Don't lint inside macro expansions where the user can't easily change the code.
    if recv.span.from_expansion() || try_desugar.scrutinee.span.from_expansion() {
        return;
    }

    if !is_from_or_into_conversion(cx, arg) {
        return;
    }

    let Some(map_err_target) = err_ty_of_result(cx, cx.typeck_results().expr_ty(try_desugar.scrutinee)) else {
        return;
    };
    let Some(try_target) = try_err_target_ty(cx) else {
        return;
    };

    // The `map_err` is only redundant when its output error type matches
    // the type that `?` converts to.
    if !same_type_modulo_regions(map_err_target, try_target) {
        return;
    }

    // When input and output error types are identical, the general
    // same-type `map_err` arm already handles it, don't double-lint.
    let Some(recv_err) = err_ty_of_result(cx, cx.typeck_results().expr_ty(recv)) else {
        return;
    };
    if same_type_modulo_regions(recv_err, map_err_target) {
        return;
    }

    let removal_span = try_desugar.scrutinee.span.with_lo(recv.span.hi());
    span_lint_and_then(
        cx,
        USELESS_CONVERSION,
        removal_span,
        "useless conversion to the same error type done via `?`",
        |diag| {
            diag.suggest_remove_item(cx, removal_span, "consider removing", Applicability::MachineApplicable);
        },
    );
}

#[expect(clippy::too_many_lines)]
impl<'tcx> LateLintPass<'tcx> for UselessConversion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        // Handle `?` desugaring before the `from_expansion()` early return.
        //
        // The `?` operator desugars into a `match` expression with
        // `MatchSource::TryDesugar`, whose span has `from_expansion() == true`.
        // We must check it here before we skip all expansions below.
        //
        // After handling the lint we fall through to the `from_expansion()`
        // block so that `expn_depth` tracking stays balanced. This matters
        // when `?` appears inside a macro, where `desugaring_kind()` is `None`
        // and the expression contributes to `expn_depth`.
        if let Some(try_desugar) = higher::TryDesugar::hir(e) {
            check_map_err_before_try(cx, &try_desugar);

            // Track the Ok/Continue arm so we don't lint its inner expressions
            // (they are compiler-generated and not written by the user).
            if let ExprKind::Ret(Some(ret)) | ExprKind::Break(_, Some(ret)) = try_desugar.arms[0].body.kind
                && let ExprKind::Call(_, [arg, ..]) = ret.kind
            {
                self.try_desugar_arm.push(arg.hir_id);
            }
        }

        if e.span.from_expansion() {
            if e.span.desugaring_kind().is_none() {
                self.expn_depth += 1;
            }
            return;
        }

        if Some(&e.hir_id) == self.try_desugar_arm.last() {
            return;
        }

        match e.kind {
            ExprKind::MethodCall(path, recv, [arg], _) => {
                if matches!(
                    path.ident.name,
                    sym::map | sym::map_err | sym::map_break | sym::map_continue
                ) && has_eligible_receiver(cx, recv, e)
                    && matches!(
                        arg.res(cx).assoc_parent(cx).opt_diag_name(cx),
                        Some(sym::Into | sym::From)
                    )
                    && let ty::FnDef(_, args) = cx.typeck_results().expr_ty(arg).kind()
                    && let &[from_ty, to_ty] = args.into_type_list(cx.tcx).as_slice()
                    && same_type_modulo_regions(from_ty, to_ty)
                {
                    span_lint_and_then(
                        cx,
                        USELESS_CONVERSION,
                        e.span.with_lo(recv.span.hi()),
                        format!("useless conversion to the same type: `{from_ty}`"),
                        |diag| {
                            diag.suggest_remove_item(
                                cx,
                                e.span.with_lo(recv.span.hi()),
                                "consider removing",
                                Applicability::MachineApplicable,
                            );
                        },
                    );
                }
            },

            ExprKind::MethodCall(name, recv, [], _) => {
                if cx.ty_based_def(e).opt_parent(cx).is_diag_item(cx, sym::Into) && name.ident.name == sym::into {
                    let a = cx.typeck_results().expr_ty(e);
                    let b = cx.typeck_results().expr_ty(recv);
                    if same_type_modulo_regions(a, b) {
                        let mut app = Applicability::MachineApplicable;
                        let sugg = snippet_with_context(cx, recv.span, e.span.ctxt(), "<expr>", &mut app).0;
                        span_lint_and_sugg(
                            cx,
                            USELESS_CONVERSION,
                            e.span,
                            format!("useless conversion to the same type: `{b}`"),
                            "consider removing `.into()`",
                            sugg.into_owned(),
                            app,
                        );
                    }
                }
                if let Some(into_iter_recv) = into_iter_call(cx, e)
                    // Make sure that there is no parent expression, or if there is, make sure it's not a `.into_iter()` call.
                    // The reason for that is that we only want to lint once (the outermost call)
                    // in cases like `foo.into_iter().into_iter()`
                    && get_parent_expr(cx, e)
                        .and_then(|parent| into_iter_call(cx, parent))
                        .is_none()
                {
                    if let Some(parent) = get_parent_expr(cx, e) {
                        let parent_fn = match parent.kind {
                            ExprKind::Call(recv, args)
                                if let ExprKind::Path(ref qpath) = recv.kind
                                    && let Some(did) = cx.qpath_res(qpath, recv.hir_id).opt_def_id()
                                    // make sure that the path indeed points to a fn-like item, so that
                                    // `fn_sig` does not ICE. (see #11065)
                                    && cx.tcx.def_kind(did).is_fn_like() =>
                            {
                                Some((
                                    did,
                                    args,
                                    cx.typeck_results().node_args(recv.hir_id),
                                    MethodOrFunction::Function,
                                ))
                            },
                            ExprKind::MethodCall(.., args, _) => {
                                cx.typeck_results().type_dependent_def_id(parent.hir_id).map(|did| {
                                    (
                                        did,
                                        args,
                                        cx.typeck_results().node_args(parent.hir_id),
                                        MethodOrFunction::Method,
                                    )
                                })
                            },
                            _ => None,
                        };

                        if let Some((parent_fn_did, args, node_args, kind)) = parent_fn
                            && let Some(into_iter_did) = cx.tcx.get_diagnostic_item(sym::IntoIterator)
                            && let sig = cx.tcx.fn_sig(parent_fn_did).skip_binder().skip_binder()
                            && let Some(arg_pos) = args.iter().position(|x| x.hir_id == e.hir_id)
                            && let Some(&into_iter_param) = sig.inputs().get(kind.param_pos(arg_pos))
                            && let ty::Param(param) = into_iter_param.kind()
                            && let Some(span) = into_iter_bound(
                                cx,
                                parent_fn_did,
                                into_iter_did,
                                cx.typeck_results().expr_ty(into_iter_recv),
                                param.index,
                                node_args,
                            )
                            && self.expn_depth == 0
                        {
                            // Get the "innermost" `.into_iter()` call, e.g. given this expression:
                            // `foo.into_iter().into_iter()`
                            //  ^^^
                            let (into_iter_recv, depth) = into_iter_deep_call(cx, into_iter_recv);

                            span_lint_and_then(
                                cx,
                                USELESS_CONVERSION,
                                e.span,
                                "explicit call to `.into_iter()` in function argument accepting `IntoIterator`",
                                |diag| {
                                    let receiver_span = into_iter_recv.span.source_callsite();
                                    let adjustments = adjustments(cx, into_iter_recv);
                                    let mut sugg = if adjustments.is_empty() {
                                        vec![]
                                    } else {
                                        vec![(receiver_span.shrink_to_lo(), adjustments)]
                                    };
                                    let plural = if depth == 0 { "" } else { "s" };
                                    sugg.push((e.span.with_lo(receiver_span.hi()), String::new()));
                                    diag.multipart_suggestion(
                                        format!("consider removing the `.into_iter()`{plural}"),
                                        sugg,
                                        Applicability::MachineApplicable,
                                    );
                                    diag.span_note(span, "this parameter accepts any `IntoIterator`, so you don't need to call `.into_iter()`");
                                },
                            );

                            // Early return to avoid linting again with contradicting suggestions
                            return;
                        }
                    }

                    if let Some(id) = recv.res_local_id()
                        && let Node::Pat(pat) = cx.tcx.hir_node(id)
                        && let PatKind::Binding(ann, ..) = pat.kind
                        && ann != BindingMode::MUT
                    {
                        // Do not remove .into_iter() applied to a non-mutable local variable used in
                        // a larger expression context as it would differ in mutability.
                        return;
                    }

                    let a = cx.typeck_results().expr_ty(e);
                    let b = cx.typeck_results().expr_ty(recv);

                    // If the types are identical then .into_iter() can be removed, unless the type
                    // implements Copy, in which case .into_iter() returns a copy of the receiver and
                    // cannot be safely omitted.
                    if same_type_modulo_regions(a, b) && !is_copy(cx, b) {
                        // Below we check if the parent method call meets the following conditions:
                        // 1. First parameter is `&mut self` (requires mutable reference)
                        // 2. Second parameter implements the `FnMut` trait (e.g., Iterator::any)
                        // For methods satisfying these conditions (like any), .into_iter() must be preserved.
                        if let Some(parent) = get_parent_expr(cx, e)
                            && let ExprKind::MethodCall(_, recv, _, _) = parent.kind
                            && recv.hir_id == e.hir_id
                            && let Some(def_id) = cx.typeck_results().type_dependent_def_id(parent.hir_id)
                            && let sig = cx.tcx.fn_sig(def_id).skip_binder().skip_binder()
                            && let inputs = sig.inputs()
                            && inputs.len() >= 2
                            && let Some(self_ty) = inputs.first()
                            && let ty::Ref(_, _, Mutability::Mut) = self_ty.kind()
                            && let Some(second_ty) = inputs.get(1)
                            && let predicates = cx.tcx.param_env(def_id).caller_bounds()
                            && predicates.iter().any(|pred| {
                                if let ty::ClauseKind::Trait(trait_pred) = pred.kind().skip_binder() {
                                    trait_pred.self_ty() == *second_ty
                                        && cx.tcx.lang_items().fn_mut_trait() == Some(trait_pred.def_id())
                                } else {
                                    false
                                }
                            })
                        {
                            return;
                        }

                        let mut applicability = Applicability::MachineApplicable;
                        let sugg = snippet_with_context(cx, recv.span, e.span.ctxt(), "<expr>", &mut applicability)
                            .0
                            .into_owned();
                        span_lint_and_sugg(
                            cx,
                            USELESS_CONVERSION,
                            e.span,
                            format!("useless conversion to the same type: `{b}`"),
                            "consider removing `.into_iter()`",
                            sugg,
                            applicability,
                        );
                    }
                }
                if cx.ty_based_def(e).opt_parent(cx).is_diag_item(cx, sym::TryInto)
                    && name.ident.name == sym::try_into
                    && let a = cx.typeck_results().expr_ty(e)
                    && let b = cx.typeck_results().expr_ty(recv)
                    && a.is_diag_item(cx, sym::Result)
                    && let ty::Adt(_, args) = a.kind()
                    && let Some(a_type) = args.types().next()
                    && same_type_modulo_regions(a_type, b)
                {
                    span_lint_and_help(
                        cx,
                        USELESS_CONVERSION,
                        e.span,
                        format!("useless conversion to the same type: `{b}`"),
                        None,
                        "consider removing `.try_into()`",
                    );
                }
            },

            ExprKind::Call(path, [arg]) => {
                if let ExprKind::Path(ref qpath) = path.kind
                    && !is_ty_alias(qpath)
                    && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
                    && let Some(name) = cx.tcx.get_diagnostic_name(def_id)
                {
                    let a = cx.typeck_results().expr_ty(e);
                    let b = cx.typeck_results().expr_ty(arg);
                    if name == sym::try_from_fn
                        && a.is_diag_item(cx, sym::Result)
                        && let ty::Adt(_, args) = a.kind()
                        && let Some(a_type) = args.types().next()
                        && same_type_modulo_regions(a_type, b)
                    {
                        let hint = format!("consider removing `{}()`", snippet(cx, path.span, "TryFrom::try_from"));
                        span_lint_and_help(
                            cx,
                            USELESS_CONVERSION,
                            e.span,
                            format!("useless conversion to the same type: `{b}`"),
                            None,
                            hint,
                        );
                    } else if name == sym::from_fn && same_type_modulo_regions(a, b) {
                        let mut app = Applicability::MachineApplicable;
                        let sugg = Sugg::hir_with_context(cx, arg, e.span.ctxt(), "<expr>", &mut app).maybe_paren();
                        let sugg_msg = format!("consider removing `{}()`", snippet(cx, path.span, "From::from"));
                        span_lint_and_sugg(
                            cx,
                            USELESS_CONVERSION,
                            e.span,
                            format!("useless conversion to the same type: `{b}`"),
                            sugg_msg,
                            sugg.to_string(),
                            app,
                        );
                    }
                }
            },

            _ => {},
        }
    }

    fn check_expr_post(&mut self, _: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if Some(&e.hir_id) == self.try_desugar_arm.last() {
            self.try_desugar_arm.pop();
        }
        if e.span.from_expansion() && e.span.desugaring_kind().is_none() {
            self.expn_depth -= 1;
        }
    }
}

/// Extracts the error type `E` from `Result<T, E>`.
fn err_ty_of_result<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    if let ty::Adt(def, args) = ty.kind()
        && cx.tcx.is_diagnostic_item(sym::Result, def.did())
    {
        Some(args.type_at(1))
    } else {
        None
    }
}

/// Gets the error type that `?` converts to. This is the error type of
/// the enclosing function's return type (since `?` uses `From` to convert
/// errors to the function's return error type).
fn try_err_target_ty<'tcx>(cx: &LateContext<'tcx>) -> Option<Ty<'tcx>> {
    let body_owner = cx.tcx.hir_enclosing_body_owner(cx.last_node_with_lint_attrs);
    let ret_ty = cx.tcx.fn_sig(body_owner).instantiate_identity().output().skip_binder();
    err_ty_of_result(cx, ret_ty)
}

fn has_eligible_receiver(cx: &LateContext<'_>, recv: &Expr<'_>, expr: &Expr<'_>) -> bool {
    if cx.ty_based_def(expr).opt_parent(cx).is_impl(cx) {
        matches!(
            cx.typeck_results().expr_ty(recv).opt_diag_name(cx),
            Some(sym::Option | sym::Result | sym::ControlFlow)
        )
    } else {
        cx.ty_based_def(expr).opt_parent(cx).is_diag_item(cx, sym::Iterator)
    }
}

fn adjustments(cx: &LateContext<'_>, expr: &Expr<'_>) -> String {
    let mut prefix = String::new();

    let adjustments = cx.typeck_results().expr_adjustments(expr);

    let [.., last] = adjustments else { return prefix };
    let target = last.target;

    for adj in adjustments {
        match adj.kind {
            Adjust::Deref(_) => prefix = format!("*{prefix}"),
            Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Mut { .. })) => prefix = format!("&mut {prefix}"),
            Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) => prefix = format!("&{prefix}"),
            _ => {},
        }

        // Stop once we reach the final target type.
        // This prevents over-adjusting (e.g. suggesting &**y instead of *y).
        if adj.target == target {
            break;
        }
    }
    prefix
}
