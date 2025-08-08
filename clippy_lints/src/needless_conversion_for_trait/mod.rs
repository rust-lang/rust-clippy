use clippy_config::Conf;
use clippy_config::types::{
    ConfPath, ConfPathWithoutReplacement, SymPath, conf_path_from_sym_path, create_conf_path_map,
};
use clippy_utils::diagnostics::{span_lint_and_help, span_lint_and_sugg};
use clippy_utils::msrvs::Msrv;
use clippy_utils::paths::{self, PathNS};
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::{
    build_check_predicates_with_new_ty_closure, get_callee_generic_args_and_args, is_copy, is_on_string_like,
    is_to_string,
};
use clippy_utils::{get_parent_expr, sym};
use rustc_errors::Applicability;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, DefIdMap};
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::adjustment::{Adjust, Adjustment, AutoBorrow};
use rustc_middle::ty::{FnDef, GenericArgsRef, Param, ParamTy, Ty, TyCtxt};
use rustc_session::impl_lint_pass;
use rustc_span::symbol::Symbol;

#[cfg(debug_assertions)]
mod check_inherent_functions;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for trait-behavior-preserving calls in positions where a trait implementation is
    /// expected.
    ///
    /// ### Why is this bad?
    ///
    /// Such unnecessary calls make the code more verbose and could impact performance.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # use std::{path::Path, process::Command};
    /// let _ = Command::new("ls").args(["-a", "-l"].iter());
    /// let _ = Path::new("/").join(Path::new("."));
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// # use std::{path::Path, process::Command};
    /// let _ = Command::new("ls").args(["-a", "-l"]);
    /// let _ = Path::new("/").join(".");
    /// ```
    #[clippy::version = "1.89.0"]
    pub NEEDLESS_CONVERSION_FOR_TRAIT,
    complexity,
    "unnecessary calls that preserve trait behavior"
}

impl_lint_pass!(NeedlessConversionForTrait => [NEEDLESS_CONVERSION_FOR_TRAIT]);

