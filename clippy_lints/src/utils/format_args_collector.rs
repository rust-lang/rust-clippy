use clippy_utils::macros::FormatArgsStorage;
use clippy_utils::source::SpanExt;
use rustc_ast::{Crate, Expr, ExprKind, FormatArgs};
use rustc_data_structures::fx::FxHashMap;
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_span::source_map::SourceMap;
use std::mem;

/// Populates [`FormatArgsStorage`] with AST [`FormatArgs`] nodes
pub struct FormatArgsCollector {
    format_args: FxHashMap<Span, FormatArgs>,
    storage: FormatArgsStorage,
}

impl FormatArgsCollector {
    pub fn new(storage: FormatArgsStorage) -> Self {
        Self {
            format_args: FxHashMap::default(),
            storage,
        }
    }
}

impl_lint_pass!(FormatArgsCollector => []);

impl EarlyLintPass for FormatArgsCollector {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if let ExprKind::FormatArgs(args) = &expr.kind {
            let sm = cx.sess().source_map();
            if args.span.in_external_macro(sm) || has_span_from_proc_macro(sm, args) {
                return;
            }

            self.format_args.insert(expr.span.with_parent(None), (**args).clone());
        }
    }

    fn check_crate_post(&mut self, _: &EarlyContext<'_>, _: &Crate) {
        self.storage.set(mem::take(&mut self.format_args));
    }
}

/// Detects if the format string or an argument has its span set by a proc macro to something inside
/// a macro callsite, e.g.
///
/// ```ignore
/// println!(some_proc_macro!("input {}"), a);
/// ```
///
/// Where `some_proc_macro` expands to
///
/// ```ignore
/// println!("output {}", a);
/// ```
///
/// But with the span of `"output {}"` set to the macro input
///
/// ```ignore
/// println!(some_proc_macro!("input {}"), a);
/// //                        ^^^^^^^^^^
/// ```
fn has_span_from_proc_macro(sm: &SourceMap, args: &FormatArgs) -> bool {
    let args_sp = args.span.data();
    let Some((scx, args_range)) = args_sp.mk_edit_cx(sm) else {
        return true;
    };

    // Check the spans between the format string and the arguments and between each argument.
    args.arguments
        .explicit_args()
        .iter()
        .try_fold(args_range.end, |start, arg| {
            let range = scx.span_to_file_range(arg.expr.span.walk_to_parent(args_sp.ctxt)?);
            let mut tks = tokenize(scx.get_text(start..range.start)?, FrontmatterAllowed::No)
                .map(|x| x.kind)
                .filter(|x| {
                    !matches!(
                        x,
                        TokenKind::LineComment { doc_style: None }
                            | TokenKind::BlockComment {
                                doc_style: None,
                                terminated: true
                            }
                            | TokenKind::Whitespace
                    )
                });

            // `,` or `, ident =`
            let matches = matches!(tks.next(), Some(TokenKind::Comma))
                && match tks.next() {
                    Some(TokenKind::Ident) => matches!(tks.next(), Some(TokenKind::Eq)),
                    Some(_) => false,
                    None => true,
                }
                && tks.next().is_none();
            matches.then_some(range.end)
        })
        .is_none()
}
