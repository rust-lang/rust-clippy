use LintKind::{AllocInsteadOfCore, StdInsteadOfAlloc, StdInsteadOfCore};
use clippy_config::Conf;
use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg, span_lint_and_then};
use clippy_utils::is_from_proc_macro;
use clippy_utils::msrvs::Msrv;
use rustc_errors::{Applicability, MultiSpan};
use rustc_hir::def_id::DefId;
use rustc_hir::{Block, Body, HirId, Item, ItemKind, Path, PathSegment, StabilityLevel, StableSince};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_session::impl_lint_pass;
use rustc_span::symbol::kw;
use rustc_span::{Ident, Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Finds items imported through `alloc` when available through `core`.
    ///
    /// ### Why restrict this?
    /// Crates which have `no_std` compatibility and may optionally require alloc may wish to ensure types are
    /// imported from core to ensure disabling `alloc` does not cause the crate to fail to compile. This lint
    /// is also useful for crates migrating to become `no_std` compatible.
    ///
    /// ### Known problems
    /// The lint is only partially aware of the required MSRV for items that were originally in `std` but moved
    /// to `core`.
    ///
    /// ### Example
    /// ```no_run
    /// # extern crate alloc;
    /// use alloc::slice::from_ref;
    /// ```
    /// Use instead:
    /// ```no_run
    /// use core::slice::from_ref;
    /// ```
    #[clippy::version = "1.64.0"]
    pub ALLOC_INSTEAD_OF_CORE,
    restriction,
    "type is imported from alloc when available in core"
}

declare_clippy_lint! {
    /// ### What it does
    /// Finds items imported through `std` when available through `alloc`.
    ///
    /// ### Why restrict this?
    /// Crates which have `no_std` compatibility and require alloc may wish to ensure types are imported from
    /// alloc to ensure disabling `std` does not cause the crate to fail to compile. This lint is also useful
    /// for crates migrating to become `no_std` compatible.
    ///
    /// ### Example
    /// ```no_run
    /// use std::vec::Vec;
    /// ```
    /// Use instead:
    /// ```no_run
    /// # extern crate alloc;
    /// use alloc::vec::Vec;
    /// ```
    #[clippy::version = "1.64.0"]
    pub STD_INSTEAD_OF_ALLOC,
    restriction,
    "type is imported from std when available in alloc"
}

declare_clippy_lint! {
    /// ### What it does
    /// Finds items imported through `std` when available through `core`.
    ///
    /// ### Why restrict this?
    /// Crates which have `no_std` compatibility may wish to ensure types are imported from core to ensure
    /// disabling `std` does not cause the crate to fail to compile. This lint is also useful for crates
    /// migrating to become `no_std` compatible.
    ///
    /// ### Example
    /// ```no_run
    /// use std::hash::Hasher;
    /// ```
    /// Use instead:
    /// ```no_run
    /// use core::hash::Hasher;
    /// ```
    #[clippy::version = "1.64.0"]
    pub STD_INSTEAD_OF_CORE,
    restriction,
    "type is imported from std when available in core"
}

impl_lint_pass!(StdReexports => [
    ALLOC_INSTEAD_OF_CORE,
    STD_INSTEAD_OF_ALLOC,
    STD_INSTEAD_OF_CORE,
]);

pub struct StdReexports {
    lint_points: Option<(LintPoint, Vec<Span>, Vec<Span>, usize)>,
    msrv: Msrv,
    paths_to_skip: usize,
}

impl StdReexports {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            lint_points: Option::default(),
            msrv: conf.msrv.into(),
            paths_to_skip: 0,
        }
    }
}

#[derive(Debug)]
struct LintPoint {
    ident: Ident,
    is_crate: bool,
    from: UsedFrom,
}

impl LintPoint {
    fn try_new(cx: &LateContext<'_>, &PathSegment { ident, res, .. }: &PathSegment<'_>) -> Option<Self> {
        let def_id = res.opt_def_id()?;
        let is_crate = def_id.is_crate_root();
        let from = match cx.tcx.crate_name(def_id.krate) {
            sym::std => UsedFrom::Std,
            sym::alloc => UsedFrom::Alloc,
            _ => return None,
        };
        Some(LintPoint { ident, is_crate, from })
    }
}

