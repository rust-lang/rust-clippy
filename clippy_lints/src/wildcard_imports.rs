use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{snippet, snippet_with_applicability};
use clippy_utils::{is_in_test, is_lint_allowed};
use rustc_data_structures::fx::{FxHashSet, FxIndexSet};
use rustc_errors::{Applicability, DiagMessage};
use rustc_hir::def::{DefKind, PerNS, Res};
use rustc_hir::{Item, ItemKind, Path, PathSegment, UseKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext as _, impl_lint_pass};
use rustc_middle::ty;
use rustc_span::symbol::{STDLIB_STABLE_CRATES, Symbol, kw, sym};
use rustc_span::{BytePos, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `use Enum::*`.
    ///
    /// ### Why is this bad?
    /// It is usually better style to use the prefixed name of
    /// an enumeration variant, rather than importing variants.
    ///
    /// ### Known problems
    /// Old-style enumerations that prefix the variants are
    /// still around.
    ///
    /// ### Example
    /// ```no_run
    /// use std::cmp::Ordering::*;
    ///
    /// # fn foo(_: std::cmp::Ordering) {}
    /// foo(Less);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// use std::cmp::Ordering;
    ///
    /// # fn foo(_: Ordering) {}
    /// foo(Ordering::Less)
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub ENUM_GLOB_USE,
    pedantic,
    "use items that import all variants of an enum"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for private wildcard imports `use _::*` from the `std`, `core`, `alloc`,
    /// `proc_macro`, and `test` crates, including reexports of their modules.
    ///
    /// ### Why is this bad?
    /// Wildcard imports from the standard library can break name resolution when a new item is
    /// added with the same name as a locally defined or imported item. This can happen even when
    /// the new standard library item is unstable.
    ///
    /// ### Exceptions
    /// Wildcard imports are allowed from modules whose names contain `prelude`. Many crates
    /// (including the standard library) provide modules named "prelude" specifically designed
    /// for wildcard imports.
    ///
    /// Wildcard imports reexported through `pub use` are also allowed.
    ///
    /// This lint does not check enum variant imports; enable `enum_glob_use` to check those. The
    /// `allowed-wildcard-imports` and `warn-on-all-wildcard-imports` configuration options only
    /// affect `wildcard_imports`. When both lints are enabled, this lint takes precedence over
    /// `wildcard_imports` for eligible standard-library imports.
    ///
    /// ### Known problems
    /// Applying the suggestion when explicit imports of items imported through the glob already
    /// exist may result in `unused_imports` warnings.
    ///
    /// ### Example
    /// ```no_run
    /// use foo::bar;
    /// use std::rc::*;
    ///
    /// # mod foo { pub fn bar() {} }
    /// bar();
    /// let _ = Rc::new(5);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// use foo::bar;
    /// use std::rc::Rc;
    ///
    /// # mod foo { pub fn bar() {} }
    /// bar();
    /// let _ = Rc::new(5);
    /// ```
    #[clippy::version = "1.99.0"]
    pub STD_WILDCARD_IMPORTS,
    pedantic,
    "lint `use _::*` from standard library crates"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for wildcard imports `use _::*`.
    ///
    /// ### Why is this bad?
    /// wildcard imports can pollute the namespace. This is especially bad if
    /// you try to import something through a wildcard, that already has been imported by name from
    /// a different source:
    ///
    /// ```rust,ignore
    /// use crate1::foo; // Imports a function named foo
    /// use crate2::*; // Has a function named foo
    ///
    /// foo(); // Calls crate1::foo
    /// ```
    ///
    /// This can lead to confusing error messages at best and to unexpected behavior at worst.
    ///
    /// ### Exceptions
    /// Wildcard imports are allowed from modules that their name contains `prelude`. Many crates
    /// (including the standard library) provide modules named "prelude" specifically designed
    /// for wildcard import.
    ///
    /// Wildcard imports reexported through `pub use` are also allowed.
    ///
    /// `use super::*` is allowed in test modules. This is defined as any module with "test" in the name.
    ///
    /// These exceptions can be disabled using the `warn-on-all-wildcard-imports` configuration flag.
    ///
    /// ### Known problems
    /// If macros are imported through the wildcard, this macro is not included
    /// by the suggestion and has to be added by hand.
    ///
    /// Applying the suggestion when explicit imports of the things imported with a glob import
    /// exist, may result in `unused_imports` warnings.
    ///
    /// ### Example
    /// ```rust,ignore
    /// use crate1::*;
    ///
    /// foo();
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// use crate1::foo;
    ///
    /// foo();
    /// ```
    #[clippy::version = "1.43.0"]
    pub WILDCARD_IMPORTS,
    pedantic,
    "lint `use _::*` statements"
}

