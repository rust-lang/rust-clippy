use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint;
use clippy_utils::{fulfill_or_allowed, is_from_proc_macro};
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{CRATE_DEF_INDEX, DefId};
use rustc_hir::{HirId, ItemKind, Node, OwnerNode, Path};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::symbol::kw;
use rustc_span::{Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of items through absolute paths, like `std::env::current_dir`.
    ///
    /// ### Why restrict this?
    /// Many codebases have their own style when it comes to importing, but one that is seldom used
    /// is using absolute paths *everywhere*. This is generally considered unidiomatic, and you
    /// should add a `use` statement.
    ///
    /// The default maximum segments (2) is pretty strict, you may want to increase this in
    /// `clippy.toml`.
    ///
    /// Note: One exception to this is code from macro expansion - this does not lint such cases, as
    /// using absolute paths is the proper way of referencing items in one.
    ///
    /// ### Known issues
    ///
    /// There are currently a few cases which are not caught by this lint:
    /// * Macro calls. e.g. `path::to::macro!()`
    /// * Derive macros. e.g. `#[derive(path::to::macro)]`
    /// * Attribute macros. e.g. `#[path::to::macro]`
    ///
    /// ### Example
    /// ```no_run
    /// let x = std::f64::consts::PI;
    /// ```
    /// Use any of the below instead, or anything else:
    /// ```no_run
    /// use std::f64;
    /// use std::f64::consts;
    /// use std::f64::consts::PI;
    /// let x = f64::consts::PI;
    /// let x = consts::PI;
    /// let x = PI;
    /// use std::f64::consts as f64_consts;
    /// let x = f64_consts::PI;
    /// ```
    #[clippy::version = "1.73.0"]
    pub ABSOLUTE_PATHS,
    restriction,
    "checks for usage of an item without a `use` statement"
}

impl_lint_pass!(AbsolutePaths => [ABSOLUTE_PATHS]);

pub struct AbsolutePaths {
    pub max_segments: u64,
    pub allowed_crates: FxHashSet<Symbol>,
    pub max_occurrences: u64,
    occurrences: Vec<((DefId, DefId), Span)>, // To track count of occurences
}

impl AbsolutePaths {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            max_segments: conf.absolute_paths_max_segments,
            allowed_crates: conf
                .absolute_paths_allowed_crates
                .iter()
                .map(|x| Symbol::intern(x))
                .collect(),
            max_occurrences: conf.absolute_paths_max_occurrences,
            occurrences: Vec::new(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for AbsolutePaths {
    // We should only lint `QPath::Resolved`s, but since `Path` is only used in `Resolved` and `UsePath`
    // we don't need to use a visitor or anything as we can just check if the `Node` for `hir_id` isn't
    // a `Use`
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, hir_id: HirId) {
        let segments = match path.segments {
            [] | [_] => return,
            // Don't count enum variants and trait items as part of the length.
            [rest @ .., _]
                if let [.., s] = rest
                    && matches!(s.res, Res::Def(DefKind::Enum | DefKind::Trait | DefKind::TraitAlias, _)) =>
            {
                rest
            },
            path => path,
        };
        if let [s1, s2, ..] = segments
            && let has_root = s1.ident.name == kw::PathRoot
            && let first = if has_root { s2 } else { s1 }
            && let len = segments.len() - usize::from(has_root)
            && len as u64 > self.max_segments
            && let crate_name = if let Res::Def(DefKind::Mod, DefId { index, .. }) = first.res
                && index == CRATE_DEF_INDEX
            {
                // `other_crate::foo` or `::other_crate::foo`
                first.ident.name
            } else if first.ident.name == kw::Crate || has_root {
                // `::foo` or `crate::foo`
                kw::Crate
            } else {
                return;
            }
            && !path.span.from_expansion()
            && let node = cx.tcx.hir_node(hir_id)
            && !matches!(node, Node::Item(item) if matches!(item.kind, ItemKind::Use(..)))
            && !self.allowed_crates.contains(&crate_name)
            && !is_from_proc_macro(cx, path)
        {
            if self.max_occurrences == 0 {
                // Default behaviour : emit lint directly, no need for occurence tracking
                span_lint(
                    cx,
                    ABSOLUTE_PATHS,
                    path.span,
                    "consider bringing this path into scope with the `use` keyword",
                );
            } else if let Res::Def(_, def_id) = path.res
                && !fulfill_or_allowed(cx, ABSOLUTE_PATHS, [hir_id])
            {
                // Occurence based behaviour : accumulate spans and emit lint in check_crate_post
                // if the occurence count is exceeded
                let module = cx
                    .tcx
                    .hir_parent_owner_iter(hir_id)
                    .find(|(_, node)| {
                        if let OwnerNode::Item(item) = node {
                            matches!(item.kind, ItemKind::Mod(..))
                        } else {
                            matches!(node, OwnerNode::Crate(..))
                        }
                    })
                    .map(|(id, _)| id);
                if let Some(module) = module {
                    self.occurrences.push(((def_id, module.to_def_id()), path.span));
                }
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Only runs when absolute_paths_max_occurrences > 0.
        // Emit lints for any path that exceeded the per-file occurrence threshold.
        self.occurrences
            .sort_by_key(|((item_did, mod_did), span)| (item_did.index.as_u32(), mod_did.index.as_u32(), span.lo()));
        for chunk in self.occurrences.chunk_by(|(key1, _), (key2, _)| key1 == key2) {
            if chunk.len() as u64 > self.max_occurrences {
                for (_, span) in chunk {
                    span_lint(
                        cx,
                        ABSOLUTE_PATHS,
                        *span,
                        "this absolute path is used too many times, consider bringing it into scope with the `use` keyword",
                    );
                }
            }
        }
    }
}
