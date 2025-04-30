use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{IntoSpan, SpanRangeExt};
use clippy_utils::trait_ref_of_method;
use core::mem;
use rustc_ast::visit::walk_list;
use rustc_errors::{Applicability, SuggestionStyle};
use rustc_hir::intravisit::{Visitor, VisitorExt, walk_poly_trait_ref, walk_ty};
use rustc_hir::{
    AmbigArg, BareFnTy, FnRetTy, FnSig, GenericParam, GenericParamKind, Generics, Impl, ImplItem, ImplItemKind,
    ImplItemRef, Item, ItemId, ItemKind, Lifetime, LifetimeKind, LifetimeParamKind, OwnerId, PatKind, PolyTraitRef,
    TraitFn, TraitItem, TraitItemKind, Ty, TyKind, lang_items,
};
use rustc_lint::{LateContext, LateLintPass, Lint};
use rustc_middle::hir::nested_filter;
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LocalDefId;
use rustc_span::symbol::kw;
use rustc_span::{Span, SyntaxContext};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for lifetime annotations which can be removed by
    /// relying on lifetime elision.
    ///
    /// ### Why is this bad?
    /// The additional lifetimes make the code look more
    /// complicated, while there is nothing out of the ordinary going on. Removing
    /// them leads to more readable code.
    ///
    /// ### Known problems
    /// This lint ignores functions with `where` clauses that reference
    /// lifetimes to prevent false positives.
    ///
    /// ### Example
    /// ```no_run
    /// // Unnecessary lifetime annotations
    /// fn in_and_out<'a>(x: &'a u8, y: u8) -> &'a u8 {
    ///     x
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn elided(x: &u8, y: u8) -> &u8 {
    ///     x
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub NEEDLESS_LIFETIMES,
    complexity,
    "using explicit lifetimes for references in function arguments when elision rules \
     would allow omitting them"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for lifetime annotations which can be replaced with anonymous lifetimes (`'_`).
    ///
    /// ### Why is this bad?
    /// The additional lifetimes can make the code look more complicated.
    ///
    /// ### Known problems
    /// This lint ignores functions with `where` clauses that reference
    /// lifetimes to prevent false positives.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::str::Chars;
    /// fn f<'a>(x: &'a str) -> Chars<'a> {
    ///     x.chars()
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # use std::str::Chars;
    /// fn f(x: &str) -> Chars<'_> {
    ///     x.chars()
    /// }
    /// ```
    #[clippy::version = "1.84.0"]
    pub ELIDABLE_LIFETIME_NAMES,
    pedantic,
    "lifetime name that can be replaced with the anonymous lifetime"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for lifetimes in generics that are never used
    /// anywhere else.
    ///
    /// ### Why is this bad?
    /// The additional lifetimes make the code look more
    /// complicated, while there is nothing out of the ordinary going on. Removing
    /// them leads to more readable code.
    ///
    /// ### Example
    /// ```no_run
    /// // unnecessary lifetimes
    /// fn unused_lifetime<'a>(x: u8) {
    ///     // ..
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn no_lifetime(x: u8) {
    ///     // ...
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub EXTRA_UNUSED_LIFETIMES,
    complexity,
    "unused lifetimes in function definitions"
}

pub struct Lifetimes {
    msrv: Msrv,
}

impl Lifetimes {
    pub fn new(conf: &'static Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl_lint_pass!(Lifetimes => [
    NEEDLESS_LIFETIMES,
    ELIDABLE_LIFETIME_NAMES,
    EXTRA_UNUSED_LIFETIMES,
]);

impl<'tcx> LateLintPass<'tcx> for Lifetimes {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        match item.kind {
            ItemKind::Fn {
                ref sig,
                generics,
                body,
                ..
            } if !generics.params.is_empty() => {
                self.check_fn_inner(cx, sig, TraitFn::Provided(body), generics, item.span, item.owner_id);
            },
            ItemKind::Impl(impl_) if !impl_.generics.params.is_empty() && !item.span.from_expansion() => {
                self.check_impl(cx, item.span, impl_);
            },
            _ => {},
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'_>) {
        if let ImplItemKind::Fn(ref sig, id) = item.kind
            && !item.generics.params.is_empty()
        {
            self.check_fn_inner(cx, sig, TraitFn::Provided(id), item.generics, item.span, item.owner_id);
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'_>) {
        if let TraitItemKind::Fn(ref sig, body) = item.kind
            && !item.generics.params.is_empty()
        {
            self.check_fn_inner(cx, sig, body, item.generics, item.span, item.owner_id);
        }
    }
}

impl Lifetimes {
    fn check_fn_inner<'tcx>(
        &self,
        cx: &LateContext<'tcx>,
        sig: &'tcx FnSig<'tcx>,
        body: TraitFn<'tcx>,
        generics: &'tcx Generics<'tcx>,
        span: Span,
        owner_id: OwnerId,
    ) {
        let Some(visitor) = self.init_from_generics(cx, span, generics) else {
            return;
        };

        // Walk the input/output types.
        let mut visitor = OutputLifetimeVisitor { cx: visitor };
        if let FnRetTy::Return(ty) = sig.decl.output {
            visitor.visit_ty_unambig(ty);
            if visitor.cx.lifetimes.is_empty() {
                return;
            }
        }
        let mut visitor = InputLifetimeVisitor { cx: visitor.cx };
        for ty in sig.decl.inputs {
            visitor.visit_ty_unambig(ty);
            if visitor.cx.lifetimes.is_empty() {
                return;
            }
        }

        // Check if the first parameter is named `self`.
        let (is_self, body) = match body {
            TraitFn::Provided(body) => {
                let body = cx.tcx.hir_body(body);
                (
                    body.params.first().is_some_and(|param| {
                        if let PatKind::Binding(_, _, ident, None) = param.pat.kind {
                            ident.name == kw::SelfLower
                        } else {
                            false
                        }
                    }),
                    Some(body),
                )
            },
            TraitFn::Required(params) => (
                params
                    .first()
                    .is_some_and(|&x| x.is_some_and(|ident| ident.name == kw::SelfLower)),
                None,
            ),
        };

        // Check if an output lifetime can be elided.
        if let Some(candidate) = visitor.cx.output_candidate {
            let can_elide = if is_self
                && let Some(ty) = sig.decl.inputs.first()
                && let TyKind::Ref(self_lt, _) = ty.kind
            {
                // Inference from `self` references takes priority over other lifetimes.
                if let LifetimeKind::Param(id) = self_lt.kind
                    && (sig.decl.implicit_self.has_implicit_self()
                        || self.msrv.meets(cx, msrvs::EXPLICIT_SELF_TYPE_ELISION))
                {
                    visitor.cx.lifetimes[candidate].id == id
                } else {
                    false
                }
            } else {
                // `can_infer_output` doesn't track if the lifetime actually appeared in the input,
                // but every other case is handled (multiple output/input lifetimes and the lifetime
                // used multiple times in the input).
                visitor.cx.can_infer_output && visitor.cx.lifetimes[candidate].used_in_input
            };
            if !can_elide {
                visitor.cx.lifetimes.swap_remove(candidate);
                if visitor.cx.lifetimes.is_empty() {
                    return;
                }
            }
        }

        // Remove any lifetimes used explicitly within the body (e.g. in a type name or generic argument).
        if let Some(body) = body {
            LifetimeVisitor { cx: &mut visitor.cx }.visit_body(body);
            if visitor.cx.lifetimes.is_empty() {
                return;
            }
        }

        report_lifetimes(
            cx,
            visitor.cx.ctxt,
            generics,
            trait_ref_of_method(cx, owner_id).is_none(),
            visitor.cx.lifetimes,
            visitor.cx.uses,
        );
    }

    fn check_impl<'tcx>(&self, cx: &LateContext<'tcx>, span: Span, impl_: &'tcx Impl<'tcx>) {
        let Some(visitor) = self.init_from_generics(cx, span, impl_.generics) else {
            return;
        };

        let mut visitor = InputLifetimeVisitor { cx: visitor };
        visitor.visit_ty_unambig(impl_.self_ty);
        if visitor.cx.lifetimes.is_empty() {
            return;
        }
        if let Some(trait_) = &impl_.of_trait {
            visitor.visit_trait_ref(trait_);
        }
        if visitor.cx.lifetimes.is_empty() {
            return;
        }

        walk_list!(
            &mut LifetimeVisitor { cx: &mut visitor.cx },
            visit_impl_item_ref,
            impl_.items
        );
        if visitor.cx.lifetimes.is_empty() {
            return;
        }

        report_lifetimes(
            cx,
            visitor.cx.ctxt,
            impl_.generics,
            true,
            visitor.cx.lifetimes,
            visitor.cx.uses,
        );
    }

    /// Creates the visitor context from the generic parameters. Returns `None` if there are no
    /// candidate lifetimes.
    ///
    /// This start with all named generics and filters out any that are required by the any of the
    /// generic predicates. Any lifetime that is from a different syntax context or from an external
    /// macro will also be filtered out.
    fn init_from_generics<'lcx, 'tcx>(
        &self,
        lcx: &'lcx LateContext<'tcx>,
        span: Span,
        generics: &'tcx Generics<'tcx>,
    ) -> Option<VisitorCx<'lcx, 'tcx>> {
        let ctxt = span.ctxt();

        // First pass over the generics without allocating. This minimizes the work when there's
        // nothing to lint.
        let params = generics.params.iter().map_while(|param: &GenericParam<'_>| {
            if let GenericParamKind::Lifetime { kind } = param.kind {
                Some((param, kind))
            } else {
                None
            }
        });
        let count = params
            .clone()
            .filter(|(param, kind)| {
                matches!(kind, LifetimeParamKind::Explicit) && param.colon_span.is_none() && param.span.ctxt() == ctxt
            })
            .count();
        if count == 0 || span.in_external_macro(lcx.tcx.sess.source_map()) {
            return None;
        }

        let mut cx = VisitorCx {
            lcx,
            ctxt,
            msrv: self.msrv,
            lifetimes: Vec::with_capacity(count),
            uses: Vec::new(),
            output_candidate: None,
            use_site: LifetimeUseSite::List,
            can_infer_output: true,
        };
        cx.lifetimes.extend(params.filter_map(|(param, kind)| {
            (matches!(kind, LifetimeParamKind::Explicit) && param.colon_span.is_none() && param.span.ctxt() == ctxt)
                .then_some(LifetimeCandidate {
                    id: param.def_id,
                    used_in_input: false,
                    span: param.span,
                })
        }));

        for pred in generics.predicates {
            LifetimeVisitor { cx: &mut cx }.visit_where_predicate(pred);
        }
        (!cx.lifetimes.is_empty()).then_some(cx)
    }
}

