use clippy_utils::diagnostics::span_lint_and_then;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, AdtDef, Ty, TyCtxt, TypeAndMut};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;
use rustc_typeck::hir_ty_to_ty;

pub struct StaticItemsLargeAlign {
    pub page_size: u64,
}

declare_clippy_lint! {
    /// ### What it does
    /// Check for usage of static items which have type alignment larger than page size.
    ///
    /// ### Why is this bad?
    /// Due to some known unsound issues, the type alignment may not be fulfilled.
    /// For more information, see:
    /// <https://github.com/rust-lang/rust/issues/70022> and
    /// <https://github.com/rust-lang/rust/issues/70143>.
    ///
    /// ### Example
    /// ```rust
    /// #[repr(align(0x100000))]
    /// struct Aligned(u8);
    ///
    /// static X: Aligned = Aligned(0);         // Bad
    ///
    /// fn main() {
    ///     let x = Aligned(0);                 // Good
    ///     println!("{:#x}", &x as *const _ as usize);
    ///     println!("{:#x}", &X as *const _ as usize);
    ///     let b = Box::new(Aligned(0));       // Good
    ///     println!("{:#x}", Box::into_raw(b) as usize);
    /// }
    #[clippy::version = "1.61.0"]
    pub STATIC_ITEMS_LARGE_ALIGN,
    pedantic,
    "static items with large type alignment, which may trigger unsound problems"
}
impl_lint_pass!(StaticItemsLargeAlign => [STATIC_ITEMS_LARGE_ALIGN]);

impl LateLintPass<'_> for StaticItemsLargeAlign {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if_chain! {
            if let ItemKind::Static(hir_ty, _, _) = item.kind;
            let ty = hir_ty_to_ty(cx.tcx, hir_ty);
            let mut visited_tys = FxHashSet::default();
            let mut intermediate_tys = Vec::new();
            if let Some(la_ty) = self.check_ty_alignment(cx.tcx, ty, &mut visited_tys, &mut intermediate_tys);
            then {
                let mut span_notes: Vec<(Span, String)> = Vec::new();
                if !intermediate_tys.is_empty() {
                    let top_ty = intermediate_tys[0].ty;
                    if !top_ty.is_adt() {
                        span_notes.push((
                            hir_ty.span,
                            format!("this {} contains an inner type with large alignment", top_ty.prefix_string(cx.tcx)),
                        ));
                    }
                    intermediate_tys.iter()
                        .filter_map(|im_ty| Self::report_im_ty(cx, im_ty))
                        .for_each(|ss| span_notes.push(ss));
                }
                span_notes.push(self.report_la_ty(cx, &la_ty));

                span_lint_and_then(
                    cx,
                    STATIC_ITEMS_LARGE_ALIGN,
                    item.span,
                    "this static item (itself or its subfield) has a type alignment,\n\
                    which is larger than page size and may not be fulfilled,\n\
                    for more information, see <https://github.com/rust-lang/rust/issues/70022>.",
                    move |diag| {
                        for (span, s) in span_notes {
                            diag.span_note(span, s.as_str());
                        }
                    }
                );
            }
        }
    }
}

impl StaticItemsLargeAlign {
    /// It checks a type with the following steps:
    /// 1. Check if the type is already visited (for a static item),
    ///     if not, continue;
    ///     otherwise, return `None` early.
    /// 2. Push the type in the checked types.
    /// 3. Pick out this type if its kind is among adt, tuple, array, ref or raw ptr to them;
    ///     otherwise return `None`.
    /// 4. Check if its (or its inner fields') alignment are larger than page size.
    /// 5. Return one of them;
    ///     otherwise pop the current checked type and return `None`.
    fn check_ty_alignment<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        ty: Ty<'tcx>,
        visited_tys: &mut FxHashSet<Ty<'tcx>>,
        intermediate_tys: &mut Vec<IntermediateTy<'tcx>>,
    ) -> Option<LargeAlignmentTy<'tcx>> {
        if visited_tys.contains(&ty) {
            return None;
        }
        visited_tys.insert(ty);
        intermediate_tys.push(IntermediateTy { ty });

        let ret = match ty.kind() {
            ty::Adt(adt_def, subst_ref) => {
                if let Some(align) = adt_def.repr().align &&
                    align.bytes() > self.page_size
                {
                    intermediate_tys.pop();  // the last element is already in the return value
                    Some(LargeAlignmentTy {
                        adt: *adt_def,
                        name: ty.sort_string(tcx).into_owned(),
                        align: align.bytes(),
                    })
                } else {
                    adt_def.all_fields()
                        .map(|field_ref| field_ref.ty(tcx, subst_ref))
                        .find_map(|ty| self.check_ty_alignment(tcx, ty, visited_tys, intermediate_tys))
                }
            },
            ty::Tuple(ty_list) => ty_list
                .iter()
                .find_map(|ty| self.check_ty_alignment(tcx, ty, visited_tys, intermediate_tys)),
            ty::Array(ty, _) | ty::Ref(_, ty, _) | ty::RawPtr(TypeAndMut { ty, .. }) => {
                self.check_ty_alignment(tcx, *ty, visited_tys, intermediate_tys)
            },
            _ => None,
        };

        if ret.is_none() {
            intermediate_tys.pop();
        }
        ret
    }

    fn report_im_ty(cx: &LateContext<'_>, im_ty: &IntermediateTy<'_>) -> Option<(Span, String)> {
        let ty = im_ty.ty;
        if let ty::Adt(adt_def, substs_ref) = ty.kind() {
            Some((
                cx.tcx.def_span(adt_def.did()),
                if substs_ref.is_empty() {
                    format!("{} contains an inner type with large alignment", ty.sort_string(cx.tcx))
                } else {
                    // TODO - can we use :#?
                    format!(
                        "{} with substitutions {:#?},\n\
                        contains an inner type with large alignment",
                        ty.sort_string(cx.tcx),
                        substs_ref
                    )
                },
            ))
        } else {
            None
        }
    }

    fn report_la_ty(&self, cx: &LateContext<'_>, la_ty: &LargeAlignmentTy<'_>) -> (Span, String) {
        (
            cx.tcx.def_span(la_ty.adt.did()),
            format!(
                "{} has alignment {:#x}, which is larger than {:#x},\n\
                if you know what you are doing, config the default page size clippy uses in clippy.toml",
                la_ty.name, la_ty.align, self.page_size,
            ),
        )
    }
}

struct IntermediateTy<'tcx> {
    ty: Ty<'tcx>,
}

struct LargeAlignmentTy<'tcx> {
    adt: AdtDef<'tcx>,
    name: String,
    align: u64,
}
