use clippy_config::msrvs::{self, Msrv};
use clippy_utils::diagnostics::span_lint;
use clippy_utils::qualify_min_const_fn::is_min_const_fn;
use clippy_utils::ty::has_drop;
use clippy_utils::{fn_has_unsatisfiable_preds, is_entrypoint_fn, is_from_proc_macro, trait_ref_of_method};
use rustc_hir as hir;
use rustc_hir::def_id::CRATE_DEF_ID;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, Constness, FnDecl, GenericParamKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::lint::in_external_macro;
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LocalDefId;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Suggests the use of `const` in functions and methods where possible.
    ///
    /// ### Why is this bad?
    /// Not having the function const prevents callers of the function from being const as well.
    ///
    /// ### Known problems
    /// Const functions are currently still being worked on, with some features only being available
    /// on nightly. This lint does not consider all edge cases currently and the suggestions may be
    /// incorrect if you are using this lint on stable.
    ///
    /// Also, the lint only runs one pass over the code. Consider these two non-const functions:
    ///
    /// ```no_run
    /// fn a() -> i32 {
    ///     0
    /// }
    /// fn b() -> i32 {
    ///     a()
    /// }
    /// ```
    ///
    /// When running Clippy, the lint will only suggest to make `a` const, because `b` at this time
    /// can't be const as it calls a non-const function. Making `a` const and running Clippy again,
    /// will suggest to make `b` const, too.
    ///
    /// If you are marking a public function with `const`, removing it again will break API compatibility.
    /// ### Example
    /// ```no_run
    /// # struct Foo {
    /// #     random_number: usize,
    /// # }
    /// # impl Foo {
    /// fn new() -> Self {
    ///     Self { random_number: 42 }
    /// }
    /// # }
    /// ```
    ///
    /// Could be a const fn:
    ///
    /// ```no_run
    /// # struct Foo {
    /// #     random_number: usize,
    /// # }
    /// # impl Foo {
    /// const fn new() -> Self {
    ///     Self { random_number: 42 }
    /// }
    /// # }
    /// ```
    #[clippy::version = "1.34.0"]
    pub MISSING_CONST_FOR_FN,
    nursery,
    "Lint functions definitions that could be made `const fn`"
}

impl_lint_pass!(MissingConstForFn => [MISSING_CONST_FOR_FN]);

pub struct MissingConstForFn {
    msrv: Msrv,
}

impl MissingConstForFn {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self { msrv }
    }
}

impl<'tcx> LateLintPass<'tcx> for MissingConstForFn {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &FnDecl<'_>,
        body: &Body<'tcx>,
        span: Span,
        def_id: LocalDefId,
    ) {
        if !self.msrv.meets(msrvs::CONST_IF_MATCH) {
            return;
        }

        if in_external_macro(cx.tcx.sess, span) || is_entrypoint_fn(cx, def_id.to_def_id()) {
            return;
        }

        // Building MIR for `fn`s with unsatisfiable preds results in ICE.
        if fn_has_unsatisfiable_preds(cx, def_id.to_def_id()) {
            return;
        }

        // Perform some preliminary checks that rule out constness on the Clippy side. This way we
        // can skip the actual const check and return early.
        match kind {
            FnKind::ItemFn(_, generics, header, ..) => {
                let has_const_generic_params = generics
                    .params
                    .iter()
                    .any(|param| matches!(param.kind, GenericParamKind::Const { .. }));

                if already_const(header) || has_const_generic_params {
                    return;
                }
            },
            FnKind::Method(_, sig, ..) => {
                if trait_ref_of_method(cx, def_id).is_some()
                    || already_const(sig.header)
                    || method_accepts_droppable(cx, def_id)
                {
                    return;
                }
            },
            FnKind::Closure => return,
        }

        let hir_id = cx.tcx.local_def_id_to_hir_id(def_id);

        // Const fns are not allowed as methods in a trait.
        {
            let parent = cx.tcx.hir().get_parent_item(hir_id).def_id;
            if parent != CRATE_DEF_ID {
                if let hir::Node::Item(item) = cx.tcx.hir_node_by_def_id(parent) {
                    if let hir::ItemKind::Trait(..) = &item.kind {
                        return;
                    }
                }
            }
        }

        if is_from_proc_macro(cx, &(&kind, body, hir_id, span)) {
            return;
        }

        let mir = cx.tcx.optimized_mir(def_id);

        if let Err((span, err)) = is_min_const_fn(cx.tcx, mir, &self.msrv) {
            if cx.tcx.is_const_fn_raw(def_id.to_def_id()) {
                cx.tcx.dcx().span_err(span, err);
            }
        } else {
            span_lint(cx, MISSING_CONST_FOR_FN, span, "this could be a `const fn`");
        }
    }
    extract_msrv_attr!(LateContext);
}

/// Returns true if any of the method parameters is a type that implements `Drop`. The method
/// can't be made const then, because `drop` can't be const-evaluated.
fn method_accepts_droppable(cx: &LateContext<'_>, def_id: LocalDefId) -> bool {
    let sig = cx.tcx.fn_sig(def_id).instantiate_identity().skip_binder();

    // If any of the params are droppable, return true
    sig.inputs().iter().any(|&ty| has_drop(cx, ty))
}

// We don't have to lint on something that's already `const`
#[must_use]
fn already_const(header: hir::FnHeader) -> bool {
    header.constness == Constness::Const
}
