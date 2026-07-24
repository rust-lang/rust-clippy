use rustc_hir::{Expr, ExprKind, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// This lint triggers if a pointer to a Rust `str` is passed into an `extern "C"` interface
    /// where you should instead be providing a pointer to a `CString`.
    ///
    /// ### Why is this bad?
    ///
    /// Foreign functions under the C ABI expect that a string ends with a null byte (`'\0'`).
    /// Rust's `str` doesn't provide a null byte.  Instead it contains a length for the string.
    /// Without a null byte, foreign functions can read beyond the memory allocated to the string searching for the null terminator, causing undefined behavior (UB).
    ///
    /// ### Example
    /// ```no_run
    /// # unsafe extern "C" fn strlen(s: *const i8) -> usize { unimplemented!() }
    /// unsafe { strlen("Hello".as_ptr() as *const _) };
    /// ```
    /// Use instead:
    /// ```no_run
    /// # unsafe extern "C" fn strlen(s: *const i8) -> usize { unimplemented!() }
    /// let cstring = std::ffi::CString::new("Hello".as_bytes()).unwrap();
    /// unsafe { strlen(cstring.as_ptr()) };
    /// ```
    #[clippy::version = "1.99.0"]
    pub STR_PTR_IN_C_ABI,
    suspicious,
    "discourage str pointers in C ABI fns"
}

declare_lint_pass!(StrPtrInCAbi => [STR_PTR_IN_C_ABI]);

impl<'tcx> LateLintPass<'tcx> for StrPtrInCAbi {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // find call expressions
        if let ExprKind::Call(callee, args) = expr.kind
        // where the signature of the callee shows it is an `extern "C" fn`
            && is_extern_c_fn(cx, callee)
        // and the fn is given a str pointer as argument
            && let spans = args
                .iter()
                .filter(|arg| is_str_ptr(cx, arg))
                .map(|arg| arg.span)
                .collect::<Vec<_>>()
            && !spans.is_empty()
        {
            span_lint_and_help(
                cx,
                STR_PTR_IN_C_ABI,
                spans,
                "giving a pointer to a Rust `str` to an `extern \"C\" fn` can cause undefined behavior",
                /* help_span */ None,
                "if the foreign function calls for a null-terminated string in this position, first convert this value to a `std::ffi::CString` and take the pointer from that",
            );
        }
    }
}

/// Does the expression represent an `extern "C" fn` of some type?
fn is_extern_c_fn<'tcx>(cx: &LateContext<'tcx>, callee: &'tcx Expr<'tcx>) -> bool {
    let callee_ty = cx.typeck_results().expr_ty_adjusted(callee);
    if !(callee_ty.is_fn() || callee_ty.is_fn_ptr()) {
        return false;
    }
    // NOTE: fn_sig panics if callee_ty isn't a function, but the above return ensures that it is
    matches!(callee_ty.fn_sig(cx.tcx).abi(), rustc_abi::ExternAbi::C { .. })
}

/// Is `arg` shaped like `derefs_to_str.as_ptr() as *const _`, maybe without the cast?
fn is_str_ptr<'tcx>(cx: &LateContext<'tcx>, arg: &Expr<'tcx>) -> bool {
    let typeck = cx.typeck_results();
    match arg.kind {
        // NOTE: recursion is bounded by how many nested casts to ptr the user does
        ExprKind::Cast(expr, ty) if matches!(ty.kind, TyKind::Ptr(_)) => is_str_ptr(cx, expr),
        ExprKind::MethodCall(method, this, _args, _span) => {
            matches!(method.ident.name, sym::as_ptr | sym::as_mut_ptr)
                && typeck.expr_ty_adjusted(this).peel_refs().is_str()
        },
        _ => false,
    }
}
