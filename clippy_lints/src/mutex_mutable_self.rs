#![allow(rustc::usage_of_ty_tykind)]

//! Suggest `get_mut` instead of `lock` on `Mutex` when the mutex is in scope as a mutable variable
//! (currently doesn't properly work when the Mutex is owned by the scope calling `lock`)

use if_chain::if_chain;

use rustc::{declare_lint_pass, declare_tool_lint};
use rustc::hir::*;
use rustc::lint::{LateContext, LateLintPass, LintPass, LintArray};
use rustc::ty::TyKind;

use crate::utils::{match_type, span_help_and_lint, paths};

declare_clippy_lint! {
    /// **What it does**: Checks for unnecessary calls to `Mutex::lock` when `Mutex::get_mut` would suffice.
    ///
    /// **Why is this bad?** Calling `Mutex::get_mut` only needs to access the inner value,
    /// as it's statically guaranteed that no other thread is concurrently accessing it.
    /// `Mutex::lock` is much more expensive.
    ///
    /// **Known problems**:
    ///   * doesn't correctly detect the case where the Mutex is owned by this scope,
    ///     so only warns for &mut self
    ///
    /// **Example**:
    /// // TODO, see tests/ui/mutex_mutable_self.rs
    ///
    pub MUTEX_MUTABLE_SELF,
    style,
    "usage of `Mutex::lock` when `Mutex::get_mut` would suffice (i.e. self is a mutable ref)"
}

declare_lint_pass!(MutexMutableSelf => [MUTEX_MUTABLE_SELF]);


impl<'a, 'tcx> LateLintPass<'a, 'tcx> for MutexMutableSelf {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        if_chain! {
            if let ExprKind::MethodCall(ref method_name, _, ref args) = expr.kind;
            if method_name.ident.name == sym!(lock);
            if args.len() == 1; // mutex.lock() has no params, just the receiver
            let type_of_receiver = cx.tables.expr_ty(&args[0]);

            then {
                match type_of_receiver.kind {
                    TyKind::Ref(_region, ty, mutability) => {
                        if match_type(cx, ty, &paths::MUTEX) && mutability == Mutability::MutMutable {
                            span_help_and_lint(
                                cx,
                                MUTEX_MUTABLE_SELF,
                                expr.span,
                                "exclusive mutex variable",
                                &format!("consider `Mutex::get_mut` instead"),
                            );
                        }
                    },
                    // i think that receiving self by value (at least sometimes) matches this one
                    TyKind::Adt(adtdef, substs) => {
                        // TODO: not sure how to get the type here; the Debug fmt shows
                        // 'std::sync::Mutex' though, so there must be a way i presume
                    },
                    _ => {},
                }
            }
        }
    }
}

