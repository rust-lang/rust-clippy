use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_in_test;
use clippy_utils::source::{snippet, snippet_with_applicability};
use rustc_data_structures::fx::{FxHashSet, FxIndexSet};
use rustc_errors::Applicability;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Item, ItemKind, PathSegment, UseKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;
use rustc_span::symbol::{STDLIB_STABLE_CRATES, Symbol, kw};
use rustc_span::{BytePos, Span, sym};

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

declare_clippy_lint! {
    /// ### What it does
    /// Checks for wildcard imports `use _::*` from the standard library crates.
    ///
    /// ### Why is this bad?
    /// Wildcard imports from the standard library crates can lead to breakages due to name
    /// resolution ambiguities when the standard library introduces new items with the same names
    /// as locally defined items.
    ///
    /// ### Exceptions
    /// Wildcard imports are allowed from modules whose names contain `prelude`. Many crates
    /// (including the standard library) provide modules named "prelude" specifically designed
    /// for wildcard imports.
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
    #[clippy::version = "1.89.0"]
    pub STD_WILDCARD_IMPORTS,
    pedantic,
    "lint `use _::*` from the standard library crates"
}

pub struct WildcardImports {
    warn_on_all: bool,
    allowed_segments: FxHashSet<String>,
}

impl WildcardImports {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            warn_on_all: conf.warn_on_all_wildcard_imports,
            allowed_segments: conf.allowed_wildcard_imports.iter().cloned().collect(),
        }
    }
}

impl_lint_pass!(WildcardImports => [ENUM_GLOB_USE, WILDCARD_IMPORTS, STD_WILDCARD_IMPORTS]);

impl LateLintPass<'_> for WildcardImports {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if cx.sess().is_test_crate() {
            return;
        }

        let module = cx.tcx.parent_module_from_def_id(item.owner_id.def_id);
        if cx.tcx.visibility(item.owner_id.def_id) != ty::Visibility::Restricted(module.to_def_id())
            && !self.warn_on_all
        {
            return;
        }
        if let ItemKind::Use(use_path, UseKind::Glob) = &item.kind
            && (self.warn_on_all
                || !self.check_exceptions(cx, item, use_path.segments)
                || (!is_prelude_import(use_path.segments) && is_std_import(use_path.segments)))
            && let used_imports = cx.tcx.names_imported_by_glob_use(item.owner_id.def_id)
            && !used_imports.is_empty() // Already handled by `unused_imports`
            && !used_imports.contains(&kw::Underscore)
        {
            let mut applicability = Applicability::MachineApplicable;
            let import_source_snippet = snippet_with_applicability(cx, use_path.span, "..", &mut applicability);

            let span = whole_glob_import_span(cx, item, import_source_snippet.is_empty())
                .expect("Not a glob import statement");
            let sugg = sugg_glob_import(&import_source_snippet, used_imports);

            // Glob imports always have a single resolution. Enums are in the value namespace.
            let (lint, message) = if let Some(Res::Def(DefKind::Enum, _)) = use_path.res.value_ns {
                (
                    ENUM_GLOB_USE,
                    String::from("usage of wildcard import for enum variants"),
                )
            } else if is_std_import(use_path.segments) {
                (
                    STD_WILDCARD_IMPORTS,
                    format!("usage of wildcard import from `{}`", use_path.segments[0].ident),
                )
            } else {
                (WILDCARD_IMPORTS, String::from("usage of wildcard import"))
            };

            span_lint_and_sugg(cx, lint, span, message, "try", sugg, applicability);
        }
    }
}

impl WildcardImports {
    fn check_exceptions(&self, cx: &LateContext<'_>, item: &Item<'_>, segments: &[PathSegment<'_>]) -> bool {
        item.span.from_expansion()
            || is_prelude_import(segments)
            || is_allowed_via_config(segments, &self.allowed_segments)
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

// Checks for the standard libraries, including `test` crate.
fn is_std_import(segments: &[PathSegment<'_>]) -> bool {
    let Some(first_segment_name) = segments.first().map(|ps| ps.ident.name) else {
        return false;
    };

    STDLIB_STABLE_CRATES.contains(&first_segment_name) || first_segment_name == sym::test
}

// Allow skipping imports containing user configured segments,
// i.e. "...::utils::...::*" if user put `allowed-wildcard-imports = ["utils"]` in `Clippy.toml`
fn is_allowed_via_config(segments: &[PathSegment<'_>], allowed_segments: &FxHashSet<String>) -> bool {
    // segment matching need to be exact instead of using 'contains', in case user unintentionally put
    // a single character in the config thus skipping most of the warnings.
    segments.iter().any(|seg| allowed_segments.contains(seg.ident.as_str()))
}

// Returns the entire span for a given glob import statement, including the `*` symbol.
fn whole_glob_import_span(cx: &LateContext<'_>, item: &Item<'_>, braced_glob: bool) -> Option<Span> {
    let ItemKind::Use(use_path, UseKind::Glob) = item.kind else {
        return None;
    };

    if braced_glob {
        // This is a `_::{_, *}` import
        // In this case `use_path.span` is empty and ends directly in front of the `*`,
        // so we need to extend it by one byte.
        Some(use_path.span.with_hi(use_path.span.hi() + BytePos(1)))
    } else {
        // In this case, the `use_path.span` ends right before the `::*`, so we need to
        // extend it up to the `*`. Since it is hard to find the `*` in weird
        // formatting like `use _ ::  *;`, we extend it up to, but not including the
        // `;`. In nested imports, like `use _::{inner::*, _}` there is no `;` and we
        // can just use the end of the item span
        let mut span = use_path.span.with_hi(item.span.hi());
        if snippet(cx, span, "").ends_with(';') {
            span = use_path.span.with_hi(item.span.hi() - BytePos(1));
        }
        Some(span)
    }
}

// Generates a suggestion for a glob import using only the actually used items.
fn sugg_glob_import(import_source_snippet: &str, used_imports: &FxIndexSet<Symbol>) -> String {
    let mut imports: Vec<_> = used_imports.iter().map(ToString::to_string).collect();
    let imports_string = if imports.len() == 1 {
        imports.pop().unwrap()
    } else if import_source_snippet.is_empty() {
        imports.join(", ")
    } else {
        format!("{{{}}}", imports.join(", "))
    };

    if import_source_snippet.is_empty() {
        imports_string
    } else {
        format!("{import_source_snippet}::{imports_string}")
    }
}
