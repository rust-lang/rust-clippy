use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{is_prelude_import, sugg_glob_import, whole_glob_import_span};
use rustc_errors::Applicability;
use rustc_hir::{Item, ItemKind, PathSegment, UseKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;
use rustc_span::symbol::{STDLIB_STABLE_CRATES, kw};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for wildcard imports `use _::*` from the standard library crates.
    ///
    /// ### Why is this bad?
    /// Wildcard imports can pollute the namespace. This is especially bad when importing from the
    /// standard library through wildcards:
    ///
    /// ```no_run
    /// use foo::bar; // Imports a function named bar
    /// use std::rc::*; // Does not have a function named bar initially
    ///
    /// # mod foo { pub fn bar() {} }
    /// bar();
    /// ```
    ///
    /// When the `std::rc` module later adds a function named `bar`, the compiler cannot decide
    /// which function to call, causing a compilation error.
    ///
    /// ### Exceptions
    /// Wildcard imports are allowed from modules whose names contain `prelude`. Many crates
    /// (including the standard library) provide modules named "prelude" specifically designed
    /// for wildcard import.
    ///
    /// ### Example
    /// ```no_run
    /// use std::rc::*;
    ///
    /// let _ = Rc::new(5);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// use std::rc::Rc;
    ///
    /// let _ = Rc::new(5);
    /// ```
    #[clippy::version = "1.89.0"]
    pub STD_WILDCARD_IMPORTS,
    style,
    "lint `use _::*` from the standard library crates"
}

declare_lint_pass!(StdWildcardImports => [STD_WILDCARD_IMPORTS]);

impl LateLintPass<'_> for StdWildcardImports {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Use(use_path, UseKind::Glob) = item.kind
            && !is_prelude_import(use_path.segments)
            && is_std_import(use_path.segments)
            && let used_imports = cx.tcx.names_imported_by_glob_use(item.owner_id.def_id)
            && !used_imports.is_empty() // Already handled by `unused_imports`
            && !used_imports.contains(&kw::Underscore)
        {
            let mut applicability = Applicability::MachineApplicable;
            let import_source_snippet = snippet_with_applicability(cx, use_path.span, "..", &mut applicability);

            let span = whole_glob_import_span(cx, item, import_source_snippet.is_empty())
                .expect("Not a glob import statement");
            let sugg = sugg_glob_import(&import_source_snippet, used_imports);

            span_lint_and_sugg(
                cx,
                STD_WILDCARD_IMPORTS,
                span,
                "usage of wildcard import from `std` crates",
                "try",
                sugg,
                applicability,
            );
        }
    }
}

// Checks for the standard libraries, including `test` crate.
fn is_std_import(segments: &[PathSegment<'_>]) -> bool {
    let Some(first_segment_name) = segments.first().map(|ps| ps.ident.name) else {
        return false;
    };

    STDLIB_STABLE_CRATES.contains(&first_segment_name) || first_segment_name == sym::test
}
