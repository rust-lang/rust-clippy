use rustc_attr::InlineAttr;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{AssocItemKind, Body, FnDecl, GenericParam, GenericParamKind, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;

use clippy_utils::diagnostics::span_lint;

declare_clippy_lint! {
    /// ### What it does
    /// It lints any generic functions, or methods which are marked with
    /// the `#[inline]`, or `#[inline(always)]` attributes.
    ///
    /// ### Why is this bad?
    /// It's not inherently bad to mark generic functions, or methods with
    /// the `#[inline]`, or `#[inline(always)]` attributes, but it can possibly
    /// increase compilation times, because the compiler will already monomorphize
    /// generic functions per-crate, while inlining a function/method will also
    /// cause the compiler to recompile per code-gen unit, which may cause even
    /// longer compile times.
    ///
    /// ### Example
    /// ```rust
    /// #[inline]
    /// fn foo<F>(_: F) {} // generic function is marked `#[inline]`
    ///
    /// #[inline(always)]
    /// fn bar<B>(_: B) {} // generic function is marked `#[inline(always)]`
    ///
    /// struct Foo {
    ///     str: String,
    /// }
    ///
    /// impl Foo {
    ///     #[inline] // generic method is marked `#[inline]`
    ///     fn new<S: AsRef<str>>(str: S) -> Self {
    ///         Self {
    ///             str: str.as_ref().to_owned(),
    ///         }
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// fn foo<F>(_: F) {}
    ///
    /// fn bar<B>(_: B) {}
    ///
    /// struct Foo {
    ///     str: String,
    /// }
    ///
    /// impl Foo {
    ///     fn new<S: AsRef<str>>(str: S) -> Self {
    ///         Self {
    ///             str: str.as_ref().to_owned(),
    ///         }
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.70.0"]
    pub INLINED_GENERICS,
    restriction,
    "detects generic functions, or methods which are marked `#[inline]`, or `#[inline(always)]`"
}

declare_lint_pass!(InlinedGenerics => [INLINED_GENERICS]);

fn requires_monomorphization<'hir>(params: &'hir [GenericParam<'hir>]) -> bool {
    params.iter().any(|param| {
        matches!(
            param.kind,
            GenericParamKind::Type { .. } | GenericParamKind::Const { .. }
        )
    })
}

fn lint_inlined_generics(ctx: &LateContext<'_>, span: Span, desc: &'static str, inline: &'static str) {
    span_lint(
        ctx,
        INLINED_GENERICS,
        span,
        &format!("generic {desc} is marked `{inline}`"),
    );
}

impl<'tcx> LateLintPass<'tcx> for InlinedGenerics {
    fn check_fn(
        &mut self,
        ctx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        _: &'tcx Body<'tcx>,
        span: Span,
        def_id: LocalDefId,
    ) {
        if !ctx.tcx.generics_of(def_id).own_requires_monomorphization() {
            return;
        }

        let inline = match ctx.tcx.codegen_fn_attrs(def_id).inline {
            InlineAttr::Never | InlineAttr::None => return,
            InlineAttr::Always => "#[inline(always)]",
            InlineAttr::Hint => "#[inline]",
        };
        match kind {
            FnKind::ItemFn(..) => {
                lint_inlined_generics(ctx, span, "function", inline);
            },
            FnKind::Method(..) => {
                lint_inlined_generics(ctx, span, "method", inline);
            },
            FnKind::Closure => {},
        }
    }

    fn check_item(&mut self, ctx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        match item.kind {
            ItemKind::Trait(.., generics, _, items) => {
                let monomorphize = requires_monomorphization(generics.params);
                for item in items {
                    let def_id = item.id.owner_id.def_id;
                    if !(monomorphize || ctx.tcx.generics_of(def_id).own_requires_monomorphization()) {
                        continue;
                    }

                    if let AssocItemKind::Fn { has_self: true } = item.kind {
                        let inline = match ctx.tcx.codegen_fn_attrs(def_id).inline {
                            InlineAttr::Never | InlineAttr::None => continue,
                            InlineAttr::Always => "#[inline(always)]",
                            InlineAttr::Hint => "#[inline]",
                        };
                        lint_inlined_generics(ctx, item.span, "trait method", inline);
                    }
                }
            },
            ItemKind::Impl(impl_block) => {
                let monomorphize = requires_monomorphization(impl_block.generics.params);
                for item in impl_block.items {
                    let def_id = item.id.owner_id.def_id;
                    if !(monomorphize || ctx.tcx.generics_of(def_id).own_requires_monomorphization()) {
                        continue;
                    }

                    if let AssocItemKind::Fn { has_self: true } = item.kind {
                        let inline = match ctx.tcx.codegen_fn_attrs(def_id).inline {
                            InlineAttr::Never | InlineAttr::None => continue,
                            InlineAttr::Always => "#[inline(always)]",
                            InlineAttr::Hint => "#[inline]",
                        };
                        lint_inlined_generics(ctx, item.span, "method", inline);
                    }
                }
            },
            ItemKind::ExternCrate(..)
            | ItemKind::Use(..)
            | ItemKind::Static(..)
            | ItemKind::Const(..)
            | ItemKind::Fn(..)
            | ItemKind::Macro(..)
            | ItemKind::Mod(..)
            | ItemKind::ForeignMod { .. }
            | ItemKind::GlobalAsm(..)
            | ItemKind::TyAlias(..)
            | ItemKind::OpaqueTy(..)
            | ItemKind::Enum(..)
            | ItemKind::Struct(..)
            | ItemKind::Union(..)
            | ItemKind::TraitAlias(..) => {},
        }
    }
}
