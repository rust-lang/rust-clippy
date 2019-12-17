use if_chain::if_chain;
use rustc::declare_lint_pass;
use rustc::hir::map::Map;
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc_hir::intravisit::*;
use rustc_hir::*;
use rustc_session::declare_tool_lint;

use crate::utils::span_lint;

declare_clippy_lint! {
    /// **What it does:** Checks for lifetime annotations on return values
    /// which might not always get bound.
    ///
    /// **Why is this bad?** If the function contains unsafe code which
    /// transmutes to the lifetime, the resulting lifetime may live longer
    /// than was intended.
    ///
    /// **Known problems*: None.
    ///
    /// **Example:**
    /// ```rust
    /// struct WrappedStr(str);
    ///
    /// // Bad: unbound return lifetime causing unsoundness (e.g. when x is String)
    /// fn unbound<'a>(x: impl AsRef<str> + 'a) -> &'a WrappedStr {
    ///    let s = x.as_ref();
    ///    unsafe { &*(s as *const str as *const WrappedStr) }
    /// }
    ///
    /// // Good: bound return lifetime is sound
    /// fn bound<'a>(x: &'a str) -> &'a WrappedStr {
    ///   unsafe { &*(x as *const str as *const WrappedStr) }
    /// }
    /// ```
    pub UNBOUND_RETURN_LIFETIMES,
    correctness,
    "unbound lifetimes in function return values"
}

declare_lint_pass!(UnboundReturnLifetimes => [UNBOUND_RETURN_LIFETIMES]);

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for UnboundReturnLifetimes {
    fn check_item(&mut self, cx: &LateContext<'a, 'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Fn(ref sig, ref generics, _id) = item.kind {
            check_fn_inner(cx, &sig.decl, generics, None);
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'a, 'tcx>, item: &'tcx ImplItem<'tcx>) {
        if let ImplItemKind::Method(ref sig, _) = item.kind {
            let parent_generics = impl_generics(cx, item.hir_id);
            check_fn_inner(cx, &sig.decl, &item.generics, parent_generics);
        }
    }
}

fn check_fn_inner<'a, 'tcx>(
    cx: &LateContext<'a, 'tcx>,
    decl: &'tcx FnDecl<'tcx>,
    generics: &'tcx Generics<'tcx>,
    parent_data: Option<(&'tcx Generics<'tcx>, &'tcx Ty<'tcx>)>,
) {
    let output_type = if let FunctionRetTy::Return(ref output_type) = decl.output {
        output_type
    } else {
        return;
    };

    let lt = if let TyKind::Rptr(ref lt, _) = output_type.kind {
        lt
    } else {
        return;
    };

    if matches!(lt.name, LifetimeName::Param(_)) {
        let target_lt = lt;

        // check function generics
        // the target lifetime parameter must appear on the left of some outlives relation
        if lifetime_outlives_something(target_lt, generics) {
            return;
        }

        // check parent generics
        // the target lifetime parameter must appear on the left of some outlives relation
        if let Some((ref parent_generics, _)) = parent_data {
            if lifetime_outlives_something(target_lt, parent_generics) {
                return;
            }
        }

        // check type generics
        // the target lifetime parameter must be included in the type
        if let Some((_, ref parent_ty)) = parent_data {
            if TypeVisitor::type_contains_lifetime(parent_ty, target_lt) {
                return;
            }
        }

        // check arguments the target lifetime parameter must be included as a
        // lifetime of a reference, either directly or through the gneric
        // parameters of the argument type.
        for input in decl.inputs.iter() {
            if TypeVisitor::type_contains_lifetime(input, target_lt) {
                return;
            }
        }

        span_lint(
            cx,
            UNBOUND_RETURN_LIFETIMES,
            target_lt.span,
            "lifetime is unconstrained",
        );
    }
}

struct TypeVisitor<'tcx> {
    found: bool,
    target_lt: &'tcx Lifetime,
}

impl<'tcx> TypeVisitor<'tcx> {
    fn type_contains_lifetime(ty: &Ty<'_>, target_lt: &'tcx Lifetime) -> bool {
        let mut visitor = TypeVisitor {
            found: false,
            target_lt,
        };
        walk_ty(&mut visitor, ty);
        visitor.found
    }
}

impl<'tcx> Visitor<'tcx> for TypeVisitor<'tcx> {
    type Map = Map<'tcx>;

    fn visit_lifetime(&mut self, lt: &'tcx Lifetime) {
        if lt.name == self.target_lt.name {
            self.found = true;
        }
    }

    fn visit_ty(&mut self, ty: &'tcx Ty<'_>) {
        match ty.kind {
            TyKind::Rptr(ref lt, _) => {
                if lt.name == self.target_lt.name {
                    self.found = true;
                }
            },
            TyKind::Path(ref qpath) => {
                if !self.found {
                    walk_qpath(self, qpath, ty.hir_id, ty.span);
                }
            },
            _ => (),
        }
    }

    fn nested_visit_map(&mut self) -> NestedVisitorMap<'_, Self::Map> {
        NestedVisitorMap::None
    }
}

fn lifetime_outlives_something<'tcx>(target_lt: &'tcx Lifetime, generics: &'tcx Generics<'tcx>) -> bool {
    if let Some(param) = generics.get_named(target_lt.name.ident().name) {
        if param.bounds.iter().any(|b| matches!(b, GenericBound::Outlives(_))) {
            return true;
        }
    }
    false
}

fn impl_generics<'tcx>(cx: &LateContext<'_, 'tcx>, hir_id: HirId) -> Option<(&'tcx Generics<'tcx>, &'tcx Ty<'tcx>)> {
    let parent_impl = cx.tcx.hir().get_parent_item(hir_id);
    if_chain! {
        if parent_impl != CRATE_HIR_ID;
        if let Node::Item(item) = cx.tcx.hir().get(parent_impl);
        if let ItemKind::Impl(_, _, _, ref parent_generics, _, ref ty, _) = item.kind;
        then {
            return Some((parent_generics, ty))
        }
    }
    None
}
