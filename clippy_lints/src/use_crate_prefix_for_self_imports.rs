use clippy_utils::diagnostics::span_lint_and_sugg;
use def_id::LOCAL_CRATE;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{Attribute, Item, ItemKind, UsePath, def_id};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::{BytePos, FileName, RealFileName, Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// This lint checks for imports from the current crate that do not use the `crate::` prefix.
    /// It suggests using `crate::` to make it clear that the item is from the same crate.
    ///
    /// ### Why is this bad?
    /// When imports from the current crate lack the `crate::` prefix, it can make the code less readable
    /// because it’s not immediately clear if the imported item is from the current crate or an external dependency.
    /// Using `crate::` for self-imports provides a consistent style, making the origin of each import clear.
    /// This helps reduce confusion and maintain a uniform codebase.
    ///
    /// ### Example
    /// ```rust,ignore
    /// // lib.rs
    /// mod foo;
    /// use foo::bar;
    /// ```
    ///
    /// ```rust,ignore
    /// // foo.rs
    /// #[path = "./foo.rs"]
    /// pub fn bar() {}
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// // lib.rs
    /// mod foo;
    /// use crate::foo::bar;
    /// ```
    ///
    /// ```rust,ignore
    /// // foo.rs
    /// #[path = "./foo.rs"]
    /// pub fn bar() {}
    /// ```
    #[clippy::version = "1.84.0"]
    pub USE_CRATE_PREFIX_FOR_SELF_IMPORTS,
    nursery,
    "checks that imports from the current crate use the `crate::` prefix"
}

#[derive(Clone, Default)]
pub struct UseCratePrefixForSelfImports<'a, 'tcx> {
    /// collect `use` in current block
    use_block: Vec<&'a UsePath<'tcx>>,
    /// collect `mod` in current block
    mod_names: FxHashSet<Symbol>,
    /// spans of `mod`, `use`, and attributes
    spans: Vec<Span>,
}

impl_lint_pass!(UseCratePrefixForSelfImports<'_, '_> => [USE_CRATE_PREFIX_FOR_SELF_IMPORTS]);

impl<'a, 'tcx> LateLintPass<'tcx> for UseCratePrefixForSelfImports<'a, 'tcx> {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'a Item<'tcx>) {
        let FileName::Real(RealFileName::LocalPath(p)) = cx.sess().source_map().span_to_filename(item.span) else {
            self.clear();
            return;
        };
        let Some(file_name) = p.file_name() else {
            self.clear();
            return;
        };
        // only check `main.rs` and `lib.rs`
        if !(file_name == "main.rs" || file_name == "lib.rs") {
            return;
        }

        if self.in_same_block(item.span) {
            self.insert_item(item);
        } else {
            self.deal(cx);
            self.clear();
            self.insert_item(item);
        }
    }

    fn check_attribute(&mut self, cx: &LateContext<'tcx>, attribute: &'a Attribute) {
        let FileName::Real(RealFileName::LocalPath(p)) = cx.sess().source_map().span_to_filename(attribute.span())
        else {
            self.clear();
            return;
        };
        let Some(file_name) = p.file_name() else {
            self.clear();
            return;
        };
        // only check `main.rs` and `lib.rs`
        if !(file_name == "main.rs" || file_name == "lib.rs") {
            return;
        }

        if self.in_same_block(attribute.span()) {
            self.spans.push(attribute.span());
        } else {
            self.deal(cx);
            self.clear();
            self.spans.push(attribute.span());
        }
    }
}

impl<'tcx> UseCratePrefixForSelfImports<'_, 'tcx> {
    fn in_same_block(&self, span: Span) -> bool {
        if self.spans.is_empty() {
            return true;
        }
        if self.spans.iter().any(|x| x.contains(span)) {
            return true;
        }
        if self.spans.iter().any(|x| span.lo() - x.hi() == BytePos(1)) {
            return true;
        }
        false
    }

    fn insert_item(&mut self, item: &Item<'tcx>) {
        match item.kind {
            ItemKind::Mod(_) => {
                self.spans.push(item.span);
                self.mod_names.insert(item.ident.name);
            },
            ItemKind::Use(use_tree, _) => {
                self.spans.push(item.span);
                self.use_block.push(use_tree);
            },
            _ => {},
        }
    }

    fn deal(&self, cx: &LateContext<'tcx>) {
        for use_path in &self.use_block {
            if let Some(segment) = use_path.segments.first()
                && let Res::Def(_, def_id) = segment.res
                && def_id.krate == LOCAL_CRATE
            {
                let root = segment.ident.name;
                if root != rustc_span::symbol::kw::Crate
                    && root != rustc_span::symbol::kw::Super
                    && root != rustc_span::symbol::kw::SelfLower
                    && !self.mod_names.contains(&root)
                {
                    span_lint_and_sugg(
                        cx,
                        USE_CRATE_PREFIX_FOR_SELF_IMPORTS,
                        segment.ident.span,
                        "this import is not clear",
                        "prefix with `crate::`",
                        format!("crate::{root}"),
                        Applicability::MachineApplicable,
                    );
                }
            }
        }
    }

    fn clear(&mut self) {
        self.use_block.clear();
        self.mod_names.clear();
        self.spans.clear();
    }
}
