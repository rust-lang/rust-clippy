use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_opt;
use rustc_errors::Applicability;
use rustc_hir::{FnSig, Item, ItemKind, Ty};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::{sym, BytePos, Pos};
use rustc_target::spec::abi::Abi;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for Rust ABI functions or statics with the `#[no_mangle]` attribute.
    ///
    /// ### Why is this bad?
    /// The Rust ABI is not stable, but in many simple cases matches enough with the C ABI
    /// that it is possible to forget to add `extern "C"` to a function called from C.
    /// Changes to the Rust ABI can break this at any point.
    ///
    /// ### Example
    /// ```rust
    /// #[no_mangle]
    /// fn example(arg_one: u32, arg_two: usize) {}
    ///
    /// pub struct UsingMeInCIsUB(u32, u32);
    /// #[no_mangle]
    /// pub static ZERO: UsingMeInCIsUB = UsingMeInCIsUB(0, 0);
    /// ```
    /// Use instead:
    /// ```rust
    /// #[no_mangle]
    /// extern "C" fn example(arg_one: u32, arg_two: usize) {}
    ///
    /// #[repr(C)]
    /// pub struct UsingMeInCIsFine(u32, u32);
    /// #[no_mangle]
    /// pub static ZERO: UsingMeInCIsFine = UsingMeInCIsFine(0, 0);
    /// ```
    #[clippy::version = "1.69.0"]
    pub NO_MANGLE_WITH_RUST_ABI,
    pedantic,
    "convert Rust ABI functions to C ABI"
}
declare_lint_pass!(NoMangleWithRustAbi => [NO_MANGLE_WITH_RUST_ABI]);

impl<'tcx> LateLintPass<'tcx> for NoMangleWithRustAbi {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        if cx.tcx.get_attr(item.owner_id, sym::no_mangle).is_none() {
            return;
        }

        match item.kind {
            ItemKind::Fn(fn_sig, _, _) => check_fn(cx, fn_sig),
            ItemKind::Static(ty, _, _) => check_static(cx, item, ty),
            _ => {},
        }
    }
}

/// Check for functions that are implicitly using the Rust ABI.
fn check_fn(cx: &LateContext<'_>, fn_sig: FnSig<'_>) {
    if fn_sig.header.abi == Abi::Rust
        && let Some(snippet) = snippet_opt(cx, fn_sig.span)
        && let Some((fn_attrs, _)) = snippet.split_once("fn")
        && !fn_attrs.contains("extern")
    {
        let sugg_span = fn_sig
            .span
            .with_lo(fn_sig.span.lo() + BytePos::from_usize(fn_attrs.len()))
            .shrink_to_lo();

        span_lint_and_then(
            cx,
            NO_MANGLE_WITH_RUST_ABI,
            fn_sig.span,
            "`#[no_mangle]` set on a function with the default (`Rust`) ABI",
            |diag| {
                diag.note_once(
                    "this function's calling convention (ABI) is unstable, and changes to the `Rust` ABI can break \
                     this at any time",
                )
                .span_suggestion(sugg_span, "set an ABI", "extern \"C\" ", Applicability::MaybeIncorrect)
                .span_suggestion(
                    sugg_span,
                    "or explicitly set the default",
                    "extern \"Rust\" ",
                    Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

/// Check for static items with a type that is implicitly using the Rust ABI.
fn check_static(cx: &LateContext<'_>, item: &Item<'_>, ty: &Ty<'_>) {
    // TODO(Centri3): Add a check here for `#[repr(Rust)]` once #114201 is merged.

    if let Some(is_local) = match cx.tcx.type_of(item.owner_id).skip_binder().kind() {
        ty::Adt(def, _) => {
            let repr = def.repr();
            (!repr.c() && !repr.transparent() && !repr.simd() && repr.int.is_none()).then(|| def.did().is_local())
        },
        ty::Tuple(tys) => (tys.len() != 0).then_some(false),
        _ => return,
    } {
        span_lint_and_then(
            cx,
            NO_MANGLE_WITH_RUST_ABI,
            item.span,
            "`#[no_mangle]` set on a static with the default (`Rust`) ABI",
            |diag| {
                diag.span_note(ty.span, "this type is implicitly `#[repr(Rust)]`")
                    .note_once(
                        "this static's layout (ABI) is unstable, and changes to the `Rust` ABI can break this at any \
                        time",
                    );

                if is_local {
                    diag.help("set an explicit ABI (like `#[repr(C)]`) on the type's definition");
                }
            },
        );
    }
}