fn report_lifetimes(
    cx: &LateContext<'_>,
    ctxt: SyntaxContext,
    generics: &Generics<'_>,
    report_unused: bool,
    mut lifetimes: Vec<LifetimeCandidate>,
    mut uses: Vec<LifetimeUse>,
) {
    // A different lint is used if explicit elision is suggested.
    let mut has_explicit_elision = false;

    // Preallocate the maximum possible space needed.
    let mut elidible_report = Vec::with_capacity(lifetimes.len());
    let mut elidible_sugg = Vec::with_capacity(uses.len() + lifetimes.len());
    let mut unused_report = Vec::with_capacity(lifetimes.len());

    // Sort both so they can be walked together in one pass.
    uses.sort_by_key(|x| x.id.local_def_index);
    lifetimes.sort_by_key(|x| x.id.local_def_index);
    let mut uses = uses.iter();
    for lt in &lifetimes {
        // Walk until we find the uses for the current def and get a slice of all of them.
        while uses.clone().next().is_some_and(|x| x.id != lt.id) {
            uses.next();
        }
        let lt_uses = uses.as_slice();
        while uses.clone().next().is_some_and(|x| x.id == lt.id) {
            uses.next();
        }
        let lt_uses = &lt_uses[..lt_uses.len() - uses.len()];

        // The suggestion to remove the def site for each parameter is added in
        // `report_lifetimes_inner`. All the def sites are needed side-by-side to build the
        // suggestion.
        if lt_uses.is_empty() {
            unused_report.push(lt.span);
        } else {
            elidible_report.push(lt.span);
            elidible_sugg.extend(lt_uses.iter().map(|x| {
                has_explicit_elision |= matches!(x.use_site, LifetimeUseSite::List);
                match x.use_site {
                    LifetimeUseSite::List => (x.span, String::from("'_")),
                    LifetimeUseSite::Ref => (x.span.with_trailing_whitespace(cx).with_ctxt(ctxt), String::new()),
                }
            }));
        }
    }

    if !elidible_report.is_empty() {
        report_lifetimes_inner(
            cx,
            ctxt,
            generics,
            if has_explicit_elision {
                ELIDABLE_LIFETIME_NAMES
            } else {
                NEEDLESS_LIFETIMES
            },
            "these lifetime parameters can be elided",
            elidible_report,
            elidible_sugg,
        );
    }
    if report_unused && !unused_report.is_empty() {
        report_lifetimes_inner(
            cx,
            ctxt,
            generics,
            EXTRA_UNUSED_LIFETIMES,
            "these lifetime parameters are unused",
            unused_report,
            Vec::new(),
        );
    }
}

