use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::SpanRangeExt;
use rustc_errors::Applicability;
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;
use std::iter;

declare_clippy_lint! {
    /// ### What it does
    /// Detects passing a mutable reference to a function that only
    /// requires an immutable reference.
    ///
    /// ### Why is this bad?
    /// The mutable reference rules out all other references to
    /// the value. Also the code misleads about the intent of the call site.
    ///
    /// ### Example
    /// ```no_run
    /// # let mut vec = Vec::new();
    /// # let mut value = 5;
    /// vec.push(&mut value);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # let mut vec = Vec::new();
    /// # let value = 5;
    /// vec.push(&value);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub UNNECESSARY_MUT_PASSED,
    style,
    "an argument passed as a mutable reference although the callee only demands an immutable reference"
}

declare_lint_pass!(UnnecessaryMutPassed => [UNNECESSARY_MUT_PASSED]);

impl<'tcx> LateLintPass<'tcx> for UnnecessaryMutPassed {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if e.span.from_expansion() {
            // Issue #11268
            return;
        }

        match e.kind {
            ExprKind::Call(fn_expr, arguments) => {
                if let ExprKind::Path(ref path) = fn_expr.kind {
                    check_arguments(
                        cx,
                        &mut arguments.iter(),
                        cx.typeck_results().expr_ty(fn_expr),
                        &rustc_hir_pretty::qpath_to_string(&cx.tcx, path),
                        "function",
                    );
                }
            },
            ExprKind::MethodCall(path, receiver, arguments, _)
                if let Some(def_id) = cx.typeck_results().type_dependent_def_id(e.hir_id) =>
            {
                let args = cx.typeck_results().node_args(e.hir_id);
                let method_type = cx.tcx.type_of(def_id).instantiate(cx.tcx, args);
                check_arguments(
                    cx,
                    &mut iter::once(receiver).chain(arguments.iter()),
                    method_type,
                    path.ident.as_str(),
                    "method",
                );
            },
            _ => (),
        }
    }
}

fn check_arguments<'tcx>(
    cx: &LateContext<'tcx>,
    arguments: &mut dyn Iterator<Item = &'tcx Expr<'tcx>>,
    type_definition: Ty<'tcx>,
    name: &str,
    fn_kind: &str,
) {
    if type_definition.is_fn() {
        let parameters = type_definition.fn_sig(cx.tcx).skip_binder().inputs();
        for (argument, parameter) in iter::zip(arguments, parameters) {
            if let ty::Ref(_, _, Mutability::Not) | ty::RawPtr(_, Mutability::Not) = parameter.kind()
                && let ExprKind::AddrOf(borrow_kind, Mutability::Mut, arg) = argument.kind
            {
                emit(cx, name, fn_kind, argument, borrow_kind, arg);
            }
        }
    }
}

fn emit(cx: &LateContext<'_>, name: &str, fn_kind: &str, argument: &Expr<'_>, borrow_kind: BorrowKind, arg: &Expr<'_>) {
    let applicability = Applicability::MachineApplicable;

    span_lint_and_then(
        cx,
        UNNECESSARY_MUT_PASSED,
        argument.span,
        format!("the {fn_kind} `{name}` doesn't need a mutable reference"),
        |diag| {
            let span_until_arg = argument.span.until(arg.span);
            match borrow_kind {
                BorrowKind::Ref => {
                    let span_to_remove = if let Some(Some(ref_pos)) = span_until_arg.with_source_text(cx, |src| {
                        src
                            // we don't use `strip_prefix` here, because `argument` might be enclosed in
                            // parens, in which case `&` is no longer the prefix
                            .find('&')
                            // just a sanity check, in case some proc-macro messes up the spans
                            .filter(|ref_pos| src[*ref_pos..].contains("mut"))
                    }) && let Ok(lo) = u32::try_from(ref_pos + '&'.len_utf8())
                    {
                        span_until_arg.split_at(lo).1
                    } else {
                        return;
                    };
                    diag.span_suggestion_verbose(span_to_remove, "remove this `mut`", String::new(), applicability);
                },
                BorrowKind::Raw => {
                    let span_to_remove =
                        if let Some(Some(ref_pos)) = span_until_arg.with_source_text(cx, |src: &str| {
                            // we don't use `strip_prefix` here, because `argument` might be enclosed in
                            // parens, and there might be arbitrary whitespace between things
                            let src_after_addr_raw = src.split_once('&')?.1.split_once("raw")?.1.trim_start();

                            Some(src_after_addr_raw)
                                // just a sanity check, in case some proc-macro messes up the spans
                                .filter(|trimmed| trimmed.contains("mut"))
                                .map(|trimmed| src.len() - trimmed.len())
                        }) && let Ok(lo) = u32::try_from(ref_pos)
                        {
                            span_until_arg.split_at(lo).1
                        } else {
                            return;
                        };
                    diag.span_suggestion_verbose(
                        span_to_remove,
                        "make this a `const` ptr",
                        // the span points at `&raw mut x`
                        //                          ^^^^
                        // so we append a space to our suggestion
                        String::from("const "),
                        applicability,
                    );
                },
                BorrowKind::Pin => {
                    // it's fine to only "check" this after we've emitted the lint -- if the
                    // reference was an `&pin`, passing it into a function requiring a ptr wouldn't
                    // have type-checked in the first place
                },
            }
        },
    );
}