impl_lint_pass!(WildcardImports => [
    ENUM_GLOB_USE,
    STD_WILDCARD_IMPORTS,
    WILDCARD_IMPORTS,
]);

pub struct WildcardImports {
    warn_on_all: bool,
    allowed_segments: &'static FxHashSet<String>,
}

impl WildcardImports {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            warn_on_all: conf.warn_on_all_wildcard_imports,
            allowed_segments: &conf.allowed_wildcard_imports,
        }
    }
}

impl LateLintPass<'_> for WildcardImports {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if cx.sess().is_test_crate() || item.span.in_external_macro(cx.sess().source_map()) {
            return;
        }

        let module = cx.tcx.parent_module_from_def_id(item.owner_id.def_id);
        if let ItemKind::Use(use_path, UseKind::Glob) = &item.kind
            && let Some(used_imports) = cx.tcx.resolutions(()).glob_map.get(&item.owner_id.def_id)
            && !used_imports.is_empty() // Already handled by `unused_imports`
            && !used_imports.contains(&kw::Underscore)
        {
            let is_private = cx.tcx.local_visibility(item.owner_id.def_id) == ty::Visibility::Restricted(module);
            let is_enum_import = matches!(use_path.res.value_ns, Some(Res::Def(DefKind::Enum, _)));
            let wildcard_imports_applies =
                || self.warn_on_all || (is_private && !self.check_exceptions(cx, item, use_path.segments));

            let (lint, message): (&'static Lint, DiagMessage) = if is_enum_import {
                if !wildcard_imports_applies() {
                    return;
                }
                (ENUM_GLOB_USE, "usage of wildcard import for enum variants".into())
            } else if is_private
                && !item.span.from_expansion()
                && !is_prelude_import(use_path.segments)
                && is_std_wildcard_import(cx, use_path)
                && !is_lint_allowed(cx, STD_WILDCARD_IMPORTS, item.hir_id())
            {
                let wildcard_imports_level = cx.tcx.lint_level_spec_at_node(WILDCARD_IMPORTS, item.hir_id());
                if wildcard_imports_level.is_expect()
                    && wildcard_imports_applies()
                    && let Some(expectation) = wildcard_imports_level.lint_id()
                {
                    // The more focused lint replaces `WILDCARD_IMPORTS`, but expectations of the
                    // latter still need to be fulfilled as it would otherwise have been emitted.
                    cx.fulfill_expectation(expectation);
                }
                (
                    STD_WILDCARD_IMPORTS,
                    "usage of wildcard import from the standard library".into(),
                )
            } else {
                if !wildcard_imports_applies() {
                    return;
                }
                (WILDCARD_IMPORTS, "usage of wildcard import".into())
            };

            let mut applicability = Applicability::MachineApplicable;
            let import_source_snippet = snippet_with_applicability(cx, use_path.span, "..", &mut applicability);
            let braced_glob = import_source_snippet.is_empty();
            let span = whole_glob_import_span(cx, use_path, item.span.hi(), braced_glob);
            let sugg = sugg_glob_import(&import_source_snippet, used_imports, braced_glob);

            span_lint_and_sugg(cx, lint, span, message, "try", sugg, applicability);
        }
    }
}