fn report_lifetimes_inner(
    cx: &LateContext<'_>,
    ctxt: SyntaxContext,
    generics: &'_ Generics<'_>,
    lint: &'static Lint,
    msg: &'static str,
    report: Vec<Span>,
    mut sugg: Vec<(Span, String)>,
) {
    if generics
        .params
        .iter()
        .filter(|&p| generics.span.contains(p.span))
        .count()
        == report.len()
    {
        // All generic parameters explicitly listed are removed. Get rid of the empty
        // brackets (`<>`) as well.
        sugg.push((generics.span, String::new()));
    } else {
        let old_len = sugg.len();
        sugg.extend(
            report
                .iter()
                .map(|sp| (sp.expand_list_item(cx).with_ctxt(ctxt), String::new())),
        );

        // If two parameters are side-by-side (e.g. `'a, 'b`) and both are removed, we need to
        // create a single suggestion to remove both.
        let new_sugg = &mut sugg[old_len..];
        new_sugg.sort_by_key(|x| x.0);
        let (new_sugg, _) = new_sugg.partition_dedup_by(|next, prev| {
            // n.b. `next` comes in the list after `prev` and is the one partitioned out.
            // `prev` is always retained.
            if prev.0.hi() >= next.0.lo() {
                prev.0 = prev.0.with_hi(next.0.hi());
                true
            } else {
                false
            }
        });
        let new_len = new_sugg.len() + old_len;
        sugg.truncate(new_len);
    }

    span_lint_and_then(cx, lint, report, msg, |diag| {
        diag.multipart_suggestion_with_style(
            "remove the lifetime parameters",
            sugg,
            if ctxt.is_root() {
                Applicability::MachineApplicable
            } else {
                Applicability::MaybeIncorrect
            },
            SuggestionStyle::ShowAlways,
        );
    });
}

