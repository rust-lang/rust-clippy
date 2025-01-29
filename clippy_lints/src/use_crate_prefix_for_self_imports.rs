use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use def_id::LOCAL_CRATE;
use rustc_data_structures::fx::FxHashSet;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{Item, ItemKind, def_id};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::{BytePos, FileName, RealFileName, Symbol};

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
    style,
    "checks that imports from the current crate use the `crate::` prefix"
}

#[derive(Clone, Default)]
pub struct UseCratePrefixForSelfImports<'a, 'tcx> {
    /// code block of `use <foo>` or `mod <foo>`
    use_block: Vec<&'a Item<'tcx>>,
}

impl_lint_pass!(UseCratePrefixForSelfImports<'_, '_> => [USE_CRATE_PREFIX_FOR_SELF_IMPORTS]);

impl<'a, 'tcx> LateLintPass<'tcx> for UseCratePrefixForSelfImports<'a, 'tcx> {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'a Item<'tcx>) {
        let FileName::Real(RealFileName::LocalPath(p)) = cx.sess().source_map().span_to_filename(item.span) else {
            self.use_block.clear();
            return;
        };
        let Some(file_name) = p.file_name() else {
            self.use_block.clear();
            return;
        };
        // only check `main.rs` and `lib.rs`
        if !(file_name == "main.rs" || file_name == "lib.rs") {
            self.use_block.clear();
            return;
        }

        match item.kind {
            ItemKind::Mod(_) | ItemKind::Use(_, _) => {},
            _ => return,
        }

        if self.in_same_block(item) {
            self.use_block.push(item);
        } else {
            self.deal(cx);
            self.use_block.clear();
            self.use_block.push(item);
        }
    }
}

impl<'tcx> UseCratePrefixForSelfImports<'_, 'tcx> {
    fn in_same_block(&self, item: &Item<'tcx>) -> bool {
        if self.use_block.is_empty() {
            return true;
        }
        if self.use_block.iter().any(|x| x.span.contains(item.span)) {
            return true;
        }
        if let Some(prev_item) = self.use_block.last()
            && item.span.lo() - prev_item.span.hi() == BytePos(1)
        {
            return true;
        }
        false
    }

    fn deal(&self, cx: &LateContext<'tcx>) {
        let mod_names: FxHashSet<Symbol> = self
            .use_block
            .iter()
            .filter_map(|item| match item.kind {
                ItemKind::Mod(_) => Some(item.ident.name),
                _ => None,
            })
            .collect();

        for item in &self.use_block {
            if let ItemKind::Use(use_path, _) = &item.kind {
                if let Some(segment) = use_path.segments.first()
                    && let Res::Def(_, def_id) = segment.res
                    && def_id.krate == LOCAL_CRATE
                {
                    let root = segment.ident.name;
                    if root != rustc_span::symbol::kw::Crate
                        && root != rustc_span::symbol::kw::Super
                        && root != rustc_span::symbol::kw::SelfLower
                        && !mod_names.contains(&root)
                    {
                        span_lint_and_sugg(
                            cx,
                            USE_CRATE_PREFIX_FOR_SELF_IMPORTS,
                            segment.ident.span,
                            "this import is not clear",
                            "prefix with `crate::`",
                            format!("crate::{}", snippet_opt(cx, segment.ident.span).unwrap()),
                            Applicability::MachineApplicable,
                        );
                    }
                }
            }
        }
    }
}