impl<'tcx> LateLintPass<'tcx> for StdReexports {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, _: HirId) {
        let Some((a, b)) = get_end_segments(path) else { return };

        if let Some(n) = self.paths_to_skip.checked_sub(1) {
            self.paths_to_skip = n;
        } else if let Some(lint_point) = LintPoint::try_new(cx, a)
            && let Some(b_def_id) = path.res.opt_def_id()
            && let Some(defined_in) = DefinedIn::try_new(cx, b_def_id)
            && let Some(kind) = LintKind::try_new(lint_point.from, defined_in)
            && is_stable(cx, b_def_id, self.msrv)
        {
            emit_lints(cx, self.lint_points.take());
            emit_lint(cx, &lint_point, &kind, false, b.ident.span);
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let ItemKind::Use(path, ..) = item.kind else { return };
        let Some((a, b)) = get_end_segments(path) else { return };

        self.paths_to_skip += path.res.present_items().count();

        if self
            .lint_points
            .as_ref()
            .is_none_or(|(x, ..)| x.ident.span != a.ident.span)
        {
            emit_lints(cx, self.lint_points.take());
            self.lint_points = LintPoint::try_new(cx, a).map(|a| (a, Vec::new(), Vec::new(), 0));
        }

        if let Some((lint_point, in_core, in_alloc, conflicts)) = self.lint_points.as_mut() {
            let lint_kind = path
                .res
                .iter()
                .flatten()
                .try_fold(DefinedIn::Core, |acc, res| {
                    let def_id = res.opt_def_id()?;
                    match (acc, DefinedIn::try_new(cx, def_id)?, is_stable(cx, def_id, self.msrv)) {
                        (_, _, false) => None,
                        (DefinedIn::Core, DefinedIn::Core, _) => Some(DefinedIn::Core),
                        _ => Some(DefinedIn::Alloc),
                    }
                })
                .and_then(|defined_in| LintKind::try_new(lint_point.from, defined_in));

            match lint_kind {
                Some(AllocInsteadOfCore | StdInsteadOfCore) => in_core.push(b.ident.span),
                Some(StdInsteadOfAlloc) => in_alloc.push(b.ident.span),
                None => *conflicts += 1,
            }
        }
    }

    fn check_block_post(&mut self, cx: &LateContext<'tcx>, _: &Block<'tcx>) {
        emit_lints(cx, self.lint_points.take());
    }

    fn check_body_post(&mut self, cx: &LateContext<'tcx>, _: &Body<'tcx>) {
        emit_lints(cx, self.lint_points.take());
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        emit_lints(cx, self.lint_points.take());
    }
}

fn emit_lints(cx: &LateContext<'_>, lint_points: Option<(LintPoint, Vec<Span>, Vec<Span>, usize)>) {
    let Some((lint_point, in_core, in_alloc, conflicts)) = lint_points else {
        return;
    };

    let total = in_core.len() + in_alloc.len() + conflicts;
    for (spans, defined_in) in [(in_core, DefinedIn::Core), (in_alloc, DefinedIn::Alloc)] {
        let Some(lint_kind) = LintKind::try_new(lint_point.from, defined_in) else {
            continue;
        };

        if !spans.is_empty() {
            emit_lint(cx, &lint_point, &lint_kind, spans.len() < total, spans);
        }
    }
}

