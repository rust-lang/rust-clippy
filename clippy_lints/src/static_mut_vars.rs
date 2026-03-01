use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{IntoSpan, SpanRangeExt, snippet};
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::ty_from_hir_ty;
use rustc_ast::{IntTy, UintTy};
use rustc_errors::Applicability;
use rustc_hir::{Item, ItemKind, MutTy, Mutability, Ty, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `static mut` variables.
    ///
    /// ### Why is this bad?
    /// `static mut` allows mutable global state without synchronization.
    /// Accessing it is `unsafe` because it can cause:
    ///
    /// * Data races in multithreaded contexts
    /// * Undefined behavior
    /// * Violations of Rust’s aliasing guarantees
    /// * Hard-to-debug global state bugs
    ///
    /// Also, since Rust 2024, the built-in lint `static_mut_refs` denies by default `&` or `&mut`
    /// references to a `static mut`.
    ///
    /// ### Alternatives
    /// Prefer one of:
    /// * `Atomic*` for integer types, booleans and pointers
    /// * `Mutex<T>` or `RwLock<T>` synchronization primitives for interior mutability
    /// * `OnceLock<T>` or `LazyLock<T>` for one-time initialization
    /// * `UnsafeCell<T>` with wrapper implementing `Sync` (or nightly `SyncUnsafeCell<T>`)
    ///
    /// ### Example
    /// ```no_run
    /// static mut A: i32 = 0i32;
    /// static mut B: (u64, u64) = (0u64, 0u64);
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::sync::atomic::AtomicI32;
    /// use std::sync::RwLock;
    ///
    /// static A: AtomicI32 = AtomicI32::new(0i32);
    /// static B: RwLock<(u64, u64)> = RwLock::new((0u64, 0u64));
    /// ```
    #[clippy::version = "1.95.0"]
    pub STATIC_MUT_VARS,
    correctness,
    "using a static mutable variable"
}
declare_lint_pass!(StaticMutVars => [STATIC_MUT_VARS]);

enum Replacement<'tcx> {
    AtomicPrimTy(&'static str),
    AtomicPtr(&'tcx Ty<'tcx>),
}

impl<'tcx> LateLintPass<'tcx> for StaticMutVars {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if let ItemKind::Static(Mutability::Mut, ref ident, ty, body_id) = item.kind {
            span_lint_and_then(
                cx,
                STATIC_MUT_VARS,
                item.span,
                format!("static `{}` is declared as mutable", ident.name),
                |diag| match replacement(cx, ty) {
                    Some(repl) => {
                        let mut applicability = Applicability::MaybeIncorrect;

                        let body = cx.tcx.hir_body(body_id).value.peel_blocks();
                        let arg = Sugg::hir_with_context(cx, body, item.span.ctxt(), "_", &mut applicability);

                        let (sugg_ty, sugg_body) = match repl {
                            Replacement::AtomicPrimTy(ty) => (
                                format!("std::sync::atomic::{ty}"),
                                format!("std::sync::atomic::{ty}::new({arg})"),
                            ),

                            Replacement::AtomicPtr(ty) => {
                                let ty_snippet = snippet(cx, ty.span, "..");
                                (
                                    format!("std::sync::atomic::AtomicPtr<{ty_snippet}>"),
                                    format!("std::sync::atomic::AtomicPtr::<{ty_snippet}>::new({arg})"),
                                )
                            },
                        };

                        diag.multipart_suggestion(
                            "try",
                            vec![
                                (
                                    item.span.until(ident.span.with_leading_whitespace(cx).into_span()),
                                    "static".to_string(),
                                ),
                                (ty.span, sugg_ty),
                                (body.span, sugg_body),
                            ],
                            applicability,
                        );
                    },

                    None => {
                        diag.help("consider using a safer alternative");
                    },
                },
            );
        }
    }
}

fn replacement<'tcx>(cx: &LateContext<'tcx>, hir_ty: &'tcx Ty<'tcx>) -> Option<Replacement<'tcx>> {
    let mid_ty = ty_from_hir_ty(cx, hir_ty);

    match mid_ty.kind() {
        ty::Int(int_ty) => match int_ty {
            IntTy::Isize => Some(Replacement::AtomicPrimTy("AtomicIsize")),
            IntTy::I8 => Some(Replacement::AtomicPrimTy("AtomicI8")),
            IntTy::I16 => Some(Replacement::AtomicPrimTy("AtomicI16")),
            IntTy::I32 => Some(Replacement::AtomicPrimTy("AtomicI32")),
            IntTy::I64 => Some(Replacement::AtomicPrimTy("AtomicI64")),
            IntTy::I128 => None,
        },

        ty::Uint(uint_ty) => match uint_ty {
            UintTy::Usize => Some(Replacement::AtomicPrimTy("AtomicUsize")),
            UintTy::U8 => Some(Replacement::AtomicPrimTy("AtomicU8")),
            UintTy::U16 => Some(Replacement::AtomicPrimTy("AtomicU16")),
            UintTy::U32 => Some(Replacement::AtomicPrimTy("AtomicU32")),
            UintTy::U64 => Some(Replacement::AtomicPrimTy("AtomicU64")),
            UintTy::U128 => None,
        },

        ty::Bool => Some(Replacement::AtomicPrimTy("AtomicBool")),

        ty::RawPtr(_, Mutability::Mut) => match hir_ty.kind {
            TyKind::Ptr(MutTy { ty: ptr_ty, .. }) => Some(Replacement::AtomicPtr(ptr_ty)),
            _ => None,
        },

        _ => None,
    }
}
