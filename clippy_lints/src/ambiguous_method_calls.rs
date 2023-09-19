use std::sync::{Mutex, OnceLock};

use clippy_utils::diagnostics::{span_lint, span_lint_and_help};
use clippy_utils::is_trait_impl_item;
use hir::intravisit::FnKind;
use hir::{Body, FnDecl};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::def_id::LocalDefId;
use rustc_span::symbol::Ident;
use rustc_span::{Span, SpanData, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Looks for methods in trait impls and struct impls with the same name,
    /// as well as method call sites.
    ///
    /// ### Why is this bad?
    /// It is easy to accidentally override a trait method impl with a method
    /// of the same name in a struct impl. Inherent methods are preferred if
    /// the call site is unqualified, and naming conflicts will happen silently.
    ///
    /// ### Example
    /// ```rust
    /// trait MyTrait {
    ///     fn ambiguous(&self);
    /// }
    ///
    /// struct Base;
    ///
    /// impl Base {
    ///     fn ambiguous(&self) {
    ///         println!("struct impl");
    ///     }
    /// }
    ///
    /// impl MyTrait for Base {
    ///     fn ambiguous(&self) {
    ///         println!("trait impl");
    ///     }
    /// }
    ///
    /// Base.ambiguous(); // prints "struct impl"
    /// ```
    /// Use instead:
    /// ```rust
    /// trait MyTrait {
    ///     fn ambiguous(&self);
    /// }
    ///
    /// struct Base;
    ///
    /// impl Base {
    ///     fn unambiguous(&self) {
    ///         println!("struct impl");
    ///     }
    /// }
    ///
    /// impl MyTrait for Base {
    ///     fn ambiguous(&self) {
    ///         println!("trait impl");
    ///     }
    /// }
    ///
    /// Base.unambiguous(); // prints "struct impl"
    /// Base.ambiguous(); // prints "trait impl"
    /// ```
    #[clippy::version = "1.74.0"]
    pub AMBIGUOUS_METHOD_CALLS,
    pedantic,
    "declarations and calls for same-named methods in struct impls and trait impls"
}
declare_lint_pass!(AmbiguousMethodCalls => [AMBIGUOUS_METHOD_CALLS]);

impl<'tcx> LateLintPass<'tcx> for AmbiguousMethodCalls {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'_>,
        _: &'tcx Body<'_>,
        _: Span,
        def_id: LocalDefId,
    ) {
        let hir_id = cx.tcx.hir().local_def_id_to_hir_id(def_id);
        let is_trait_impl = is_trait_impl_item(cx, hir_id);

        if let FnKind::Method(ident, _) = kind {
            let parent_item = cx.tcx.hir().get_parent_item(hir_id);

            // Calling type_of on a method's default impl causes an ICE
            if let Some(hir::Node::Item(item)) = cx.tcx.hir().find(hir::HirId::from(parent_item)) {
                if let hir::ItemKind::Trait(..) = item.kind {
                    return;
                }
            }

            let parent_type = cx.tcx.type_of(parent_item.to_def_id()).skip_binder();
            let parent_ty_str = format!("{parent_type}");

            insert_method(is_trait_impl, parent_type, ident);

            if has_ambiguous_name(parent_type, ident) {
                let trait_methods = trait_methods().lock().unwrap();
                let struct_methods = struct_methods().lock().unwrap();

                span_lint(
                    cx,
                    AMBIGUOUS_METHOD_CALLS,
                    trait_methods.get(&(parent_ty_str.clone(), ident.name)).unwrap().span(),
                    "ambiguous trait method name",
                );
                span_lint_and_help(
                    cx,
                    AMBIGUOUS_METHOD_CALLS,
                    struct_methods.get(&(parent_ty_str, ident.name)).unwrap().span(),
                    "ambiguous struct method name",
                    None,
                    "consider renaming the struct impl's method",
                );
            }
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>) {
        if let hir::ExprKind::MethodCall(path, receiver, _, call_span) = &expr.kind {
            let recv_ty = cx.typeck_results().expr_ty(receiver).peel_refs();
            let recv_ty_str = format!("{recv_ty}");

            let struct_methods = struct_methods().lock().unwrap();
            let trait_methods = trait_methods().lock().unwrap();
            if struct_methods.contains_key(&(recv_ty_str.clone(), path.ident.name))
                && trait_methods.contains_key(&(recv_ty_str, path.ident.name))
            {
                span_lint_and_help(
                    cx,
                    AMBIGUOUS_METHOD_CALLS,
                    *call_span,
                    "ambiguous struct method call",
                    None,
                    "consider renaming the struct impl's method or explicitly qualifying the call site",
                );
            }
        }
    }
}

fn has_ambiguous_name(ty: Ty<'_>, ident: Ident) -> bool {
    let ty_str = format!("{ty}");
    let trait_methods = trait_methods().lock().unwrap();
    let struct_methods = struct_methods().lock().unwrap();

    trait_methods.contains_key(&(ty_str.clone(), ident.name)) && struct_methods.contains_key(&(ty_str, ident.name))
}

fn trait_methods() -> &'static Mutex<FxHashMap<(String, Symbol), SpanData>> {
    static NAMES: OnceLock<Mutex<FxHashMap<(String, Symbol), SpanData>>> = OnceLock::new();
    NAMES.get_or_init(|| Mutex::new(FxHashMap::default()))
}

fn struct_methods() -> &'static Mutex<FxHashMap<(String, Symbol), SpanData>> {
    static NAMES: OnceLock<Mutex<FxHashMap<(String, Symbol), SpanData>>> = OnceLock::new();
    NAMES.get_or_init(|| Mutex::new(FxHashMap::default()))
}

fn insert_method(is_trait_impl: bool, ty: Ty<'_>, ident: Ident) {
    let ty_str = format!("{ty}");
    let mut trait_methods = trait_methods().lock().unwrap();
    let mut struct_methods = struct_methods().lock().unwrap();

    if is_trait_impl {
        trait_methods.insert((ty_str, ident.name), ident.span.data());
    } else {
        struct_methods.insert((ty_str, ident.name), ident.span.data());
    }
}
