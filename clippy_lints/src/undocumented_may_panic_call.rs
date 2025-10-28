use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint;
use clippy_utils::paths::{PathNS, lookup_path_str};
use clippy_utils::{get_unique_attr, sym};
use rustc_data_structures::fx::FxHashSet;
use rustc_hir as hir;
use rustc_hir::def_id::DefId;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::TyCtxt;
use rustc_session::impl_lint_pass;
use rustc_span::Pos;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to functions marked with `#[clippy::may_panic]` or configured in
    /// `may-panic-functions` that don't have a `// Panic:` comment on the line above.
    ///
    /// ### Why is this bad?
    /// Functions that may panic should be documented at their call sites to explain why the
    /// panic is acceptable or impossible in that context.
    ///
    /// ### Example
    /// ```rust,ignore
    /// #[clippy::may_panic]
    /// fn my_panicable_func(n: u32) {
    ///     if n % 2 == 0 {
    ///         panic!("even number not allowed")
    ///     }
    /// }
    ///
    /// fn main() {
    ///     // Missing documentation - will lint
    ///     my_panicable_func(1);
    /// }
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// #[clippy::may_panic]
    /// fn my_panicable_func(n: u32) {
    ///     if n % 2 == 0 {
    ///         panic!("even number not allowed")
    ///     }
    /// }
    ///
    /// fn main() {
    ///     // Panic: This is safe, it's an odd number
    ///     my_panicable_func(1);
    /// }
    /// ```
    ///
    /// ### Configuration
    /// This lint can be configured to check calls to external functions that may panic:
    /// ```toml
    /// # clippy.toml
    /// may-panic-functions = [
    ///     "alloc::vec::Vec::push",  # Can panic on allocation failure
    ///     "std::fs::File::open",    # Can panic in some configurations
    /// ]
    /// ```
    #[clippy::version = "1.92.0"]
    pub UNDOCUMENTED_MAY_PANIC_CALL,
    pedantic,
    "missing `// Panic:` documentation on calls to functions that may panic"
}

pub struct UndocumentedMayPanicCall {
    may_panic_def_ids: FxHashSet<DefId>,
}

impl_lint_pass!(UndocumentedMayPanicCall => [UNDOCUMENTED_MAY_PANIC_CALL]);

impl UndocumentedMayPanicCall {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf) -> Self {
        let may_panic_def_ids = conf
            .may_panic_functions
            .iter()
            .flat_map(|path| lookup_path_str(tcx, PathNS::Value, path))
            .collect();

        Self { may_panic_def_ids }
    }

    // A function is a may_panic_function if it has the may_panic attribute
    // or is in the may-panic-functions configuration
    fn is_may_panic_function(&self, cx: &LateContext<'_>, def_id: DefId) -> bool {
        if get_unique_attr(cx.sess(), cx.tcx.get_all_attrs(def_id), sym::may_panic).is_some() {
            return true;
        }

        self.may_panic_def_ids.contains(&def_id)
    }
}

impl LateLintPass<'_> for UndocumentedMayPanicCall {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &'_ rustc_hir::Expr<'_>) {
        let def_id = match &expr.kind {
            hir::ExprKind::Call(func, _args) => {
                if let hir::ExprKind::Path(qpath) = &func.kind {
                    cx.qpath_res(qpath, func.hir_id).opt_def_id()
                } else {
                    None
                }
            },
            hir::ExprKind::MethodCall(_path, _receiver, _args, _span) => {
                cx.typeck_results().type_dependent_def_id(expr.hir_id)
            },
            _ => None,
        };

        if let Some(def_id) = def_id
            && self.is_may_panic_function(cx, def_id)
            && !has_panic_comment_above(cx, expr.span)
        {
            span_lint(
                cx,
                UNDOCUMENTED_MAY_PANIC_CALL,
                expr.span,
                "call to a function that may panic is not documented with a `// Panic:` comment",
            );
        }
    }
}

/// Checks if the lines immediately preceding the call contain a "Panic:" comment.
fn has_panic_comment_above(cx: &LateContext<'_>, call_span: rustc_span::Span) -> bool {
    let source_map = cx.sess().source_map();

    if let Ok(call_line) = source_map.lookup_line(call_span.lo())
        && call_line.line > 0
        && let Some(src) = call_line.sf.src.as_deref()
    {
        let lines = call_line.sf.lines();
        let line_starts = &lines[..=call_line.line];

        has_panic_comment_in_text(src, line_starts)
    } else {
        false
    }
}

fn has_panic_comment_in_text(src: &str, line_starts: &[rustc_span::RelativeBytePos]) -> bool {
    let mut lines = line_starts
        .array_windows::<2>()
        .rev()
        .map_while(|[start, end]| {
            let start = start.to_usize();
            let end = end.to_usize();
            let text = src.get(start..end)?;
            let trimmed = text.trim_start();
            Some((trimmed, text.len() - trimmed.len()))
        })
        .filter(|(text, _)| !text.is_empty());

    let Some((line, _)) = lines.next() else {
        return false;
    };

    if line.starts_with("//") {
        let mut current_line = line;
        loop {
            if current_line.to_ascii_uppercase().contains("PANIC:") {
                return true;
            }
            match lines.next() {
                Some((text, _)) if text.starts_with("//") => current_line = text,
                _ => return false,
            }
        }
    }

    false
}
