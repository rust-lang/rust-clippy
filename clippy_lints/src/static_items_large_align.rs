use clippy_utils::diagnostics::span_lint_and_note;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, AdtDef, Ty, TyCtxt};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_target::abi::Align;
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
            if let Some(adt_ref) = self.check_ty_alignment(cx.tcx, ty);
            let def_span = cx.tcx.def_span(adt_ref.did());
            then {
                span_lint_and_note(
                    cx,
                    STATIC_ITEMS_LARGE_ALIGN,
                    item.span,
                    "this static item (itself or its subfield) has large type alignment, which may not be fulfilled,\n\
                    for more information, see <https://github.com/rust-lang/rust/issues/70022>",
                    Some(def_span),
                    format!("this type has an alignment larger than page size ({}KB)", self.page_size/1024).as_str()
                );
            }
        }
    }
}

impl StaticItemsLargeAlign {
    /// It checks a type with the following steps:
    /// 1. picks out this type if its kind is among adt, array, tuple or ref to them;
    ///    otherwise return None
    /// 2. check if its (or its inner fields') alignment are larger than page size
    /// 3. return one of them;
    ///    otherwise return None
    fn check_ty_alignment<'tcx>(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<AdtDef<'tcx>> {
        match ty.kind() {
            ty::Adt(adt_ref, subst_ref) => {
                if let Some(align) = adt_ref.repr().align &&
                    align > Align::from_bytes(self.page_size).unwrap()
                {
                    Some(*adt_ref)
                } else {
                    adt_ref.all_fields()
                        .map(|field_ref| field_ref.ty(tcx, subst_ref))
                        .find_map(|ty| self.check_ty_alignment(tcx, ty))
                }
            },
            ty::Array(ty, _) => self.check_ty_alignment(tcx, *ty),
            ty::Tuple(ty_list) => ty_list.iter()
                .find_map(|ty| self.check_ty_alignment(tcx, ty)),
            ty::Ref(region, ty, _) if region.is_static() => self.check_ty_alignment(tcx, *ty),
            _ => None,
        }
    }
}