struct LifetimeCandidate {
    id: LocalDefId,
    used_in_input: bool,
    span: Span,
}

#[derive(Clone, Copy)]
enum LifetimeUseSite {
    List,
    Ref,
    // Will need to be extended to handle use in generic bounds.
}
impl LifetimeUseSite {
    fn is_ref(self) -> bool {
        matches!(self, Self::Ref)
    }
}

struct LifetimeUse {
    id: LocalDefId,
    span: Span,
    use_site: LifetimeUseSite,
}

/// Shared context used by all visitors.
struct VisitorCx<'lcx, 'tcx> {
    lcx: &'lcx LateContext<'tcx>,
    ctxt: SyntaxContext,
    msrv: Msrv,
    // Don't use a hashmap here since even ten would be an extremely large number explicit lifetimes
    // to have. This list also shrinks over the course of the lint running as we determine lifetimes
    // can't be elided.
    lifetimes: Vec<LifetimeCandidate>,
    uses: Vec<LifetimeUse>,
    /// The first output lifetime seen while `can_infer_output` is `true`.
    ///
    /// If a second output lifetime is seen this will be set back to `None` and `can_infer_output`
    /// will be set to `false`.
    output_candidate: Option<usize>,
    use_site: LifetimeUseSite,
    /// Can regular function inference be applied to the current signature.
    ///
    /// n.b. This may be `true` if there are no input lifetimes even though an output candidate
    /// exists. All other cases blocking lifetime inference will set this to `false`.
    can_infer_output: bool,
}
impl VisitorCx<'_, '_> {
    /// Removes the lifetime at the given index, adjusting the current output candidate if needed.
    fn remove_lifetime_idx(&mut self, idx: usize) {
        self.lifetimes.swap_remove(idx);
        if let Some(candidate) = self.output_candidate {
            if candidate == idx {
                self.output_candidate = None;
                self.can_infer_output = false;
            } else if candidate == self.lifetimes.len() {
                self.output_candidate = Some(idx);
            }
        }
    }
}

