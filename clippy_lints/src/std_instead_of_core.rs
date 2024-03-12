use std::hash::{Hash, Hasher};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_from_proc_macro;
use rustc_data_structures::fx::FxIndexSet;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_hir::{HirId, Item, ItemKind, Path, PathSegment, UseKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::impl_lint_pass;
use rustc_span::symbol::kw;
use rustc_span::{sym, Span};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Finds items imported through `std` when available through `core`.
    ///
    /// ### Why is this bad?
    ///
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

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Finds items imported through `std` when available through `alloc`.
    ///
    /// ### Why is this bad?
    ///
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
    ///
    /// Finds items imported through `alloc` when available through `core`.
    ///
    /// ### Why is this bad?
    ///
    /// Crates which have `no_std` compatibility and may optionally require alloc may wish to ensure types are
    /// imported from core to ensure disabling `alloc` does not cause the crate to fail to compile. This lint
    /// is also useful for crates migrating to become `no_std` compatible.
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

#[derive(Default)]
pub struct StdReexports {
    // Paths which can be either a module or a macro (e.g. `std::env`) will cause this check to happen
    // twice. First for the mod, second for the macro. This is used to avoid the lint reporting for the macro
    // when the path could be also be used to access the module.
    prev_span: Span,
    open_use: Option<OpenUseSpan>
}

impl_lint_pass!(StdReexports => [STD_INSTEAD_OF_CORE, STD_INSTEAD_OF_ALLOC, ALLOC_INSTEAD_OF_CORE]);

#[derive(Debug)]
struct OpenUseSpan {
    container: Span,
    members: FxIndexSet<UseSpanMember>
}

#[derive(Debug, Copy, Clone)]
struct UseSpanMember {
    inner: Span,
    lint_data: LintData,
}

impl PartialEq for UseSpanMember {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl Eq for UseSpanMember {}

impl Hash for UseSpanMember {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

#[derive(Debug, Copy, Clone)]
enum LintData {
    CanReplace {
        lint: &'static crate::Lint,
        used_mod: &'static str,
        replace_with: &'static str,
    },
    NoReplace,
}

impl<'tcx> LateLintPass<'tcx> for StdReexports {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, _: HirId) {
        if let Res::Def(_, def_id) = path.res
            && let Some(first_segment) = get_first_segment(path)
            && is_stable(cx, def_id)
            && !in_external_macro(cx.sess(), path.span)
            && !is_from_proc_macro(cx, &first_segment.ident)
        {
            let lint_data = match first_segment.ident.name {
                sym::std => match cx.tcx.crate_name(def_id.krate) {
                    sym::core => LintData::CanReplace {
                        lint: STD_INSTEAD_OF_CORE,
                        used_mod: "std",
                        replace_with: "core",
                    },
                    sym::alloc => LintData::CanReplace {
                        lint: STD_INSTEAD_OF_ALLOC,
                        used_mod: "std",
                        replace_with: "alloc",
                    },
                    _ => {
                        self.prev_span = first_segment.ident.span;
                        LintData::NoReplace
                    },
                },
                sym::alloc => {
                    if cx.tcx.crate_name(def_id.krate) == sym::core {
                        LintData::CanReplace {
                            lint: ALLOC_INSTEAD_OF_CORE,
                            used_mod: "alloc",
                            replace_with: "core",
                        }
                    } else {
                        self.prev_span = first_segment.ident.span;
                        LintData::NoReplace
                    }
                },
                _ => return,
            };
            if let Some(in_use) = self.open_use.as_mut() {
                in_use.members.insert(UseSpanMember {
                    inner: path.span,
                    lint_data,
                });
                return;
            }
            if let LintData::CanReplace { lint, used_mod, replace_with } = lint_data {
                if first_segment.ident.span != self.prev_span {
                    span_lint_and_sugg(
                        cx,
                        lint,
                        first_segment.ident.span,
                        &format!("used import from `{used_mod}` instead of `{replace_with}`"),
                        &format!("consider importing the item from `{replace_with}`"),
                        replace_with.to_string(),
                        Applicability::MachineApplicable,
                    );
                    self.prev_span = first_segment.ident.span;
                }
            }
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if matches!(item.kind, ItemKind::Use(_, UseKind::ListStem)) {
            self.open_use = Some(OpenUseSpan {
                container: item.span,
                members: FxIndexSet::default(),
            })
        }

    }

    fn check_item_post(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let Some(collected_use) = self.open_use.take() {

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

/// Checks if all ancestors of `def_id` are stable, to avoid linting
/// [unstable moves](https://github.com/rust-lang/rust/pull/95956)
fn is_stable(cx: &LateContext<'_>, mut def_id: DefId) -> bool {
    loop {
        if cx
            .tcx
            .lookup_stability(def_id)
            .map_or(false, |stability| stability.is_unstable())
        {
            return false;
        }

        match cx.tcx.opt_parent(def_id) {
            Some(parent) => def_id = parent,
            None => return true,
        }
    }
}
