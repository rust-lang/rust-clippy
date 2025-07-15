use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::is_type_lang_item;
use rustc_errors::Applicability;
use rustc_hir::{Expr, LangItem};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol};

declare_clippy_lint! {
   /// ### What it does
   /// It detects useless calls to `str::as_bytes()` before calling `len()` or `is_empty()`.
   ///
   /// ### Why is this bad?
   /// The `len()` and `is_empty()` methods are also directly available on strings, and they
   /// return identical results. In particular, `len()` on a string returns the number of
   /// bytes.
   ///
   /// ### Example
   /// ```
   /// let len = "some string".as_bytes().len();
   /// let b = "some string".as_bytes().is_empty();
   /// ```
   /// Use instead:
   /// ```
   /// let len = "some string".len();
   /// let b = "some string".is_empty();
   /// ```
   #[clippy::version = "1.84.0"]
   pub NEEDLESS_AS_BYTES,
   complexity,
   "detect useless calls to `as_bytes()`"
}

pub fn check(cx: &LateContext<'_>, prev_method: Symbol, method: Symbol, prev_recv: &Expr<'_>, span: Span) {
    let ty1 = cx.typeck_results().expr_ty_adjusted(prev_recv).peel_refs();
    if is_type_lang_item(cx, ty1, LangItem::String) || ty1.is_str() {
        let mut app = Applicability::MachineApplicable;
        let sugg = Sugg::hir_with_context(cx, prev_recv, span.ctxt(), "..", &mut app);
        span_lint_and_sugg(
            cx,
            NEEDLESS_AS_BYTES,
            span,
            format!("needless call to `{prev_method}`"),
            format!("`{method}()` can be called directly on strings"),
            format!("{sugg}.{method}()"),
            app,
        );
    }
}