/// Visitor which removes all found lifetimes.
struct LifetimeVisitor<'a, 'lcx, 'tcx> {
    cx: &'a mut VisitorCx<'lcx, 'tcx>,
}
impl<'tcx> Visitor<'tcx> for LifetimeVisitor<'_, '_, 'tcx> {
    // We only want to descend into `ImplItem`s, not all `Item`s.
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.lcx.tcx
    }

    fn visit_lifetime(&mut self, lifetime: &'tcx Lifetime) {
        if let LifetimeKind::Param(id) = lifetime.kind
            && let Some(idx) = self.cx.lifetimes.iter().position(|x| x.id == id)
        {
            self.cx.remove_lifetime_idx(idx);
        }
    }

    // Only used by `check_impl`
    fn visit_impl_item_ref(&mut self, item: &'tcx ImplItemRef) -> Self::Result {
        let item = self.cx.lcx.tcx.hir_impl_item(item.id);
        self.visit_generics(item.generics);
        match item.kind {
            ImplItemKind::Const(ty, body) => {
                self.visit_ty_unambig(ty);
                self.visit_body(self.cx.lcx.tcx.hir_body(body));
            },
            ImplItemKind::Fn(ref sig, body) => {
                self.visit_fn_decl(sig.decl);
                self.visit_body(self.cx.lcx.tcx.hir_body(body));
            },
            ImplItemKind::Type(ty) => self.visit_ty_unambig(ty),
        }
    }

    // Avoid monomorphising all `visit_*` functions.
    fn visit_nested_item(&mut self, _: ItemId) {}
}

