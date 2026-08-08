use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_no_std_crate;
use clippy_utils::msrvs::{self, Msrv};
use rustc_errors::Applicability;
use rustc_hir::attrs::CrateType;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{HirId, Item, ItemKind, Path};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::CrateNum;
use rustc_span::sym;
use rustc_span::symbol::kw;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects linkage of `std` within a library crate when the crate itself
    /// does not use `std`.
    ///
    /// ### Why restrict this?
    ///
    /// Linking `std` prevents all downstream dependencies from being `no_std` compatible,
    /// which can be a prerequisite for many targets, such as embedded.
    ///
    /// ### Known problems
    ///
    /// * This lint won't detect usage of `std` that could be replaced with `core` or `alloc`.
    ///   You should consider using `std_instead_of_core` for that.
    /// * Dependencies which link `std` are not detected by this lint.
    /// * This lint only applies to library crates.
    ///   Final artifacts (e.g., binaries) may rely on global items such as `#[panic_handler]`
    ///   which have defaults provided by `std`.
    /// * Cannot detect `alloc` implicit prelude usage, such as `Vec` or `Box`.
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
    msrv: Msrv,
    std_used: bool,
    has_extern_crate_alloc: bool,
    std_crate_num: Option<CrateNum>,
    alloc_crate_num: Option<CrateNum>,
}

impl UnnecessaryStdLinkage {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            msrv: conf.msrv,
            std_used: false,
            has_extern_crate_alloc: false,
            std_crate_num: None,
            alloc_crate_num: None,
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryStdLinkage {
    fn check_path(&mut self, _: &LateContext<'tcx>, path: &Path<'tcx>, _: HirId) {
        if let Some(krate_num) = self.std_crate_num
            && !self.std_used
            && path_starts_with(path, krate_num)
        {
            self.std_used = true;
        }
    }

    fn check_item(&mut self, _: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        match item.kind {
            ItemKind::ExternCrate(original_name, ident) => {
                match original_name.unwrap_or(ident.name) {
                    sym::std => {
                        // extern crate std; is implicit for non-`no_std` crates
                        if !self.std_used && item.span.can_be_used_for_suggestions() {
                            self.std_used = true;
                        }
                    },
                    sym::alloc => {
                        self.has_extern_crate_alloc = true;
                    },
                    _ => {},
                }
            },
            ItemKind::Use(path, ..) => {
                if let Some(krate_num) = self.std_crate_num
                    && !self.std_used
                    && path_starts_with(path, krate_num)
                {
                    self.std_used = true;
                }
            },
            _ => {},
        }
    }

    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.std_used = false;
        self.has_extern_crate_alloc = false;
        self.std_crate_num = None;
        self.alloc_crate_num = None;

        let mut krates = cx.tcx.used_crates(()).iter();
        while let Some(&krate_num) = krates.next()
            && self.std_crate_num.is_none()
            && self.alloc_crate_num.is_none()
        {
            match cx.tcx.crate_name(krate_num) {
                sym::std => self.std_crate_num = Some(krate_num),
                sym::alloc => self.alloc_crate_num = Some(krate_num),
                _ => {},
            }
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        if !is_no_std_crate(cx)
            && !self.std_used
            && cx.tcx.crate_types().iter().any(|&t| matches!(t, CrateType::Rlib))
            && self.msrv.meets(cx, msrvs::NO_STD)
        {
            let root_span = cx.tcx.hir_root_module().spans.inject_use_span;
            span_lint_and_then(
                cx,
                UNNECESSARY_STD_LINKAGE,
                root_span,
                "unnecessary linkage of `std`",
                |diag| {
                    if !self.has_extern_crate_alloc && self.msrv.meets(cx, msrvs::ALLOC_CRATE) {
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
    }
}

fn path_starts_with<T>(path: &Path<'_, T>, krate: CrateNum) -> bool {
    if let Some(segment) = path.segments.iter().find(|segment| segment.ident.name != kw::PathRoot)
        && let Res::Def(DefKind::Mod, crate_def_id) = segment.res
        && crate_def_id.krate == krate
        && crate_def_id.is_crate_root()
    {
        true
    } else {
        false
    }
}
