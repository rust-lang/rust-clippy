use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeTypeckRes as _};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::implements_trait;
use clippy_utils::{path_to_local_with_projections, sym};
use rustc_ast::{BindingMode, Mutability};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Node, PatKind};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol};

use super::FILTER_NEXT;

#[derive(Clone, Copy)]
pub(super) enum Direction {
    Forward,
    Backward,
}

/// An iterator method sitting between `filter(..)` and `next()` which has an `Option` method of
/// the same name that behaves the same on the single item `find` returns, e.g. `map(..)`.
struct Adapter {
    name: Symbol,
    has_args: bool,
    /// Source of the adapter call, starting right after `filter(..)`, e.g. `.map(|x| x + 1)`
    span: Span,
}

impl Adapter {
    fn call(&self) -> String {
        let dots = if self.has_args { ".." } else { "" };
        format!("{}({dots})", self.name)
    }
}

/// Whether `call` resolves to a method of the diagnostic item `trait_name`, and not to an unrelated
/// method with the same name, e.g. an inherent one.
fn is_trait_method(cx: &LateContext<'_>, call: &Expr<'_>, trait_name: Symbol) -> bool {
    cx.ty_based_def(call).opt_parent(cx).is_diag_item(cx, trait_name)
}

/// lint use of `filter().next()` for `Iterator` and `filter().next_back()` for
/// `DoubleEndedIterator`
///
/// `filter_call` is the `filter` call, with `recv` and `filter_arg` being its receiver and argument.
pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    filter_call: &Expr<'_>,
    recv: &Expr<'_>,
    filter_arg: &Expr<'_>,
    direction: Direction,
) {
    lint(cx, expr, filter_call, recv, filter_arg, direction, None);
}

/// lint use of `filter().<adapter>().next()` for `Iterator` and `filter().<adapter>().next_back()`
/// for `DoubleEndedIterator`, where `<adapter>` is called on the first item `find` returns instead.
///
/// `adapter_call` is the `<adapter>` call and `filter_call` the `filter` call it is called on, with
/// `recv` and `filter_arg` being the receiver and argument of that `filter` call.
/// The caller decides which adapters are handled, they have to behave the same when applied to
/// the first item of the filtered iterator and when applied to the `Option` returned by `find`.
pub(super) fn check_adapter(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    adapter_call: &Expr<'_>,
    filter_call: &Expr<'_>,
    recv: &Expr<'_>,
    filter_arg: &Expr<'_>,
    direction: Direction,
) {
    if let ExprKind::MethodCall(adapter_path, _, adapter_args, _) = adapter_call.kind
        && is_trait_method(cx, adapter_call, sym::Iterator)
    {
        let adapter = Adapter {
            name: adapter_path.ident.name,
            has_args: !adapter_args.is_empty(),
            span: adapter_call.span.with_lo(filter_call.span.hi()),
        };
        lint(cx, expr, filter_call, recv, filter_arg, direction, Some(&adapter));
    }
}

fn lint(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    filter_call: &Expr<'_>,
    recv: &Expr<'_>,
    filter_arg: &Expr<'_>,
    direction: Direction,
    adapter: Option<&Adapter>,
) {
    let (required_trait, next_method, find_method) = match direction {
        Direction::Forward => (sym::Iterator, "next", "find"),
        Direction::Backward => (sym::DoubleEndedIterator, "next_back", "rfind"),
    };
    if !is_trait_method(cx, filter_call, sym::Iterator) || !is_trait_method(cx, expr, required_trait) {
        return;
    }
    if !cx
        .tcx
        .get_diagnostic_item(required_trait)
        .is_some_and(|id| implements_trait(cx, cx.typeck_results().expr_ty(recv), id, &[]))
    {
        return;
    }
    let (called, instead) = match adapter {
        Some(adapter) => (
            format!("filter(..).{}.{next_method}()", adapter.call()),
            format!(".{find_method}(..).{}", adapter.call()),
        ),
        None => (format!("filter(..).{next_method}()"), format!(".{find_method}(..)")),
    };
    span_lint_and_then(
        cx,
        FILTER_NEXT,
        expr.span,
        format!("called `{called}` on an `{required_trait}`"),
        |diag| {
            let mut app = Applicability::MachineApplicable;
            let filter_snippet = snippet_with_applicability(cx, filter_arg.span, "..", &mut app);
            let iter_snippet = snippet_with_applicability(cx, recv.span, "..", &mut app);
            let adapter_snippet = adapter.map_or_else(String::new, |a| {
                snippet_with_applicability(cx, a.span, "..", &mut app).into_owned()
            });

            let pat = if let Some(id) = path_to_local_with_projections(recv)
                && let Node::Pat(pat) = cx.tcx.hir_node(id)
                && let PatKind::Binding(BindingMode(_, Mutability::Not), _, ident, _) = pat.kind
            {
                app = Applicability::Unspecified;
                Some((pat.span, ident))
            } else {
                None
            };

            diag.span_suggestion_verbose(
                expr.span,
                format!("use `{instead}` instead"),
                format!("{iter_snippet}.{find_method}({filter_snippet}){adapter_snippet}"),
                app,
            );

            if let Some((pat_span, ident)) = pat {
                diag.span_help(
                    pat_span,
                    format!("you will also need to make `{ident}` mutable, because `{find_method}` takes `&mut self`"),
                );
            }
        },
    );
}
