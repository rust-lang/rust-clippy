use std::hash::{Hash, Hasher};
use clippy_utils::diagnostics::{span_lint_and_sugg, span_lint_hir_and_then};
use clippy_utils::is_from_proc_macro;
use rustc_ast::Attribute;
use rustc_data_structures::fx::FxIndexSet;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_hir::{Arm, Block, Body, Expr, FieldDef, FnDecl, ForeignItem, GenericParam, Generics, HirId, ImplItem, Item, ItemKind, Local, Mod, Pat, Path, PathSegment, PolyTraitRef, Stmt, TraitItem, Ty, UseKind, Variant, VariantData};
use rustc_hir::intravisit::FnKind;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::impl_lint_pass;
use rustc_span::symbol::kw;
use rustc_span::{sym, Span};
use rustc_span::def_id::LocalDefId;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Finds items imported through `std` when available through `core`.
    ///
    /// ### Why is this bad?
    ///
    /// Crates which have `no_std` compatibility may wish to ensure types are imported from core to ensure
    /// disabling `std` does not cause the crate to fail to compile. This lint is also useful for crates
    /// migrating to become `no_std` compatible.
    ///
    /// ### Example
    /// ```no_run
    /// use std::hash::Hasher;
    /// ```
    /// Use instead:
    /// ```no_run
    /// use core::hash::Hasher;
    /// ```
    #[clippy::version = "1.64.0"]
    pub STD_INSTEAD_OF_CORE,
    restriction,
    "type is imported from std when available in core"
}

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Finds items imported through `std` when available through `alloc`.
    ///
    /// ### Why is this bad?
    ///
    /// Crates which have `no_std` compatibility and require alloc may wish to ensure types are imported from
    /// alloc to ensure disabling `std` does not cause the crate to fail to compile. This lint is also useful
    /// for crates migrating to become `no_std` compatible.
    ///
    /// ### Example
    /// ```no_run
    /// use std::vec::Vec;
    /// ```
    /// Use instead:
    /// ```no_run
    /// # extern crate alloc;
    /// use alloc::vec::Vec;
    /// ```
    #[clippy::version = "1.64.0"]
    pub STD_INSTEAD_OF_ALLOC,
    restriction,
    "type is imported from std when available in alloc"
}

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Finds items imported through `alloc` when available through `core`.
    ///
    /// ### Why is this bad?
    ///
    /// Crates which have `no_std` compatibility and may optionally require alloc may wish to ensure types are
    /// imported from core to ensure disabling `alloc` does not cause the crate to fail to compile. This lint
    /// is also useful for crates migrating to become `no_std` compatible.
    ///
    /// ### Example
    /// ```no_run
    /// # extern crate alloc;
    /// use alloc::slice::from_ref;
    /// ```
    /// Use instead:
    /// ```no_run
    /// use core::slice::from_ref;
    /// ```
    #[clippy::version = "1.64.0"]
    pub ALLOC_INSTEAD_OF_CORE,
    restriction,
    "type is imported from alloc when available in core"
}

#[derive(Default)]
pub struct StdReexports {
    // Paths which can be either a module or a macro (e.g. `std::env`) will cause this check to happen
    // twice. First for the mod, second for the macro. This is used to avoid the lint reporting for the macro
    // when the path could be also be used to access the module.
    prev_span: Span,
    open_use: Option<OpenUseSpan>
}

impl_lint_pass!(StdReexports => [STD_INSTEAD_OF_CORE, STD_INSTEAD_OF_ALLOC, ALLOC_INSTEAD_OF_CORE]);

#[derive(Debug)]
struct OpenUseSpan {
    container: Span,
    members: FxIndexSet<UseSpanMember>
}

#[derive(Debug, Copy, Clone)]
struct UseSpanMember {
    inner: Span,
    hir_id: HirId,
    first_seg_ident_span: Span,
    lint_data: LintData,
}

