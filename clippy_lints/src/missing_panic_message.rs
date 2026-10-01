use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::macros::{PanicCall, root_macro_call_first_node};
use clippy_utils::res::MaybeDef as _;
use clippy_utils::{peel_blocks_with_stmt, sym};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_span::hygiene::MacroKind;
use rustc_span::symbol::Symbol;

declare_clippy_lint! {
    /// ### What it does
    /// Checks if `panic!`, `todo!`, `unimplemented!`, and `unreachable!`
    /// have a custom panic message.
    ///
    /// ### Why restrict this?
    /// Custom messages make code more readable and debugging easier.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::panic::panic_any;
    /// panic!();
    /// todo!();
    /// unimplemented!();
    /// unreachable!();
    /// ```
    /// Use instead:
    /// ```no_run
    /// # use std::panic::panic_any;
    /// panic!("why this is bad");
    /// todo!("what to do");
    /// unimplemented!("why this is intentionally unimplemented");
    /// unreachable!("why this is unreachable");
    /// ```
    #[clippy::version = "1.100.0"]
    pub MISSING_PANIC_MESSAGE,
    restriction,
    "panic without a useful message"
}

declare_lint_pass!(MissingPanicMessage => [MISSING_PANIC_MESSAGE]);

impl<'tcx> LateLintPass<'tcx> for MissingPanicMessage {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(macro_call) = root_macro_call_first_node(cx, expr)
            && macro_call.kind == MacroKind::Bang
            && let Some(kind) = macro_call.def_id.opt_diag_name(cx).and_then(PanicMacroKind::new)
            && PanicCall::parse(peel_blocks_with_stmt(expr)).is_some_and(|panic_call| panic_call.is_default_message())
        {
            span_lint_and_help(cx, MISSING_PANIC_MESSAGE, expr.span, kind.message(), None, kind.help());
        }
    }
}

enum PanicMacroKind {
    Panic,
    Todo,
    Unimplemented,
    Unreachable,
}

impl PanicMacroKind {
    fn new(name: Symbol) -> Option<Self> {
        match name {
            sym::core_panic_macro
            | sym::std_panic_macro
            | sym::core_panic_2015_macro
            | sym::std_panic_2015_macro
            | sym::core_panic_2021_macro => Some(Self::Panic),
            sym::todo_macro => Some(Self::Todo),
            sym::unimplemented_macro => Some(Self::Unimplemented),
            sym::unreachable_macro => Some(Self::Unreachable),
            _ => None,
        }
    }

    fn message(&self) -> String {
        let kind = match self {
            PanicMacroKind::Panic => "panic!",
            PanicMacroKind::Todo => "todo!",
            PanicMacroKind::Unimplemented => "unimplemented!",
            PanicMacroKind::Unreachable => "unreachable!",
        };
        format!("using `{kind}` macro without a custom message")
    }

    fn help(&self) -> String {
        let help = match self {
            PanicMacroKind::Panic => "why this is bad",
            PanicMacroKind::Todo => "what to do",
            PanicMacroKind::Unimplemented => "why this is intentionally unimplemented",
            PanicMacroKind::Unreachable => "why this is unreachable",
        };
        format!("consider describing {help}")
    }
}
