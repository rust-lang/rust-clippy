use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_from_proc_macro;
use rustc_hir::intravisit::{Visitor, walk_block, walk_item};
use rustc_hir::{Block, HirId, HirIdSet, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::hir::nested_filter;
use rustc_session::impl_lint_pass;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for blocks which are nested beyond a certain threshold.
    ///
    /// Note: Even though this lint is warn-by-default, it will only trigger if a maximum nesting level is defined in the clippy.toml file.
    ///
    /// ### Why is this bad?
    /// It can severely hinder readability.
    ///
    /// ### Example
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// excessive-nesting-threshold = 3
    /// ```
    /// ```rust,ignore
    /// // lib.rs
    /// pub mod a {
    ///     pub struct X;
    ///     impl X {
    ///         pub fn run(&self) {
    ///             if true {
    ///                 // etc...
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// // a.rs
    /// fn private_run(x: &X) {
    ///     if true {
    ///         // etc...
    ///     }
    /// }
    ///
    /// pub struct X;
    /// impl X {
    ///     pub fn run(&self) {
    ///         private_run(self);
    ///     }
    /// }
    /// ```
    /// ```rust,ignore
    /// // lib.rs
    /// pub mod a;
    /// ```
    #[clippy::version = "1.72.0"]
    pub EXCESSIVE_NESTING,
    complexity,
    "checks for blocks nested beyond a certain threshold"
}

impl_lint_pass!(ExcessiveNesting => [EXCESSIVE_NESTING]);

pub struct ExcessiveNesting {
    pub excessive_nesting_threshold: u64,
    pub nodes: HirIdSet,
}

impl ExcessiveNesting {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            excessive_nesting_threshold: conf.excessive_nesting_threshold,
            nodes: HirIdSet::default(),
        }
    }

    pub fn check_node_id(&self, cx: &LateContext<'_>, span: Span, node_id: HirId) {
        if self.nodes.contains(&node_id) {
            span_lint_and_help(
                cx,
                EXCESSIVE_NESTING,
                span,
                "this block is too nested",
                None,
                "try refactoring your code to minimize nesting",
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ExcessiveNesting {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        if self.excessive_nesting_threshold == 0 {
            return;
        }

        let mut visitor = NestingVisitor {
            conf: self,
            cx,
            nest_level: 0,
        };

        cx.tcx.hir_walk_toplevel_module(&mut visitor);
    }

    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'tcx>) {
        self.check_node_id(cx, item.span, item.hir_id());
    }

    fn check_block(&mut self, cx: &LateContext<'_>, block: &Block<'tcx>) {
        self.check_node_id(cx, block.span, block.hir_id);
    }
}

struct NestingVisitor<'conf, 'tcx> {
    conf: &'conf mut ExcessiveNesting,
    cx: &'conf LateContext<'tcx>,
    nest_level: u64,
}

impl<'conf, 'tcx> NestingVisitor<'conf, 'tcx> {
    fn check_indent(&mut self, span: Span, id: HirId) -> bool {
        if self.nest_level > self.conf.excessive_nesting_threshold
            && !span.in_external_macro(self.cx.sess().source_map())
        {
            self.conf.nodes.insert(id);

            return true;
        }

        false
    }
}

impl<'tcx> Visitor<'tcx> for NestingVisitor<'_, 'tcx> {
    type NestedFilter = nested_filter::All;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_block(&mut self, block: &'tcx Block<'_>) {
        // If it's a compiler desugaring (like `for`, `while`, or `async`),
        // keep walking so we reach the inner user blocks!
        if block.span.from_expansion() {
            if block.span.desugaring_kind().is_some() {
                walk_block(self, block);
            }

            return;
        }

        if is_from_proc_macro(self.cx, block) {
            return;
        }

        self.nest_level += 1;

        if !self.check_indent(block.span, block.hir_id) {
            walk_block(self, block);
        }

        self.nest_level -= 1;
    }

    fn visit_item(&mut self, item: &'tcx Item<'tcx>) {
        if item.span.from_expansion() {
            return;
        }

        match &item.kind {
            ItemKind::Trait { .. } | ItemKind::Impl(_) => {
                self.nest_level += 1;

                if !self.check_indent(item.span, item.hir_id()) {
                    walk_item(self, item);
                }

                self.nest_level -= 1;
            },
            ItemKind::Mod(_, module) => {
                let sm = self.cx.sess().source_map();
                let is_inline = module
                    .item_ids
                    .first()
                    .map(|&id| {
                        let inner_item = self.cx.tcx.hir_item(id);
                        sm.lookup_source_file(item.span.lo()).name == sm.lookup_source_file(inner_item.span.lo()).name
                    })
                    .unwrap_or(true);

                if is_inline {
                    self.nest_level += 1;
                    if !self.check_indent(item.span, item.hir_id()) {
                        walk_item(self, item);
                    }
                    self.nest_level -= 1;
                } else {
                    // Reset nesting level for non-inline modules (since these are in another file)
                    let mut visitor = NestingVisitor {
                        conf: self.conf,
                        cx: self.cx,
                        nest_level: 0,
                    };
                    walk_item(&mut visitor, item);
                }
            },
            _ => walk_item(self, item),
        }
    }
}