fn emit_lint(cx: &LateContext<'_>, point: &LintPoint, kind: &LintKind, has_conflict: bool, span: impl Into<MultiSpan>) {
    if point.ident.span.in_external_macro(cx.sess().source_map()) || is_from_proc_macro(cx, &point.ident) {
        return;
    }

    let (lint, lint_message) = match kind {
        StdInsteadOfCore => (STD_INSTEAD_OF_CORE, "used import from `std` instead of `core`"),
        StdInsteadOfAlloc => (STD_INSTEAD_OF_ALLOC, "used import from `std` instead of `alloc`"),
        AllocInsteadOfCore => (ALLOC_INSTEAD_OF_CORE, "used import from `alloc` instead of `core`"),
    };

    let (help_message, replace_with) = match kind {
        StdInsteadOfCore | AllocInsteadOfCore => ("consider importing the item from `core`", &sym::core),
        StdInsteadOfAlloc => ("consider importing the item from `alloc`", &sym::alloc),
    };

    if !has_conflict && point.is_crate {
        span_lint_and_sugg(
            cx,
            lint,
            point.ident.span,
            lint_message,
            help_message,
            (*replace_with).to_string(),
            Applicability::MachineApplicable,
        );
        return;
    }

    let leaf_spans = span.into();
    if leaf_spans.primary_spans().len() == 1 {
        span_lint_and_help(cx, lint, leaf_spans, lint_message, None, help_message);
    } else {
        span_lint_and_then(cx, lint, point.ident.span, lint_message, |diag| {
            diag.span_help(leaf_spans, help_message);
        });
    }
}

/// Returns the first and last named segments of a [`Path`].
///
/// If this is a global path (such as `::std::fmt::Debug`), then the segment after [`kw::PathRoot`]
/// is returned.
fn get_end_segments<'tcx, T>(path: &Path<'tcx, T>) -> Option<(&'tcx PathSegment<'tcx>, &'tcx PathSegment<'tcx>)> {
    match path.segments {
        // A global path will have PathRoot as the first segment. In this case, return the segment after.
        [x, y, .., z] if x.ident.name == kw::PathRoot => Some((y, z)),
        [x, .., y] => Some((x, y)),
        _ => None,
    }
}

/// Checks if all ancestors of `def_id` meet `msrv` to avoid linting [unstable moves](https://github.com/rust-lang/rust/pull/95956)
/// or now stable moves that were once unstable.
///
/// Does not catch individually moved items
fn is_stable(cx: &LateContext<'_>, mut def_id: DefId, msrv: Msrv) -> bool {
    loop {
        if let Some(stability) = cx.tcx.lookup_stability(def_id) {
            match stability.level {
                // Workaround for items from `core::intrinsics` with a stable export in a different module.
                // Not that we ignore the `since` field as we are already accessing the item in question.
                StabilityLevel::Stable {
                    allowed_through_unstable_modules: Some(_),
                    ..
                } => return true,
                StabilityLevel::Stable { since, .. } => match since {
                    StableSince::Version(v) if !msrv.meets(cx, v) => return false,
                    StableSince::Current if msrv.current(cx).is_none() => return false,
                    StableSince::Err(_) => return false,
                    StableSince::Version(_) | StableSince::Current => {},
                },
                StabilityLevel::Unstable { .. } => return false,
            }
        }

        match cx.tcx.opt_parent(def_id) {
            Some(parent) => def_id = parent,
            None => return true,
        }
    }
}

enum LintKind {
    StdInsteadOfCore,
    StdInsteadOfAlloc,
    AllocInsteadOfCore,
}

impl LintKind {
    fn try_new(used_from: UsedFrom, defined_in: DefinedIn) -> Option<Self> {
        match (used_from, defined_in) {
            (UsedFrom::Alloc, DefinedIn::Core) => Some(AllocInsteadOfCore),
            (UsedFrom::Std, DefinedIn::Core) => Some(StdInsteadOfCore),
            (UsedFrom::Std, DefinedIn::Alloc) => Some(StdInsteadOfAlloc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DefinedIn {
    Core,
    Alloc,
}

impl DefinedIn {
    fn try_new(cx: &LateContext<'_>, def_id: DefId) -> Option<Self> {
        match cx.tcx.crate_name(def_id.krate) {
            sym::alloc => Some(DefinedIn::Alloc),
            sym::core => Some(DefinedIn::Core),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UsedFrom {
    Alloc,
    Std,
}
