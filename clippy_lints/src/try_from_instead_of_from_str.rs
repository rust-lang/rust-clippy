use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::paths::{PathNS, lookup_path};
use clippy_utils::source::snippet_opt;
use clippy_utils::sym;
use clippy_utils::ty::{implements_trait, ty_from_hir_ty};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{BodyId, GenericArg, Impl, ImplItemKind, Item, ItemKind, LifetimeKind, TraitRef, Ty};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Mutability};
use rustc_session::declare_lint_pass;
use rustc_span::symbol::kw;

use core::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Check if there is a `TryFrom<&str>` implementation.
    ///
    /// ### Why is this bad?
    ///
    /// It is more idiomatic to use `FromStr`.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// # use std::convert::TryFrom;
    /// # struct MyType;
    /// # struct MyError;
    /// impl TryFrom<&str> for MyType {
    ///     type Error = MyError;
    ///     fn try_from(value: &str) -> Result<Self, Self::Error> {
    /// #       Ok(MyType)
    ///     }
    /// }
    /// ```
    ///
    /// Use instead:
    ///
    /// ```no_run
    /// # use std::str::FromStr;
    /// # struct MyType;
    /// # struct MyError;
    /// impl FromStr for MyType {
    ///     type Err = MyError;
    ///     fn from_str(value: &str) -> Result<Self, Self::Err> {
    /// #       Ok(MyType)
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.97.0"]
    pub TRY_FROM_INSTEAD_OF_FROM_STR,
    style,
    "TryFrom<str> instead of FromStr"
}

declare_lint_pass!(TryFromInsteadOfFromStr => [TRY_FROM_INSTEAD_OF_FROM_STR]);

struct LifetimeVisitor {
    forbidden_lifetime: LocalDefId,
}

impl<'tcx> rustc_hir::intravisit::Visitor<'tcx> for LifetimeVisitor {
    type Result = ControlFlow<(), ()>;

    fn visit_lifetime(&mut self, lifetime: &'tcx rustc_hir::Lifetime) -> Self::Result {
        if let LifetimeKind::Param(def_id) = lifetime.kind
            && def_id == self.forbidden_lifetime
        {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }
}

// In this function, we check that the error type doesn't use the same lifetime as
// `TryFrom<&'a str>`.
fn check_lifetimes(ty: &Ty<'_>, imp: &Impl<'_>, trait_ref: &TraitRef<'_>) -> bool {
    let Some(forbidden_lifetime) = get_try_from_str_lifetime(trait_ref) else {
        return true;
    };

    let mut visitor = LifetimeVisitor { forbidden_lifetime };

    if rustc_hir::intravisit::walk_unambig_ty(&mut visitor, ty).is_break()
        || rustc_hir::intravisit::walk_unambig_ty(&mut visitor, imp.self_ty).is_break()
    {
        return false;
    }

    // And finally we check that the `&str` lifetime is not used in the impl generics.
    imp.generics
        .predicates
        .iter()
        .all(|pred| rustc_hir::intravisit::walk_where_predicate(&mut visitor, pred).is_continue())
}

fn get_impl_items<'tcx>(cx: &LateContext<'tcx>, imp: &Impl<'_>) -> Option<(&'tcx Ty<'tcx>, BodyId)> {
    if let &[item1, item2] = &imp.items {
        match (
            cx.tcx.hir_expect_impl_item(item1.owner_id.def_id).kind,
            cx.tcx.hir_expect_impl_item(item2.owner_id.def_id).kind,
        ) {
            (ImplItemKind::Type(ty), ImplItemKind::Fn(_, body_id))
            | (ImplItemKind::Fn(_, body_id), ImplItemKind::Type(ty)) => Some((ty, body_id)),
            _ => None,
        }
    } else {
        None
    }
}

fn get_try_from_str_lifetime(trait_ref: &TraitRef<'_>) -> Option<LocalDefId> {
    if let Some(segment) = trait_ref.path.segments.last()
        && let Some(args) = segment.args
        && let Some(ty) = args.args.iter().find_map(|arg| match arg {
            GenericArg::Type(ty) => Some(ty),
            _ => None,
        })
        && let rustc_hir::TyKind::Ref(lifetime, ..) = ty.kind
        && lifetime.ident.name != kw::UnderscoreLifetime
        && let LifetimeKind::Param(def_id) = lifetime.kind
    {
        Some(def_id)
    } else {
        None
    }
}

impl<'tcx> LateLintPass<'tcx> for TryFromInsteadOfFromStr {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Impl(imp) = &item.kind
            && let Some(of_trait) = imp.of_trait
            // It should only have an associated type (`Error`) and a method (`try_from`).
            && let Some((err_ty, fn_body)) = get_impl_items(cx, imp)
            && let Some(trait_def_id) = of_trait.trait_ref.trait_def_id()
            && let impl_def_id = item.owner_id.to_def_id()
            && let trait_ref = cx.tcx.impl_trait_ref(impl_def_id)
            && let instantiated_trait_ref = trait_ref.instantiate_identity().skip_normalization()
            // We check if the "from" item is a non-mutable `str`.
            && let [_, from_arg] = &**instantiated_trait_ref.args
            && let Some(from_ty) = from_arg.as_type()
            && let ty::Ref(_, inner_ty, Mutability::Not) = from_ty.kind()
            && inner_ty.is_str()
            // We check that the trait is `TryFrom`.
            && cx.tcx.is_diagnostic_item(sym::TryFrom, trait_def_id)
            // We check that our type doesn't already implement `FromStr`.
            && !lookup_path(cx.tcx, PathNS::Type, &[sym::core, sym::str, sym::FromStr])
                .into_iter()
                .any(|def_id| {
                    implements_trait(cx, ty_from_hir_ty(cx, imp.self_ty), def_id, &[])
                })
            // We check that the lifetime used by `TryFrom::Error` is not used anywhere else.
            && check_lifetimes(err_ty, imp, &of_trait.trait_ref)
            // We retrieve all snippets for lint suggestion.
            && let Some(fn_body) = snippet_opt(cx, cx.tcx.hir_span_with_body(fn_body.hir_id))
            && let Some(err_ty) = snippet_opt(cx, err_ty.span)
            && let Some(generics) = snippet_opt(cx, imp.generics.span)
            && let Some(self_ty) = snippet_opt(cx, imp.self_ty.span)
        {
            span_lint_and_sugg(
                cx,
                TRY_FROM_INSTEAD_OF_FROM_STR,
                item.span,
                "`TryFrom<str>` could be `FromStr`",
                "replace with",
                format!(
                    "\
impl{generics} core::str::FromStr for {self_ty} {{
    type Err = {err_ty};

    fn from_str(value: &str) -> Result<Self, Self::Err> {fn_body}
}}"
                ),
                Applicability::MaybeIncorrect,
            );
        }
    }
}
