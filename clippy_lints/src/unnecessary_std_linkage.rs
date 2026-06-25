use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_no_std_crate;
use rustc_errors::Applicability;
use rustc_hir::attrs::CrateType;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{HirId, Item, ItemKind, Path};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::symbol::kw;
use rustc_span::{Symbol, sym};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects linkage of `std` within a library crate when the crate itself
    /// does not use `std.
    ///
    /// ### Why restrict this?
    ///
    /// Linking `std` prevents all downstream dependencies from being `no_std` compatible,
    /// which can be a prerequisite for many targets, such as embedded.
    ///
    /// ### Known problems
    ///
    /// * This lint wont detect usage of `std` that could be replaced with `core` or `alloc`.
    ///   You should consider using `std_instead_of_core` for that.
    /// * Dependencies which link `std` are not detected by this lint.
    /// * This lint only applies to library crates.
    ///   Final artifacts (e.g., binaries) may rely on global items such as `#[panic_handler]`
    ///   which have defaults provided by `std`.
    /// * Cannot detect non-`core` implicit prelude usage, such as `Vec` or `println!`.
    ///
    /// ### Example
    /// ```ignore
    /// /// My special API which doesn't directly use `std`.
    /// pub struct Foo(pub usize);
    /// ```
    /// Use instead:
    /// ```ignore
    /// #![no_std]
    ///
    /// /// My special API which doesn't directly use `std`.
    /// pub struct Foo(pub usize);
    /// ```
    #[clippy::version = "1.98.0"]
    pub UNNECESSARY_STD_LINKAGE,
    nursery,
    "unnecessary linkage of `std`"
}

impl_lint_pass!(UnnecessaryStdLinkage => [UNNECESSARY_STD_LINKAGE]);

pub struct UnnecessaryStdLinkage {
    std_used: bool,
    has_extern_crate_alloc: bool,
}

impl UnnecessaryStdLinkage {
    pub fn new() -> Self {
        Self {
            std_used: false,
            has_extern_crate_alloc: false,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryStdLinkage {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, _: HirId) {
        self.std_used |= path_references(cx, path, sym::std);
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        match item.kind {
            ItemKind::ExternCrate(original_name, ident) => {
                match original_name.unwrap_or(ident.name) {
                    sym::std => {
                        // extern crate std; is implicit for non-`no_std` crates
                        self.std_used |= item.span.can_be_used_for_suggestions();
                    },
                    sym::alloc => {
                        self.has_extern_crate_alloc = true;
                    },
                    _ => {},
                }
            },
            ItemKind::Use(path, ..) => {
                self.std_used |= path_starts_with(cx, path, sym::std);
            },
            ItemKind::Static(..)
            | ItemKind::Const(..)
            | ItemKind::Fn { .. }
            | ItemKind::Macro(..)
            | ItemKind::Mod(..)
            | ItemKind::ForeignMod { .. }
            | ItemKind::GlobalAsm { .. }
            | ItemKind::TyAlias(..)
            | ItemKind::Enum(..)
            | ItemKind::Struct(..)
            | ItemKind::Union(..)
            | ItemKind::Trait { .. }
            | ItemKind::TraitAlias(..)
            | ItemKind::Impl(..) => {},
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        if !is_no_std_crate(cx) && !self.std_used && cx.tcx.crate_types().iter().any(|&t| matches!(t, CrateType::Rlib))
        {
            let root_span = cx.tcx.hir_root_module().spans.inject_use_span;
            span_lint_and_then(
                cx,
                UNNECESSARY_STD_LINKAGE,
                root_span,
                "unnecessary linkage of `std`",
                |diag| {
                    if !self.has_extern_crate_alloc {
                        diag.note(
                            "consider adding `extern crate alloc;` for access to allocation items such as `Box`, `Vec`",
                        );
                    }
                    diag.span_suggestion(
                        root_span,
                        "add `#![no_std]` to the crate root",
                        "#![no_std]\n",
                        Applicability::MaybeIncorrect,
                    );
                },
            );
        }

        self.std_used = false;
        self.has_extern_crate_alloc = false;
    }
}

fn path_references<'tcx>(cx: &LateContext<'tcx>, path: &Path<'tcx>, symbol: Symbol) -> bool {
    path_starts_with(cx, path, symbol) || path_resolves_into(cx, path, symbol)
}

fn path_starts_with<'tcx, T>(_cx: &LateContext<'tcx>, path: &Path<'tcx, T>, symbol: Symbol) -> bool {
    if let Some(segment) = path.segments.iter().find(|segment| segment.ident.name != kw::PathRoot)
        && let Res::Def(DefKind::Mod, crate_def_id) = segment.res
        && crate_def_id.is_crate_root()
        && segment.ident.name == symbol
    {
        true
    } else {
        false
    }
}

fn path_resolves_into<'tcx>(cx: &LateContext<'tcx>, path: &Path<'tcx>, symbol: Symbol) -> bool {
    if let Some(def_id) = path.res.opt_def_id()
        && cx.tcx.crate_name(def_id.krate) == symbol
    {
        true
    } else {
        false
    }
}
