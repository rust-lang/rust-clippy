use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_from_proc_macro;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl, FnHeader, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_target::spec::abi::Abi;

const LINT_MSG: &str = "`extern` missing explicit ABI";
const LINT_HELP_MSG: &str = "consider using";

const EXTERN: &str = "extern";
const FN: &str = "fn";
const OPEN_BRACE: &str = "{";
const ABI: &str = r#""C""#;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `extern` without an explicit ABI.
    ///
    /// ### Why is this bad?
    /// Explicitly declaring the ABI is the recommended convention. See:
    /// [Rust Style Guide - `extern` items](https://doc.rust-lang.org/nightly/style-guide/items.html#extern-items)
    ///
    /// It's also enforced by `rustfmt` when the `force_explicit_abi` option is enabled. See:
    /// [Configuring Rustfmt](https://rust-lang.github.io/rustfmt/?version=master&search=#force_explicit_abi)
    ///
    /// ### Example
    /// ```no_run
    /// extern fn foo() {}
    ///
    /// extern {
    ///     fn bar();
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// extern "C" fn foo() {}
    ///
    /// extern "C" {
    ///     fn bar();
    /// }
    /// ```
    #[clippy::version = "1.83.0"]
    pub EXTERN_WITHOUT_ABI,
    style,
    "`extern` missing explicit ABI"
}

declare_lint_pass!(ExternWithoutAbi => [EXTERN_WITHOUT_ABI]);

impl<'tcx> LateLintPass<'tcx> for ExternWithoutAbi {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::ForeignMod { abi: Abi::C { .. }, .. } = item.kind
            && !in_external_macro(cx.sess(), item.span)
            && let snippet = snippet(cx.sess(), item.span, "").as_ref()
            && is_extern_followed_by(OPEN_BRACE, snippet)
            && !is_from_proc_macro(cx, item)
        {
            emit_lint(cx, item.span, snippet);
        }
    }

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _: LocalDefId,
    ) {
        if let FnKind::ItemFn(_, _, header) = kind
            && let FnHeader { abi: Abi::C { .. }, .. } = header
            && !in_external_macro(cx.sess(), span)
            && let snippet = snippet(cx.sess(), span, "").as_ref()
            && is_extern_followed_by(FN, snippet)
            && let hir_id = cx.tcx.hir().body_owner(body.id())
            && !is_from_proc_macro(cx, &(&kind, body, hir_id, span))
        {
            emit_lint(cx, span, snippet);
        }
    }
}

fn is_extern_followed_by(item: &str, snippet: &str) -> bool {
    let mut tokens = snippet.split_whitespace();

    if let (Some(EXTERN), Some(i)) = (tokens.next(), tokens.next())
        && i.starts_with(item)
    {
        return true;
    }
    false
}

fn emit_lint(cx: &LateContext<'_>, span: Span, snippet: &str) {
    let sugg = snippet.replacen(EXTERN, format!("{EXTERN} {ABI}").as_str(), 1);
    span_lint_and_sugg(
        cx,
        EXTERN_WITHOUT_ABI,
        span,
        LINT_MSG,
        LINT_HELP_MSG,
        sugg,
        Applicability::MachineApplicable,
    );
}