struct OutputLifetimeVisitor<'lcx, 'tcx> {
    cx: VisitorCx<'lcx, 'tcx>,
}
impl<'tcx> Visitor<'tcx> for OutputLifetimeVisitor<'_, 'tcx> {
    fn visit_lifetime(&mut self, lifetime: &'tcx Lifetime) {
        if let LifetimeKind::Param(id) = lifetime.kind {
            match (
                self.cx.output_candidate,
                self.cx.lifetimes.iter().position(|x| x.id == id),
            ) {
                // Lifetime found in a different context; no named output lifetime can be inferred.
                (_, Some(idx)) if self.cx.ctxt != lifetime.ident.span.ctxt() => {
                    self.cx.remove_lifetime_idx(idx);
                    if let Some(idx) = self.cx.output_candidate.take() {
                        self.cx.lifetimes.swap_remove(idx);
                    }
                    self.cx.can_infer_output = false;
                },
                // Same lifetime as the current inference candidate.
                (Some(x), Some(y)) if x == y => {
                    // Lifetimes in output references might not be written.
                    // e.g. `fn f<'a>(_: &'a u32) -> &u32`.
                    // The output will be `'a`, but the lifetime wasn't written out.
                    if !(self.cx.use_site.is_ref() && lifetime.ident.span.is_empty()) {
                        self.cx.uses.push(LifetimeUse {
                            id,
                            span: lifetime.ident.span,
                            use_site: self.cx.use_site,
                        });
                    }
                },
                // Multiple different lifetime names found. No output lifetime can be inferred.
                (Some(x), Some(y)) => {
                    self.cx.lifetimes.swap_remove(x);
                    // The item at `y` might have been moved to `x` due to the previous
                    // `swap_remove`.
                    self.cx
                        .lifetimes
                        .swap_remove(if y == self.cx.lifetimes.len() { x } else { y });
                    self.cx.output_candidate = None;
                    self.cx.can_infer_output = false;
                },
                // First lifetime seen. This is our inference candidate.
                (None, Some(x)) if self.cx.can_infer_output => {
                    self.cx.output_candidate = Some(x);
                    // Lifetimes in output references might not be written.
                    // e.g. `fn f<'a>(_: &'a u32) -> &u32`.
                    // The output will be `'a`, but the lifetime wasn't written out.
                    if !(self.cx.use_site.is_ref() && lifetime.ident.span.is_empty()) {
                        self.cx.uses.push(LifetimeUse {
                            id,
                            span: lifetime.ident.span,
                            use_site: self.cx.use_site,
                        });
                    }
                },
                // Output inference has already been ruled out.
                (None, Some(x)) => {
                    self.cx.can_infer_output = false;
                    self.cx.lifetimes.swap_remove(x);
                },
                // The current lifetime isn't the same as the current inference candidate.
                (Some(x), None) => {
                    self.cx.output_candidate = None;
                    self.cx.can_infer_output = false;
                    self.cx.lifetimes.swap_remove(x);
                },
                // The current lifetime can't be an inference candidate.
                (None, None) => {
                    self.cx.can_infer_output = false;
                },
            }
        }
    }

    fn visit_poly_trait_ref(&mut self, poly_tref: &'tcx PolyTraitRef<'tcx>) {
        let trait_ref = &poly_tref.trait_ref;
        if let Some(id) = trait_ref.trait_def_id()
            && lang_items::FN_TRAITS
                .iter()
                .any(|&item| self.cx.lcx.tcx.lang_items().get(item) == Some(id))
        {
            // Lifetimes in `impl Fn(..)` syntax can only infer higher ranked bounds.
            // e.g. `for<'a> impl Fn(&'a T, &'b U)` can't have `'b` inferred.
            LifetimeVisitor { cx: &mut self.cx }.visit_trait_ref(&poly_tref.trait_ref);
        } else {
            walk_poly_trait_ref(self, poly_tref);
        }
    }

    fn visit_ty(&mut self, ty: &'tcx Ty<'_, AmbigArg>) {
        match ty.kind {
            // Lifetimes in `fn(..)` syntax can only infer higher ranked bounds.
            // e.g. `for<'a> fn(&'a T, &'b U)` can't have `'b` inferred.
            TyKind::BareFn(&BareFnTy { decl, .. }) => LifetimeVisitor { cx: &mut self.cx }.visit_fn_decl(decl),
            TyKind::TraitObject(bounds, ref lt) => {
                let lt = &**lt;
                if !matches!(lt.kind, LifetimeKind::ImplicitObjectLifetimeDefault) {
                    if self.cx.msrv.meets(self.cx.lcx, msrvs::DYN_TRAIT_LIFETIME_INFERENCE) {
                        self.visit_lifetime(lt);
                    } else {
                        if let LifetimeKind::Param(id) = lt.kind
                            && let Some(idx) = self.cx.lifetimes.iter().position(|x| x.id == id)
                        {
                            self.cx.remove_lifetime_idx(idx);
                        }
                        if let Some(idx) = self.cx.output_candidate.take() {
                            self.cx.lifetimes.swap_remove(idx);
                        }
                        self.cx.can_infer_output = false
                    }
                }
                walk_list!(self, visit_poly_trait_ref, bounds);
            },
            TyKind::Ref(lt, ty) => {
                self.cx.use_site = LifetimeUseSite::Ref;
                self.visit_lifetime(lt);
                self.cx.use_site = LifetimeUseSite::List;
                self.visit_ty_unambig(ty.ty);
            },
            _ => walk_ty(self, ty),
        }
    }

