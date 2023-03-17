use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_errors::Applicability;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Find traits that are not explicitely used in scope (like `AsRef::as_ref()`)
    /// but directly with the trait method (like `opt.as_ref()`).
    ///
    /// These traits can be imported anonymously with `use crate::Trait as _`.
    /// This avoids name collision with other traits (possibly with the same name).
    /// It also helps identify the traits in `use` statements.
    ///
    /// ### Why is this bad?
    /// This needlessly brings a trait into the type namespace, where it could
    /// shadow other things. This is not really a problem, this lint is just
    /// for those who like to keep things tidy.
    ///
    /// ### Example
    /// ```rust
    /// use std::io::Read;
    /// fn main() {
    ///   let mut b = "I'm your father!".as_bytes();
    ///   let mut buffer = [0; 10];
    ///   b.read(&mut buffer)
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::io::Read as _;
    /// fn main() {
    ///   let mut b = "I'm your father!".as_bytes();
    ///   let mut buffer = [0; 10];
    ///   b.read(&mut buffer)
    /// }
    /// ```
    #[clippy::version = "1.69.0"]
    pub NEEDLESS_TRAITS_IN_SCOPE,
    restriction,
    "trait is needlessly imported into the type namespace, and can be anonymously imported"
}
declare_lint_pass!(NeedlessTraitsInScope => [NEEDLESS_TRAITS_IN_SCOPE]);

impl<'tcx> LateLintPass<'tcx> for NeedlessTraitsInScope {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Only process `use` statements, ignore `UseKind::Glob`
        let ItemKind::Use(use_path, UseKind::Single) = item.kind else {
            return
        };
        // Check if it's a trait
        if !use_path
            .res
            .iter()
            .any(|res| matches!(res, def::Res::Def(def::DefKind::Trait, _)))
        {
            return;
        }
        // Check if the `use` is aliased with ` as `.
        // If aliased, then do not process, it's probably for a good reason
        if item.ident != use_path.segments.last().unwrap().ident {
            return;
        }
        let path = use_path
            .segments
            .iter()
            .map(|segment| segment.ident)
            .fold(String::new(), |mut acc, ident| {
                if !acc.is_empty() {
                    acc += "::";
                }
                acc += ident.as_str();
                acc
            });
        span_lint_and_sugg(
            cx,
            NEEDLESS_TRAITS_IN_SCOPE,
            use_path.span,
            "trait is needlessly imported into the type namespace",
            "you can import the trait anonymously",
            format!("{path} as _"),
            Applicability::MachineApplicable,
        );
    }
}