impl PartialEq for UseSpanMember {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl Eq for UseSpanMember {}

impl Hash for UseSpanMember {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

#[derive(Debug, Copy, Clone)]
enum LintData {
    CanReplace(ReplaceLintData),
    NoReplace,
}

#[derive(Debug, Copy, Clone)]
struct ReplaceLintData {
    lint: &'static crate::Lint,
    used_mod: &'static str,
    replace_with: &'static str,
}

impl StdReexports {
    fn suggest_for_open_use_item_if_after<'tcx>(&mut self, cx: &LateContext<'tcx>, span: Span) {
        if let Some(collected_use) = self.open_use.take() {
            // Still contains other span, throw it back
            if collected_use.container.contains(span) {
                self.open_use = Some(collected_use);
                return;
            }
            if collected_use.members.is_empty() {
                return;
            }
            let mut place_holder_unique_check: Option<(Span, ReplaceLintData)> = None;
            let mut can_chunk = true;
            for member in collected_use.members.iter() {
                match &member.lint_data {
                    LintData::CanReplace(lint_data) => {
                        if let Some((_span, prev_lint_data)) = place_holder_unique_check.take() {
                            if prev_lint_data.lint.name == lint_data.lint.name && prev_lint_data.used_mod == lint_data.used_mod && prev_lint_data.replace_with == lint_data.replace_with {
                                place_holder_unique_check = Some((member.first_seg_ident_span, *lint_data));
                            } else {
                                // Will have to warn for individual entries
                                can_chunk = false;
                                break;
                            }
                        } else {
                            place_holder_unique_check = Some((member.first_seg_ident_span, *lint_data));
                        }
                    }
                    LintData::NoReplace => {
                        // Will have to warn for individual entries
                        can_chunk = false;
                        break;
                    }
                }
            }
            // If they can all be replaced with the same thing, just lint and suggest, then
            // clippy-fix works as well
            if can_chunk {
                if let Some((first_segment_ident_span, ReplaceLintData { lint, used_mod, replace_with })) = place_holder_unique_check {
                    span_lint_and_sugg(
                        cx,
                        lint,
                        first_segment_ident_span,
                        &format!("used import from `{used_mod}` instead of `{replace_with}`"),
                        &format!("consider importing the item from `{replace_with}`"),
                        replace_with.to_string(),
                        Applicability::MachineApplicable,
                    );
                }
            } else {
                for member in collected_use.members {
                    if let LintData::CanReplace(ReplaceLintData { lint, used_mod, replace_with }) = member.lint_data {
                        span_lint_hir_and_then(cx, lint, member.hir_id, member.inner, &format!("used import from `{used_mod}` instead of `{replace_with}`"), |diag| {
                            diag.help(format!("consider importing the item from `{replace_with}`"));
                        })
                    }
                }
            }
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for StdReexports {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, hir_id: HirId) {
        self.suggest_for_open_use_item_if_after(cx, path.span);
        if let Res::Def(_, def_id) = path.res
            && let Some(first_segment) = get_first_segment(path)
            && is_stable(cx, def_id)
            && !in_external_macro(cx.sess(), path.span)
            && !is_from_proc_macro(cx, &first_segment.ident)
        {
            let lint_data = match first_segment.ident.name {
                sym::std => match cx.tcx.crate_name(def_id.krate) {
                    sym::core => LintData::CanReplace(ReplaceLintData{
                        lint: STD_INSTEAD_OF_CORE,
                        used_mod: "std",
                        replace_with: "core",
                    }),
                    sym::alloc => LintData::CanReplace(ReplaceLintData{
                        lint: STD_INSTEAD_OF_ALLOC,
                        used_mod: "std",
                        replace_with: "alloc",
                    }),
                    _ => {
                        self.prev_span = first_segment.ident.span;
                        LintData::NoReplace
                    },
                },
                sym::alloc => {
                    if cx.tcx.crate_name(def_id.krate) == sym::core {
                        LintData::CanReplace(ReplaceLintData{
                            lint: ALLOC_INSTEAD_OF_CORE,
                            used_mod: "alloc",
                            replace_with: "core",
                        })
                    } else {
                        self.prev_span = first_segment.ident.span;
                        LintData::NoReplace
                    }
                },
                _ => return,
            };
            if let Some(in_use) = self.open_use.as_mut() {
                in_use.members.insert(UseSpanMember {
                    inner: path.span,
                    hir_id,
                    first_seg_ident_span: first_segment.ident.span,
                    lint_data,
                });
                return;
            }
            if let LintData::CanReplace(ReplaceLintData { lint, used_mod, replace_with }) = lint_data {
                if first_segment.ident.span != self.prev_span {
                    span_lint_and_sugg(
                        cx,
                        lint,
                        first_segment.ident.span,
                        &format!("used import from `{used_mod}` instead of `{replace_with}`"),
                        &format!("consider importing the item from `{replace_with}`"),
                        replace_with.to_string(),
                        Applicability::MachineApplicable,
                    );
                    self.prev_span = first_segment.ident.span;
                }
            }
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, item.span);
        if matches!(item.kind, ItemKind::Use(_, UseKind::ListStem)) {
            self.open_use = Some(OpenUseSpan {
                container: item.span,
                members: FxIndexSet::default(),
            })
        }

    }

    #[inline]
    fn check_item_post(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, item.span);
    }

    #[inline]
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, body.value.span);
    }