    // Avoid monomorphising all `visit_*` functions.
    fn visit_nested_item(&mut self, _: ItemId) {}
}

struct InputLifetimeVisitor<'lcx, 'tcx> {
    cx: VisitorCx<'lcx, 'tcx>,
}
impl<'tcx> Visitor<'tcx> for InputLifetimeVisitor<'_, 'tcx> {
    fn visit_lifetime(&mut self, lifetime: &'tcx Lifetime) {
        if let LifetimeKind::Param(id) = lifetime.kind {
            if let Some(idx) = self.cx.lifetimes.iter().position(|x| x.id == id) {
                if mem::replace(&mut self.cx.lifetimes[idx].used_in_input, true)
                    || self.cx.ctxt != lifetime.ident.span.ctxt()
                {
                    // An input lifetime is used at least twice or a macro is involved. In either
                    // case this lifetime cannot be removed.
                    self.cx.remove_lifetime_idx(idx);
                    self.cx.can_infer_output = false;
                } else {
                    self.cx.uses.push(LifetimeUse {
                        id,
                        span: lifetime.ident.span,
                        use_site: self.cx.use_site,
                    });
                    if self.cx.output_candidate != Some(idx) {
                        // Multiple elidible input lifetimes can't infer an output lifetime.
                        self.cx.can_infer_output = false;
                    }
                }
            } else {
                // Input parameter can't be elided, therefore the output can't be either.
                // If this is the only input parameter it could technically be elided, but
                // `elided_named_lifetimes` warns about that and is enabled by default.
                // e.g. `fn foo<'a, T: Foo<'a>>(_: &'a T) -> &'a str` can elide the output lifetime.
                self.cx.can_infer_output = false;
            }
        } else {
            // An inferred input lifetime can't be used to infer an explicit output lifetime.
            // Either the output is bound to another input, or it is unbound.
            self.cx.can_infer_output = false;
        }
    }

    fn visit_poly_trait_ref(&mut self, poly_tref: &'tcx PolyTraitRef<'tcx>) {
        if let Some(id) = poly_tref.trait_ref.trait_def_id()
            && lang_items::FN_TRAITS
                .iter()
                .any(|&item| self.cx.lcx.tcx.lang_items().get(item) == Some(id))
        {
            // Lifetimes in `impl Fn(..)` syntax can only infer higher ranked bounds.
            // e.g. `for<'a> impl Fn(&'a T, &'b U)` can't have `'b` inferred.
            LifetimeVisitor { cx: &mut self.cx }.visit_trait_ref(&poly_tref.trait_ref);
        } else {
            walk_poly_trait_ref(self, poly_tref);
        }
    }

    fn visit_ty(&mut self, ty: &'tcx Ty<'_, AmbigArg>) {
        match ty.kind {
            // Lifetimes in `fn(..)` syntax can only infer higher ranked bounds.
            // e.g. `for<'a> fn(&'a T, &'b U)` can't have `'b` inferred.
            TyKind::BareFn(&BareFnTy { decl, .. }) => LifetimeVisitor { cx: &mut self.cx }.visit_fn_decl(decl),
            TyKind::TraitObject(bounds, ref lt) => {
                let lt = &**lt;
                if !matches!(lt.kind, LifetimeKind::ImplicitObjectLifetimeDefault) {
                    if !self.cx.msrv.meets(self.cx.lcx, msrvs::DYN_TRAIT_LIFETIME_INFERENCE) {
                        self.cx.can_infer_output = false;
                    }
                    self.visit_lifetime(lt);
                }
                walk_list!(self, visit_poly_trait_ref, bounds);
            },
            TyKind::Ref(lt, ty) => {
                self.cx.use_site = LifetimeUseSite::Ref;
                self.visit_lifetime(lt);
                self.cx.use_site = LifetimeUseSite::List;
                self.visit_ty_unambig(ty.ty);
            },
            _ => walk_ty(self, ty),
        }
    }

    // Avoid monomorphising all `visit_*` functions.
    fn visit_nested_item(&mut self, _: ItemId) {}
}