pub struct NeedlessConversionForTrait {
    watched_inherent_functions_builtin: DefIdMap<(&'static [Symbol], &'static ConfPath<SymPath<'static>, false>)>,
    watched_trait_methods_builtin: DefIdMap<(&'static [Symbol], &'static ConfPath<SymPath<'static>, false>)>,
    watched_functions_from_conf: DefIdMap<(&'static str, &'static ConfPathWithoutReplacement)>,
    msrv: Msrv,
}

impl NeedlessConversionForTrait {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf) -> Self {
        let (watched_inherent_functions_builtin, _) = create_conf_path_map(
            tcx,
            WATCHED_INHERENT_FUNCTIONS,
            PathNS::Value,
            |def_kind| matches!(def_kind, DefKind::Fn | DefKind::AssocFn),
            "function",
            false,
        );
        let (watched_trait_methods_builtin, _) = create_conf_path_map(
            tcx,
            WATCHED_TRAIT_METHODS,
            PathNS::Value,
            |def_kind| matches!(def_kind, DefKind::Fn | DefKind::AssocFn),
            "function",
            false,
        );
        let (watched_functions_from_conf, _) = create_conf_path_map(
            tcx,
            &conf.watched_functions,
            PathNS::Value,
            |def_kind| matches!(def_kind, DefKind::Fn | DefKind::AssocFn),
            "function",
            false,
        );
        Self {
            watched_inherent_functions_builtin,
            watched_trait_methods_builtin,
            watched_functions_from_conf,
            msrv: conf.msrv,
        }
    }
}

macro_rules! conf_paths {
    ($($sym_path:expr,)*) => {
        &[
            $(conf_path_from_sym_path($sym_path),)*
        ]
    };
}

/// Inherent functions that are trait preserving.
///
/// Intuitively, a function is trait preserving if it changes only its argument's type, but not the
/// argument's value.
///
/// For example, [`str::as_bytes`] is trait preserving because it changes the type of its argument
/// from `&str` to `&[u8]`, but does not change the argument's value.
///
/// On the other hand, [`str::to_lowercase`] is not trait preserving because it changes uppercase
/// characters to lowercase characters.
///
/// [`str::to_lowercase`]: https://doc.rust-lang.org/std/primitive.str.html#method.to_lowercase
const WATCHED_INHERENT_FUNCTIONS: &[ConfPath<SymPath<'static>, false>] = conf_paths!(
    &[sym::slice, sym::into_vec],
    &[sym::slice, sym::to_vec],
    &[sym::str, sym::into_boxed_bytes],
    &[sym::str, sym::into_string],
    &[sym::alloc, sym::string, sym::String, sym::as_bytes],
    &[sym::alloc, sym::string, sym::String, sym::as_mut_str],
    &[sym::alloc, sym::string, sym::String, sym::as_str],
    &[sym::alloc, sym::string, sym::String, sym::into_boxed_str],
    &[sym::alloc, sym::string, sym::String, sym::into_bytes],
    &[sym::alloc, sym::vec, sym::Vec, sym::as_mut_slice],
    &[sym::alloc, sym::vec, sym::Vec, sym::as_slice],
    &[sym::alloc, sym::vec, sym::Vec, sym::into_boxed_slice],
    &[sym::slice, sym::iter],
    &[sym::slice, sym::iter_mut],
    &[sym::str, sym::as_bytes],
    &[sym::str, sym::as_str],
    &[sym::std, sym::ffi, sym::os_str, sym::OsStr, sym::as_encoded_bytes],
    &[sym::std, sym::ffi, sym::os_str, sym::OsStr, sym::into_os_string],
    &[sym::std, sym::ffi, sym::os_str, sym::OsStr, sym::new],
    &[sym::std, sym::ffi, sym::os_str, sym::OsStr, sym::to_os_string],
    &[sym::std, sym::ffi, sym::os_str, sym::OsString, sym::as_os_str],
    &[sym::std, sym::ffi, sym::os_str, sym::OsString, sym::into_boxed_os_str],
    &[sym::std, sym::ffi, sym::os_str, sym::OsString, sym::into_encoded_bytes],
    &[sym::std, sym::path, sym::Path, sym::as_mut_os_str],
    &[sym::std, sym::path, sym::Path, sym::as_os_str],
    &[sym::std, sym::path, sym::Path, sym::into_path_buf],
    &[sym::std, sym::path, sym::Path, sym::iter],
    &[sym::std, sym::path, sym::Path, sym::new],
    &[sym::std, sym::path, sym::Path, sym::to_path_buf],
    &[sym::std, sym::path, sym::PathBuf, sym::as_mut_os_string],
    &[sym::std, sym::path, sym::PathBuf, sym::as_path],
    &[sym::std, sym::path, sym::PathBuf, sym::into_boxed_path],
    &[sym::std, sym::path, sym::PathBuf, sym::into_os_string],
);

/// Inherent functions that are not trait preserving.
///
/// See [`WATCHED_INHERENT_FUNCTIONS`] for an explanation of "trait preserving".
const IGNORED_INHERENT_FUNCTIONS: &[ConfPath<SymPath<'static>, false>] = conf_paths!(
    &[sym::str, sym::to_ascii_lowercase],
    &[sym::str, sym::to_ascii_uppercase],
    &[sym::str, sym::to_lowercase],
    &[sym::str, sym::to_uppercase],
    &[sym::alloc, sym::string, sym::String, sym::from_utf16_lossy],
    &[sym::alloc, sym::string, sym::String, sym::from_utf16be_lossy],
    &[sym::alloc, sym::string, sym::String, sym::from_utf16le_lossy],
    &[sym::alloc, sym::string, sym::String, sym::from_utf8_lossy_owned],
    &[sym::alloc, sym::string, sym::String, sym::leak],
    &[sym::alloc, sym::vec, sym::Vec, sym::into_chunks],
    &[sym::alloc, sym::vec, sym::Vec, sym::into_flattened],
    &[sym::alloc, sym::vec, sym::Vec, sym::leak],
    &[sym::alloc, sym::vec, sym::Vec, sym::spare_capacity_mut],
    &[sym::str, sym::trim],
    &[sym::str, sym::trim_ascii],
    &[sym::str, sym::trim_ascii_end],
    &[sym::str, sym::trim_ascii_start],
    &[sym::str, sym::trim_end],
    &[sym::str, sym::trim_left],
    &[sym::str, sym::trim_right],
    &[sym::str, sym::trim_start],
    &[sym::std, sym::ffi, sym::os_str, sym::OsStr, sym::to_ascii_lowercase],
    &[sym::std, sym::ffi, sym::os_str, sym::OsStr, sym::to_ascii_uppercase],
    &[sym::std, sym::ffi, sym::os_str, sym::OsString, sym::leak],
    &[sym::std, sym::path, sym::PathBuf, sym::leak],
);

/// Trait methods that are trait preserving.
///
/// See [`WATCHED_INHERENT_FUNCTIONS`] for an explanation of "trait preserving".
const WATCHED_TRAIT_METHODS: &[ConfPath<SymPath<'static>, false>] = conf_paths!(
    &[sym::alloc, sym::borrow, sym::ToOwned, sym::to_owned],
    &[sym::alloc, sym::string, sym::ToString, sym::to_string],
    &[sym::core, sym::borrow, sym::Borrow, sym::borrow],
    &[sym::core, sym::borrow, sym::BorrowMut, sym::borrow_mut],
    &[sym::core, sym::convert, sym::AsMut, sym::as_mut],
    &[sym::core, sym::convert, sym::AsRef, sym::as_ref],
    &[sym::core, sym::ops, sym::deref, sym::Deref, sym::deref],
    &[sym::core, sym::ops, sym::deref, sym::DerefMut, sym::deref_mut],
);

impl<'tcx> LateLintPass<'tcx> for NeedlessConversionForTrait {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some((maybe_call, maybe_arg, ancestor_mutabilities)) = ancestor_addr_of_mutabilities(cx, expr)
            && let Some((outer_callee_def_id, outer_generic_args, outer_receiver, outer_args)) =
                get_callee_generic_args_and_args(cx, maybe_call)
            && let outer_args = std::iter::once(outer_receiver)
                .flatten()
                .chain(outer_args)
                .collect::<Vec<_>>()
            && let outer_fn_sig = cx.tcx.fn_sig(outer_callee_def_id).skip_binder().skip_binder()
            && let Some(i) = outer_args.iter().position(|arg| arg.hir_id == maybe_arg.hir_id)
            && let Some(input) = outer_fn_sig.inputs().get(i)
            && let Param(param_ty) = input.kind()
        {
            let strip_unnecessary_conversions = |mut expr, mut mutabilities| {
                let mut refs_prefix = None;

                loop {
                    if let Some((inner_callee_def_id, _inner_generic_args, inner_receiver, inner_args)) =
                        get_callee_generic_args_and_args(cx, expr)
                        && let inner_args = std::iter::once(inner_receiver)
                            .flatten()
                            .chain(inner_args)
                            .collect::<Vec<_>>()
                        && let &[maybe_boxed_inner_arg] = inner_args.as_slice()
                        && let inner_arg = peel_boxes(cx, maybe_boxed_inner_arg)
                        && let inner_arg_ty = cx.typeck_results().expr_ty(inner_arg)
                        && let adjustment_mutabilities = adjustment_mutabilities(cx, inner_arg)
                        && let new_mutabilities = [adjustment_mutabilities, mutabilities].concat()
                        && let (new_ty, new_refs_prefix) = build_ty_and_refs_prefix(cx, inner_arg_ty, &new_mutabilities)
                        && inner_arg_implements_traits(
                            cx,
                            outer_callee_def_id,
                            outer_generic_args,
                            i,
                            *param_ty,
                            new_ty,
                            self.msrv,
                        )
                        // For `unnecessary_to_owned` to eliminate a call to `ToString::to_string`,
                        // the receiver's type must implement `Deref<Target = str>` or `AsRef<str>`.
                        // The same restriction is applied here. See:
                        // https://github.com/rust-lang/rust-clippy/blob/e6dc2e9be026e9d26296a51e496d1b25b5e0721e/clippy_lints/src/methods/unnecessary_to_owned.rs#L651-L652
                        && (!inner_callee_def_id
                            .opt_parent(cx)
                            .is_some_and(|inner_callee_parent_id| is_to_string(cx, expr, inner_callee_parent_id))
                            || is_on_string_like(cx, expr))
                    {
                        if !self
                            .watched_inherent_functions_builtin
                            .contains_key(&inner_callee_def_id)
                            && !self.watched_trait_methods_builtin.contains_key(&inner_callee_def_id)
                            && !self.watched_functions_from_conf.contains_key(&inner_callee_def_id)
                        {
                            break;
                        }
                        expr = inner_arg;
                        mutabilities = new_mutabilities;
                        refs_prefix = Some(new_refs_prefix);
                        continue;
                    }
                    break;
                }

                Some(expr).zip(refs_prefix)
            };

            if let Some((inner_arg, refs_prefix)) = strip_unnecessary_conversions(expr, ancestor_mutabilities) {
                let (is_bare_method_call, subject) = if matches!(expr.kind, ExprKind::MethodCall(..)) {
                    (maybe_arg.hir_id == expr.hir_id, "receiver")
                } else {
                    (false, "inner argument")
                };
                let msg = format!("the {subject} implements the required traits");
                if is_bare_method_call && refs_prefix.is_empty() && !maybe_arg.span.from_expansion() {
                    span_lint_and_sugg(
                        cx,
                        NEEDLESS_CONVERSION_FOR_TRAIT,
                        maybe_arg.span.with_lo(inner_arg.span.hi()),
                        msg,
                        "remove this",
                        String::new(),
                        Applicability::MachineApplicable,
                    );
                } else if maybe_arg.span.from_expansion()
                    && let Some(span) = maybe_arg.span.parent_callsite()
                {
                    span_lint_and_help(
                        cx,
                        NEEDLESS_CONVERSION_FOR_TRAIT,
                        span,
                        msg,
                        None,
                        "use the macro arguments directly",
                    );
                } else if let Some(snippet) = snippet_opt(cx, inner_arg.span) {
                    span_lint_and_sugg(
                        cx,
                        NEEDLESS_CONVERSION_FOR_TRAIT,
                        maybe_arg.span,
                        msg,
                        "use",
                        format!("{refs_prefix}{snippet}"),
                        Applicability::MachineApplicable,
                    );
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        check_inherent_functions::check_inherent_functions(cx, self);
    }
}

/// `inner_arg_implements_traits` is similar to `needless_borrow_count` from
/// [`needless_borrows_for_generic_args`]. The core logic of both functions is in
/// [`clippy_utils::ty::build_check_predicates_with_new_ty_closure`].
///
/// [`needless_borrows_for_generic_args`]: https://rust-lang.github.io/rust-clippy/master/index.html#needless_borrows_for_generic_args
fn inner_arg_implements_traits<'tcx>(
    cx: &LateContext<'tcx>,
    fn_id: DefId,
    callee_generic_args: GenericArgsRef<'tcx>,
    arg_index: usize,
    param_ty: ParamTy,
    new_ty: Ty<'tcx>,
    msrv: Msrv,
) -> bool {
    build_check_predicates_with_new_ty_closure(cx, fn_id, callee_generic_args, arg_index, param_ty, false, msrv)
        .is_some_and(|mut f| f(new_ty))
}

fn ancestor_addr_of_mutabilities<'tcx>(
    cx: &LateContext<'tcx>,
    mut expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>, Vec<Mutability>)> {
    let mut mutabilities = Vec::new();
    while let Some(parent) = get_parent_expr(cx, expr) {
        if let ExprKind::AddrOf(BorrowKind::Ref, mutability, _) = parent.kind {
            mutabilities.push(mutability);
            expr = parent;
        } else {
            return Some((parent, expr, mutabilities));
        }
    }
    None
}

fn peel_boxes<'tcx>(cx: &LateContext<'tcx>, mut expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    loop {
        if let ExprKind::Call(callee, args) = expr.kind
            && let callee_ty = cx.typeck_results().expr_ty(callee)
            && let FnDef(callee_def_id, _) = callee_ty.kind()
            && paths::BOX_NEW.matches(cx, *callee_def_id)
            && let [inner_arg] = args
        {
            expr = inner_arg;
            continue;
        }

        break;
    }

    expr
}

fn adjustment_mutabilities<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Vec<Mutability> {
    cx.typeck_results()
        .expr_adjustments(expr)
        .iter()
        .map_while(|adjustment| {
            if let Adjustment {
                kind: Adjust::Borrow(AutoBorrow::Ref(mutability)),
                target: _,
            } = adjustment
            {
                Some((*mutability).into())
            } else {
                None
            }
        })
        .collect()
}

fn build_ty_and_refs_prefix<'tcx>(
    cx: &LateContext<'tcx>,
    mut ty: Ty<'tcx>,
    mutabilities: &[Mutability],
) -> (Ty<'tcx>, String) {
    let mut refs_prefix = String::new();
    for &mutability in mutabilities {
        // If the type is already copy, don't bother adding any more refs.
        if is_copy(cx, ty) {
            break;
        }
        ty = Ty::new_ref(cx.tcx, cx.tcx.lifetimes.re_erased, ty, mutability);
        refs_prefix = "&".to_owned() + mutability.prefix_str() + &refs_prefix;
    }
    (ty, refs_prefix)
}
