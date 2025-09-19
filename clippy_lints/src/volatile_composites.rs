use clippy_utils::diagnostics::span_lint;
use clippy_utils::sym;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// This lint warns when volatile load/store operations
    /// (`write_volatile`/`read_volatile`) are applied to composite types.
    ///
    /// ### Why is this bad?
    ///
    /// Volatile operations are typically used with memory mapped IO devices,
    /// where the precise number and ordering of load and store instructions is
    /// important because they can have side effects. This is well defined for
    /// primitive types like `u32`, but less well defined for structures and
    /// other composite types. In practice it's implementation defined, and the
    /// behavior can be rustc-version dependent.
    ///
    /// As a result, code should only apply `write_volatile`/`read_volatile` to
    /// primitive types to be fully well-defined.
    ///
    /// ### Example
    /// ```no_run
    /// struct MyDevice {
    ///     addr: usize,
    ///     count: usize
    /// }
    ///
    /// fn start_device(device: *mut MyDevice, addr: usize, count: usize) {
    ///     unsafe {
    ///         device.write_volatile(MyDevice { addr, count });
    ///     }
    /// }
    /// ```
    /// Instead, operate on each primtive field individually:
    /// ```no_run
    /// struct MyDevice {
    ///     addr: usize,
    ///     count: usize
    /// }
    ///
    /// fn start_device(device: *mut MyDevice, addr: usize, count: usize) {
    ///     unsafe {
    ///         (&raw mut (*device).addr).write_volatile(addr);
    ///         (&raw mut (*device).count).write_volatile(count);
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.91.0"]
    pub VOLATILE_COMPOSITES,
    nursery,
    "warn about volatile read/write applied to composite types"
}
declare_lint_pass!(VolatileComposites => [VOLATILE_COMPOSITES]);

// functions:
// core::ptr::{read_volatile,write_volatile}
// methods:
// pointer::{read_volatile,write_volatile}
// NonNull::{read_volatile,write_volatile}

// primitive type:
// unit, [iu]{8,16,32,64,128?}, f{32,64}, thin pointer, usize, isize, bool, char
// C enum with primitive repr
// #[repr(transparent)] wrapper of above

// Zero-sized types are intrinsically safe to use volatile on since they won't
// actually generate *any* loads or stores. But this is also used to skip zero
// fields of #[repr(transparent)] structures.
fn is_zero_sized_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let Ok(ty) = cx.tcx.try_normalize_erasing_regions(cx.typing_env(), ty)
        && let Ok(layout) = cx.tcx.layout_of(cx.typing_env().as_query_input(ty))
    {
        layout.layout.size().bytes() == 0
    } else {
        false
    }
}

// Make sure the raw pointer has no metadata
fn is_narrow_raw_ptr<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let ty::RawPtr(_inner, _) = ty.kind() {
        ty.pointee_metadata_ty_or_projection(cx.tcx).is_unit()
    } else {
        false
    }
}

// Enum with some fixed representation and no data-carrying variants
fn is_enum_repr_c<'tcx>(_cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let ty::Adt(adt_def, _args) = ty.kind()
        && adt_def.is_enum()
        && adt_def.repr().inhibit_struct_field_reordering()
    {
        adt_def.is_payloadfree()
    } else {
        false
    }
}

// #[repr(transparent)] structures are also OK if the only non-zero
// sized field contains a volatile-safe type
fn is_struct_repr_transparent<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let ty::Adt(adt_def, args) = ty.kind()
        && adt_def.is_struct()
        && adt_def.repr().transparent()
        && let [fieldty] = adt_def
            .all_fields()
            .filter_map(|field| {
                let fty = field.ty(cx.tcx, args);
                if is_zero_sized_ty(cx, fty) { None } else { Some(fty) }
            })
            .collect::<Vec<_>>()
            .as_slice()
    {
        is_volatile_safe_ty(cx, *fieldty)
    } else {
        false
    }
}

// SIMD can be useful to get larger atomic loads/stores, though this is still
// pretty machine-dependent.
fn is_simd_repr<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    if let ty::Adt(adt_def, _args) = ty.kind()
        && adt_def.is_struct()
        && adt_def.repr().simd()
    {
        let (_size, simdty) = ty.simd_size_and_type(cx.tcx);
        is_volatile_safe_ty(cx, simdty)
    } else {
        false
    }
}

// We can't know about a generic type, so just let it pass to avoid noise
fn is_generic<'tcx>(_cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.flags().intersects(ty::TypeFlags::HAS_PARAM)
}

fn is_volatile_safe_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.is_primitive()
        || is_narrow_raw_ptr(cx, ty)
        || is_zero_sized_ty(cx, ty)
        || is_enum_repr_c(cx, ty)
        || is_simd_repr(cx, ty)
        || is_struct_repr_transparent(cx, ty)
        || is_generic(cx, ty)
}

fn report_volatile_safe<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, ty: Ty<'tcx>) {
    if !is_volatile_safe_ty(cx, ty) {
        span_lint(
            cx,
            VOLATILE_COMPOSITES,
            expr.span,
            format!("type `{ty}` is not volatile-compatible"),
        );
    }
}

impl<'tcx> LateLintPass<'tcx> for VolatileComposites {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        // Check our expr is calling a method with pattern matching
        match expr.kind {
            // Look for method calls to `write_volatile`/`read_volatile`, which
            // apply to both raw pointers and std::ptr::NonNull.
            ExprKind::MethodCall(name, self_arg, _, _)
                if matches!(name.ident.name, sym::read_volatile | sym::write_volatile) =>
            {
                let self_ty = cx.typeck_results().expr_ty(self_arg);
                match self_ty.kind() {
                    // Raw pointers
                    ty::RawPtr(innerty, _) => report_volatile_safe(cx, expr, *innerty),
                    // std::ptr::NonNull
                    ty::Adt(adt_def, args) if cx.tcx.is_diagnostic_item(sym::NonNull, adt_def.did()) => {
                        report_volatile_safe(cx, expr, args.type_at(0));
                    },
                    _ => (),
                }
            },

            // Also plain function calls to std::ptr::{read,write}_volatile
            ExprKind::Call(func, [arg_ptr, ..]) => {
                if let ExprKind::Path(ref qpath) = func.kind
                    && let Some(def_id) = cx.qpath_res(qpath, func.hir_id).opt_def_id()
                    && matches!(
                        cx.tcx.get_diagnostic_name(def_id),
                        Some(sym::ptr_read_volatile | sym::ptr_write_volatile)
                    )
                    && let ty::RawPtr(ptrty, _) = cx.typeck_results().expr_ty(arg_ptr).kind()
                {
                    report_volatile_safe(cx, expr, *ptrty);
                }
            },
            _ => {},
        }
    }
}