    #[inline]
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, Span::default());
    }

    #[inline]
    fn check_mod(&mut self, cx: &LateContext<'tcx>, _: &'tcx Mod<'tcx>, _: HirId) {
        self.suggest_for_open_use_item_if_after(cx, Span::default());
    }

    #[inline]
    fn check_foreign_item(&mut self, cx: &LateContext<'tcx>, f_item: &'tcx ForeignItem<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, f_item.span);
    }

    #[inline]
    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx Local<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, local.span);
    }

    #[inline]
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, block.span);
    }

    #[inline]
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, stmt.span);
    }

    #[inline]
    fn check_arm(&mut self, cx: &LateContext<'tcx>, arm: &'tcx Arm<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, arm.span);
    }

    #[inline]
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx Pat<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, pat.span);
    }

    #[inline]
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, expr.span);
    }

    #[inline]
    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &'tcx Ty<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, ty.span);
    }

    #[inline]
    fn check_generic_param(&mut self, cx: &LateContext<'tcx>, gp: &'tcx GenericParam<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, gp.span);
    }

    #[inline]
    fn check_generics(&mut self, cx: &LateContext<'tcx>, g: &'tcx Generics<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, g.span);
    }

    #[inline]
    fn check_poly_trait_ref(&mut self, cx: &LateContext<'tcx>, ptr: &'tcx PolyTraitRef<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, ptr.span);
    }

    #[inline]
    fn check_fn(&mut self, cx: &LateContext<'tcx>, _: FnKind<'tcx>, _: &'tcx FnDecl<'tcx>, _: &'tcx Body<'tcx>, s: Span, _: LocalDefId) {
        self.suggest_for_open_use_item_if_after(cx, s);
    }

    #[inline]
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, ti: &'tcx TraitItem<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, ti.span);
    }

    #[inline]
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, imp: &'tcx ImplItem<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, imp.span);
    }

    #[inline]
    fn check_struct_def(&mut self, cx: &LateContext<'tcx>, _: &'tcx VariantData<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, Span::default());
    }

    #[inline]
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, fd: &'tcx FieldDef<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, fd.span);
    }

    #[inline]
    fn check_variant(&mut self, cx: &LateContext<'tcx>, v: &'tcx Variant<'tcx>) {
        self.suggest_for_open_use_item_if_after(cx, v.span);
    }
}

/// Returns the first named segment of a [`Path`].
///
/// If this is a global path (such as `::std::fmt::Debug`), then the segment after [`kw::PathRoot`]
/// is returned.
fn get_first_segment<'tcx>(path: &Path<'tcx>) -> Option<&'tcx PathSegment<'tcx>> {
    match path.segments {
        // A global path will have PathRoot as the first segment. In this case, return the segment after.
        [x, y, ..] if x.ident.name == kw::PathRoot => Some(y),
        [x, ..] => Some(x),
        _ => None,
    }
}

/// Checks if all ancestors of `def_id` are stable, to avoid linting
/// [unstable moves](https://github.com/rust-lang/rust/pull/95956)
fn is_stable(cx: &LateContext<'_>, mut def_id: DefId) -> bool {
    loop {
        if cx
            .tcx
            .lookup_stability(def_id)
            .map_or(false, |stability| stability.is_unstable())
        {
            return false;
        }

        match cx.tcx.opt_parent(def_id) {
            Some(parent) => def_id = parent,
            None => return true,
        }
    }
}
