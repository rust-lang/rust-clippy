use clippy_config::Conf;
use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::is_from_proc_macro;
use clippy_utils::msrvs::Msrv;
use clippy_utils::paths::{PathNS, lookup_path};
use rustc_errors::{Applicability, MultiSpan};
use rustc_hir::def::{DefKind, Namespace, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{Block, Body, HirId, Path, PathSegment, StabilityLevel, StableSince};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::symbol::kw;
use rustc_span::{Span, Symbol, sym};

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
    lint_points: Option<(Span, Vec<LintPoint>)>,
    msrv: Msrv,
}

impl StdReexports {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            lint_points: Option::default(),
            msrv: conf.msrv,
        }
    }

    fn lint_if_finish(&mut self, cx: &LateContext<'_>, krate: Span, lint_point: LintPoint) {
        match &mut self.lint_points {
            Some((prev_krate, prev_lints)) if prev_krate.overlaps(krate) => {
                prev_lints.push(lint_point);
            },
            _ => emit_lints(cx, self.lint_points.replace((krate, vec![lint_point]))),
        }
    }
}

#[derive(Debug)]
struct LintPoint {
    span: Span,
    used_from: Symbol,
    is_stable: bool,
    available_from_core: bool,
    available_from_alloc: bool,
}

impl<'tcx> LateLintPass<'tcx> for StdReexports {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, _: HirId) {
        if let Res::Def(def_kind, def_id) = path.res
            && let Some(first_segment) = get_first_segment(path)
            && !path.span.in_external_macro(cx.sess().source_map())
            && !is_from_proc_macro(cx, &first_segment.ident)
            && !matches!(def_kind, DefKind::Macro(_))
            && let Some(last_segment) = path.segments.last()
            && let Res::Def(DefKind::Mod, crate_def_id) = first_segment.res
            && crate_def_id.is_crate_root()
        {
            let namespace = match def_kind.ns() {
                Some(Namespace::TypeNS) => PathNS::Type,
                Some(Namespace::ValueNS) => PathNS::Value,
                Some(Namespace::MacroNS) => PathNS::Macro,
                None => PathNS::Arbitrary,
            };

            let mut path_new = path
                .segments
                .iter()
                .map(|segment| segment.ident.name)
                .skip_while(|&segment| segment == kw::PathRoot)
                .collect::<Vec<_>>();

            path_new[0] = sym::core;
            let available_from_core = lookup_path(cx.tcx, namespace, &path_new).contains(&def_id);

            path_new[0] = sym::alloc;
            let available_from_alloc = lookup_path(cx.tcx, namespace, &path_new).contains(&def_id);

            self.lint_if_finish(
                cx,
                first_segment.ident.span,
                LintPoint {
                    span: last_segment.ident.span,
                    used_from: first_segment.ident.name,
                    is_stable: is_stable(cx, def_id, self.msrv),
                    available_from_core,
                    available_from_alloc,
                },
            );
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

fn emit_lints(cx: &LateContext<'_>, lint_points: Option<(Span, Vec<LintPoint>)>) {
    let Some((krate_span, mut lint_points)) = lint_points else {
        return;
    };

    // It's possible for multiple items to come from the same path.
    // For example, `std::vec` refers to a macro and a module.
    // In these cases, it's possible for them to have different availabilities.
    // `std::vec` as a macro and a module are both defined in `alloc` and unavailable in `core`.
    // Whereas, `std::env` is not available in `alloc`, and only the macro is available in `core`.
    // Since we aren't checking which of the shadowed items the user needs, we take the intersection
    // of these availabilities to ensure we don't provide the user a false positive.
    lint_points.sort_by_key(|lint_point| lint_point.span);
    lint_points.dedup_by(|a, b| {
        if a.span == b.span {
            b.is_stable &= a.is_stable;
            b.available_from_alloc &= a.available_from_alloc;
            b.available_from_core &= a.available_from_core;
            true
        } else {
            false
        }
    });

    let mut core_span = MultiSpan::new();
    let mut alloc_span = MultiSpan::new();
    let mut all_from_std = true;
    let mut all_from_alloc = true;
    let mut all_core = true;
    let mut all_alloc = true;

    for lint_point in lint_points {
        all_from_std &= lint_point.used_from == sym::std;
        all_from_alloc &= lint_point.used_from == sym::alloc;

        if lint_point.is_stable && lint_point.available_from_core {
            core_span.push_primary_span(lint_point.span);
        } else {
            all_core = false;
        }

        if lint_point.is_stable && lint_point.available_from_alloc {
            if !lint_point.available_from_core {
                alloc_span.push_primary_span(lint_point.span);
            }
        } else {
            all_alloc = false;
        }
    }

    let mut helps = Vec::new();
    let mut suggestions = Vec::new();

    if all_from_std {
        if all_core {
            suggestions.push((STD_INSTEAD_OF_CORE, &sym::std, &sym::core));
            helps.push((STD_INSTEAD_OF_ALLOC, &sym::std, &sym::alloc, alloc_span));
        } else if all_alloc {
            suggestions.push((STD_INSTEAD_OF_ALLOC, &sym::std, &sym::alloc));
            helps.push((STD_INSTEAD_OF_CORE, &sym::std, &sym::core, core_span));
        } else {
            helps.push((STD_INSTEAD_OF_CORE, &sym::std, &sym::core, core_span));
            helps.push((STD_INSTEAD_OF_ALLOC, &sym::std, &sym::alloc, alloc_span));
        }
    } else if all_from_alloc {
        if all_core {
            suggestions.push((ALLOC_INSTEAD_OF_CORE, &sym::alloc, &sym::core));
        } else {
            helps.push((ALLOC_INSTEAD_OF_CORE, &sym::alloc, &sym::core, core_span));
        }
    }

    for (lint, used_mod, replace_with) in suggestions {
        span_lint_and_sugg(
            cx,
            lint,
            krate_span,
            format!("used import from `{used_mod}` instead of `{replace_with}`"),
            format!("consider importing the item from `{replace_with}`"),
            (*replace_with).to_string(),
            Applicability::MachineApplicable,
        );
    }

    for (lint, used_mod, replace_with, span) in helps {
        for &span in span.primary_spans() {
            span_lint_and_help(
                cx,
                lint,
                span,
                format!("used import from `{used_mod}` instead of `{replace_with}`"),
                None,
                format!("consider importing the item from `{replace_with}`"),
            );
        }
    }
}

/// Returns the first named segment of a [`Path`].
///
/// If this is a global path (such as `::std::fmt::Debug`), then the segment after [`kw::PathRoot`]
/// is returned.
fn get_first_segment<'tcx>(path: &Path<'tcx>) -> Option<&'tcx PathSegment<'tcx>> {
    match path.segments {
        // A global path will have PathRoot as the first segment. In this case, return the segment after.
        [x, y, ..] if x.ident.name == kw::PathRoot => Some(y),
        [x, ..] => Some(x),
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
