use clippy_utils::diagnostics::span_lint_and_then;
use itertools::Itertools as _;
use rustc_hir::{GenericArg, GenericBound, GenericBounds, PolyTraitRef};
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Enforce that sealed generic traits use their generic type parameters in their "sealing" supertrait bounds.
    ///
    /// ### Why restrict this?
    /// Not sealing a generic trait properly can result in certain downstream impls still being possible.
    ///
    /// Also, even if one of the supertraits does have all generic args of the trait, a supertrait
    /// without them allows certain downstream blanket impls. This can make it a breaking change
    /// to remove such a bound.
    ///
    /// The only way to fully guard against this is to make sure that all type parameters of the trait
    /// appear in all sealing (i.e. publicly inaccessible) supertraits.
    ///
    /// ### Known limitations
    /// This lint currently only warns if the sealing supertraits all have no generic type arguments at all.
    ///
    /// ### Example
    /// ```no_run
    /// mod sealed {
    ///     pub trait Sealed {}
    /// }
    /// pub trait Generic<T>: sealed::Sealed {}
    /// ```
    ///
    /// Instead write:
    /// ```no_run
    /// mod sealed {
    ///     pub trait Sealed<T> {}
    /// }
    /// pub trait Generic<T>: sealed::Sealed<T> {}
    /// ```
    #[clippy::version = "1.101.0"]
    pub SUSPICIOUS_SEALED_TRAIT,
    suspicious,
    "enforce that sealed generic traits are properly sealed"
}

impl_lint_pass!(SuspiciousSealedTrait => [SUSPICIOUS_SEALED_TRAIT]);

#[derive(Default)]
pub struct SuspiciousSealedTrait;

impl<'tcx> LateLintPass<'tcx> for SuspiciousSealedTrait {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx rustc_hir::Item<'tcx>) {
        if let rustc_hir::ItemKind::Trait { generics, bounds, .. } = item.kind
            && cx.tcx.visibility(item.owner_id).is_public()
            && cx.effective_visibilities.is_exported(item.owner_id.def_id)
            && generics
                .params
                .iter()
                .any(|param| matches!(param.kind, rustc_hir::GenericParamKind::Type { .. }))
            && let Some(sealing_clause) = check_supertraits(cx, bounds)
        {
            lint_trait(
                cx,
                sealing_clause.span,
                &cx.tcx.def_path_str(item.owner_id),
                &cx.tcx.def_path_str(sealing_clause.trait_ref.path.res.def_id()),
                generics
                    .params
                    .iter()
                    .filter(|param| matches!(param.kind, rustc_hir::GenericParamKind::Type { .. })),
            );
        }
    }
}

fn check_supertraits<'tcx>(
    cx: &LateContext<'tcx>,
    super_bounds: GenericBounds<'tcx>,
) -> Option<&'tcx PolyTraitRef<'tcx>> {
    let mut found = None;

    for bound in super_bounds {
        // Find empty local supertraits that are not publicly accessible
        if let GenericBound::Trait(trait_clause) = bound
            && let Some(local_id) = trait_clause.trait_ref.path.res.def_id().as_local()
            && !cx.effective_visibilities.is_exported(local_id)
            && cx.tcx.associated_items(local_id).len() == 0
        {
            // if at least one is generic, don't lint
            if trait_clause
                .trait_ref
                .path
                .segments
                .last()
                .unwrap()
                .args
                .is_some_and(|args| args.args.iter().any(|it| matches!(it, GenericArg::Type(_))))
            {
                return None;
            }

            found = Some(trait_clause);
        }
    }

    found
}

fn lint_trait<'tcx>(
    cx: &LateContext<'tcx>,
    span: Span,
    trait_name: &str,
    supertrait_name: &str,
    type_params: impl Iterator<Item = &'tcx rustc_hir::GenericParam<'tcx>>,
) {
    span_lint_and_then(
        cx,
        SUSPICIOUS_SEALED_TRAIT,
        span,
        format!("sealed generic trait `{trait_name}` uses non-generic sealing supertrait"),
        |diag| {
            let param_names = type_params
                .format_with(", ", |param, fmt| fmt(&format_args!("`{}`", param.name.ident())))
                .to_string();

            diag.note(format!("without depending on {param_names}, this bound fails to fully seal the trait, which potentially allows downstream implementations"));
            diag.help(format!("make `{supertrait_name}` generic over {param_names}"));
            diag.note("fixing this is technically a breaking change");
        },
    );
}