impl WildcardImports {
    fn check_exceptions(&self, cx: &LateContext<'_>, item: &Item<'_>, segments: &[PathSegment<'_>]) -> bool {
        item.span.from_expansion()
            || is_prelude_import(segments)
            || is_allowed_via_config(segments, self.allowed_segments)
            || (is_super_only_import(segments) && is_in_test(cx.tcx, item.hir_id()))
    }
}

// Allow "...prelude::..::*" imports.
// Many crates have a prelude, and it is imported as a glob by design.
fn is_prelude_import(segments: &[PathSegment<'_>]) -> bool {
    segments.iter().any(|ps| ps.ident.as_str().contains("prelude"))
}

// Allow "super::*" imports in tests.
fn is_super_only_import(segments: &[PathSegment<'_>]) -> bool {
    segments.len() == 1 && segments[0].ident.name == kw::Super
}

fn is_std_wildcard_import(cx: &LateContext<'_>, use_path: &Path<'_, PerNS<Option<Res>>>) -> bool {
    let Some(Res::Def(DefKind::Mod, def_id)) = use_path.res.type_ns else {
        return false;
    };
    if def_id.is_local() {
        return false;
    }

    // Crate names are not reserved, so require the resolved crate to come from the sysroot too.
    let crate_name = cx.tcx.crate_name(def_id.krate);
    (STDLIB_STABLE_CRATES.contains(&crate_name) || crate_name == sym::test)
        && cx
            .tcx
            .used_crate_source(def_id.krate)
            .paths()
            .any(|path| path.starts_with(cx.sess().opts.sysroot.path()))
}

// Allow skipping imports containing user configured segments,
// i.e. "...::utils::...::*" if user put `allowed-wildcard-imports = ["utils"]` in `Clippy.toml`
fn is_allowed_via_config(segments: &[PathSegment<'_>], allowed_segments: &FxHashSet<String>) -> bool {
    // segment matching need to be exact instead of using 'contains', in case user unintentionally put
    // a single character in the config thus skipping most of the warnings.
    segments.iter().any(|seg| allowed_segments.contains(seg.ident.as_str()))
}

/// Returns the entire span for a glob import statement, including the `*` symbol.
fn whole_glob_import_span(
    cx: &LateContext<'_>,
    use_path: &Path<'_, PerNS<Option<Res>>>,
    item_hi: BytePos,
    braced_glob: bool,
) -> Span {
    if braced_glob {
        // This is a `_::{_, *}` import. In this case `use_path.span` is empty and ends directly
        // in front of the `*`, so we need to extend it by one byte.
        use_path.span.with_hi(use_path.span.hi() + BytePos(1))
    } else {
        // In this case, the `use_path.span` ends right before the `::*`, so we need to extend it
        // up to the `*`. Since it is hard to find the `*` in weird formatting like
        // `use _ ::  *;`, we extend it up to, but not including the `;`. In nested imports, like
        // `use _::{inner::*, _}` there is no `;` and we can just use the end of the item span.
        let mut span = use_path.span.with_hi(item_hi);
        if snippet(cx, span, "").ends_with(';') {
            span = use_path.span.with_hi(item_hi - BytePos(1));
        }
        span
    }
}

/// Generates a replacement for a glob import using only the names that are actually used.
fn sugg_glob_import(import_source_snippet: &str, used_imports: &FxIndexSet<Symbol>, braced_glob: bool) -> String {
    let mut imports: Vec<_> = used_imports.iter().map(ToString::to_string).collect();
    let imports_string = if imports.len() == 1 {
        imports.pop().unwrap()
    } else if braced_glob {
        imports.join(", ")
    } else {
        format!("{{{}}}", imports.join(", "))
    };

    if braced_glob {
        imports_string
    } else {
        format!("{import_source_snippet}::{imports_string}")
    }
}
