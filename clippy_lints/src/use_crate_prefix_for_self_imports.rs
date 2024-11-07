use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use def_id::LOCAL_CRATE;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{Item, ItemKind, def_id};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::declare_lint_pass;
use rustc_span::{FileName, RealFileName};

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

declare_lint_pass!(UseCratePrefixForSelfImports => [USE_CRATE_PREFIX_FOR_SELF_IMPORTS]);

impl LateLintPass<'_> for UseCratePrefixForSelfImports {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        let FileName::Real(RealFileName::LocalPath(p)) = cx.sess().source_map().span_to_filename(item.span) else {
            return;
        };
        let Some(file_name) = p.file_name() else {
            return;
        };
        // only check `main.rs` and `lib.rs`
        if !(file_name == "main.rs" || file_name == "lib.rs") {
            return;
        }

        if let ItemKind::Use(use_path, _) = &item.kind {
            if let Some(segment) = use_path.segments.first()
                && let Res::Def(_, def_id) = segment.res
                && def_id.krate == LOCAL_CRATE
            {
                let root = segment.ident.name;
                if root != rustc_span::symbol::kw::Crate
                    && root != rustc_span::symbol::kw::Super
                    && root != rustc_span::symbol::kw::SelfLower
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
