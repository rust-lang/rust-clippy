use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::Msrv;
use itertools::Itertools as _;
use rustc_errors::{Applicability, Diag};
use rustc_hir::def::{DefKind, PerNS, Res};
use rustc_hir::def_id::{CrateNum, DefId};
use rustc_hir::{HirId, Item, ItemKind, Path, PathSegment, StabilityLevel, StableSince, UseKind};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_session::impl_lint_pass;
use rustc_span::symbol::kw;
use rustc_span::{Span, sym};

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
    /// Paths which could be candidates for linting.
    lint_points: Vec<LintPoint>,
    /// Tracks nesting when linting a multi-import `use` statement.
    item_context: Vec<ItemContext>,
    /// Current Minimum Supported Rust Version.
    msrv: Msrv,
}

impl StdReexports {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            lint_points: Vec::new(),
            item_context: Vec::new(),
            msrv: conf.msrv,
        }
    }

    fn lint_if_finish(&mut self, cx: &LateContext<'_>, item: Span) {
        while let Some(top) = self.item_context.pop_if(|top| !top.span.contains(item)) {
            let count = match LintPoint::merge_top(&self.lint_points, &top) {
                Some(merged) => {
                    self.lint_points.truncate(self.lint_points.len() - top.lint_points);
                    self.lint_points.push(merged);
                    1
                },
                None => top.lint_points,
            };

            if let Some(next_top) = self.item_context.last_mut() {
                next_top.lint_points += count;
            }
        }

        if self.item_context.is_empty() {
            emit_lints(cx, self);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LintPoint {
    first: Path<'static, DefId>,
    last: Path<'static, PerNS<Option<DefId>>>,
}

impl<'tcx> LateLintPass<'tcx> for StdReexports {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, _: HirId) {
        if let Some(lint_point) = LintPoint::try_from_path(path) {
            if let Some(last) = self.lint_points.last_mut()
                && last.last.span == lint_point.last.span
            {
                last.last.res = PerNS {
                    value_ns: last.last.res.value_ns.or(lint_point.last.res.value_ns),
                    type_ns: last.last.res.type_ns.or(lint_point.last.res.type_ns),
                    macro_ns: last.last.res.macro_ns.or(lint_point.last.res.macro_ns),
                };
            } else {
                if let Some(top) = self.item_context.last_mut()
                    && top.span.contains(lint_point.last.span)
                {
                    top.lint_points += 1;
                } else {
                    let span = path
                        .segments
                        .iter()
                        .map(|s| s.ident.span)
                        .reduce(Span::to)
                        .unwrap_or(path.span);
                    self.lint_if_finish(cx, span);
                    self.item_context.push(ItemContext { span, lint_points: 1 });
                }

                self.lint_points.push(lint_point);
            }
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        self.lint_if_finish(cx, item.span);

        if let ItemKind::Use(_path, UseKind::ListStem) = item.kind {
            self.item_context.push(ItemContext {
                span: item.span,
                lint_points: 0,
            });
        }
    }

    fn check_item_post(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        self.lint_if_finish(cx, item.span);
    }
}

fn emit_lints(cx: &LateContext<'_>, this: &mut StdReexports) {
    this.lint_points
        .sort_by_key(|path| (path.first.span, path.first.res.krate, path.defined_in()));

    let mut drain = this
        .lint_points
        .drain(..)
        .filter(|p| p.should_emit(cx, this.msrv))
        .peekable();

    while let Some(path) = drain.next() {
        let mut spans = drain
            .peeking_take_while(|other| path.compatible_with(other))
            .map(|other| other.last.span)
            .peekable();

        let used_from = cx.tcx.crate_name(path.first.res.krate);
        let defined_in = cx.tcx.crate_name(path.defined_in());

        let (suggestion, lint, message, help) = match (used_from, defined_in) {
            (sym::std, sym::core) => (
                Some(sym::core),
                STD_INSTEAD_OF_CORE,
                "used import from `std` instead of `core`",
                "consider importing the item from `core`",
            ),
            (sym::std, sym::alloc) => (
                Some(sym::alloc),
                STD_INSTEAD_OF_ALLOC,
                "used import from `std` instead of `alloc`",
                "consider importing the item from `alloc`",
            ),
            (sym::alloc, sym::core) => (
                Some(sym::core),
                ALLOC_INSTEAD_OF_CORE,
                "used import from `alloc` instead of `core`",
                "consider importing the item from `core`",
            ),
            _ => continue,
        };

        let should_suggest = path.last.span.contains(path.first.span) && path.first.res.is_crate_root();

        let then = |diag: &mut Diag<'_, ()>| {
            if should_suggest {
                if let Some(suggestion) = suggestion {
                    diag.span_suggestion(path.first.span, help, suggestion, Applicability::MaybeIncorrect);
                } else {
                    diag.help(help);
                    diag.help("consider adding an `extern crate` statement at the crate root");
                }
            } else {
                diag.help(help);
            }
        };

        if spans.peek().is_none() {
            let span = if should_suggest {
                path.first.span
            } else {
                path.last.span
            };

            span_lint_and_then(cx, lint, span, message, then);
        } else {
            for span in core::iter::once(path.last.span).chain(spans) {
                span_lint_and_then(cx, lint, span, message, then);
            }
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

impl LintPoint {
    fn try_from_path(path: &Path<'_>) -> Option<Self> {
        let first = get_first_segment(path)?;
        let last = path.segments.last()?;

        if !matches!(first.res, Res::Def(DefKind::Mod, _)) {
            return None;
        }

        Some(LintPoint {
            first: Path {
                span: first.ident.span,
                res: first.res.opt_def_id()?,
                segments: &[],
            },
            last: Path {
                span: last.ident.span,
                res: {
                    let mut res = PerNS::default();
                    res[path.res.ns()?] = Some(path.res.opt_def_id()?);
                    res
                },
                segments: &[],
            },
        })
    }

    /// Indicates that two [`LintPoint`]s could be merged.
    fn compatible_with(&self, other: &Self) -> bool {
        self.first.span == other.first.span
            && self.first.res == other.first.res
            && self
                .last
                .res
                .present_items()
                .all(|a| other.last.res.present_items().all(|b| a.krate == b.krate))
    }

    /// Indicates this [`LintPoint`] should be emitted to the user.
    fn should_emit(&self, cx: &LateContext<'_>, msrv: Msrv) -> bool {
        // FIXME(#11159): Delete this.
        if !self.first.res.is_crate_root() {
            return false;
        }

        // FIXME(#17260): Delete this.
        if self.last.res.value_ns.is_none() && self.last.res.type_ns.is_none() && self.last.res.macro_ns.is_some() {
            return false;
        }

        // NOTE:
        // Consider using `self.first.span.can_be_used_for_suggestions()`

        !self.first.span.in_external_macro(cx.sess().source_map())
            && self
                .last
                .res
                .present_items()
                .all(|a| is_stable(cx, a, msrv) && self.first.res.krate != a.krate && self.defined_in() == a.krate)
    }

    fn defined_in(&self) -> CrateNum {
        self.last
            .res
            .present_items()
            .next()
            .expect("LintPoint only created if at least one Namespace resolved")
            .krate
    }

    fn merge_top(lint_points: &[Self], top: &ItemContext) -> Option<Self> {
        lint_points
            .iter()
            .rev()
            .take(top.lint_points)
            .try_fold(Option::<&Self>::None, |a, b| match a {
                Some(a) if a.compatible_with(b) => Some(Some(a)),
                None => Some(Some(b)),
                _ => None,
            })
            .flatten()
            .map(|representative| Self {
                last: Path {
                    span: top.span,
                    ..representative.last
                },
                ..*representative
            })
    }
}

#[derive(Debug)]
struct ItemContext {
    span: Span,
    lint_points: usize,
}
