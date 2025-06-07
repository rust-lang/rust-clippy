use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::source::{snippet, trim_span};
use clippy_utils::sugg::DiagExt;
use clippy_utils::{is_default_equivalent_call, return_ty};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::HirIdMap;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{Adt, Ty, VariantDef};
use rustc_session::impl_lint_pass;
use rustc_span::{BytePos, Pos as _, Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for public types with a `pub fn new() -> Self` method and no
    /// implementation of
    /// [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html).
    ///
    /// ### Why is this bad?
    /// The user might expect to be able to use
    /// [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html) as the
    /// type can be constructed without arguments.
    ///
    /// ### Example
    /// ```ignore
    /// pub struct Foo(Bar);
    ///
    /// impl Foo {
    ///     pub fn new() -> Self {
    ///         Foo(Bar::new())
    ///     }
    /// }
    /// ```
    ///
    /// To fix the lint, add a `Default` implementation that delegates to `new`:
    ///
    /// ```ignore
    /// pub struct Foo(Bar);
    ///
    /// impl Default for Foo {
    ///     fn default() -> Self {
    ///         Foo::new()
    ///     }
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub NEW_WITHOUT_DEFAULT,
    style,
    "`pub fn new() -> Self` method without `Default` implementation"
}

declare_clippy_lint! {
    /// ### What it does
    /// If a type has an auto-derived `Default` trait and a `fn new() -> Self`,
    /// this lint checks if the `new()` method performs custom logic rather
    /// than simply calling the `default()` method.
    ///
    /// ### Why is this bad?
    /// Users expect the `new()` method to be equivalent to `default()`,
    /// so if the `Default` trait is auto-derived, the `new()` method should
    /// not perform custom logic.  Otherwise, there is a risk of different
    /// behavior between the two instantiation methods.
    ///
    /// ### Example
    /// ```no_run
    /// #[derive(Default)]
    /// struct MyStruct(i32);
    /// impl MyStruct {
    ///   fn new() -> Self {
    ///     Self(42)
    ///   }
    /// }
    /// ```
    ///
    /// Users are unlikely to notice that `MyStruct::new()` and `MyStruct::default()` would produce
    /// different results. The `new()` method should use auto-derived `default()` instead to be consistent:
    ///
    /// ```no_run
    /// #[derive(Default)]
    /// struct MyStruct(i32);
    /// impl MyStruct {
    ///   fn new() -> Self {
    ///     Self::default()
    ///   }
    /// }
    /// ```
    ///
    /// Alternatively, if the `new()` method requires a non-default initialization, consider renaming
    /// it to another less standard name. Lastly, if the `new()` method needs to be `const`,
    /// implement a custom `Default`:
    ///
    /// ```no_run
    /// struct MyStruct(i32);
    /// impl MyStruct {
    ///   const fn new() -> Self {
    ///     Self(42)
    ///   }
    /// }
    /// impl Default for MyStruct {
    ///   fn default() -> Self {
    ///     Self::new()
    ///   }
    /// }
    #[clippy::version = "1.86.0"]
    pub DEFAULT_MISMATCHES_NEW,
    suspicious,
    "`fn new() -> Self` method does not forward to auto-derived `Default` implementation"
}

#[derive(Debug, Clone, Copy)]
enum DefaultType {
    AutoDerived,
    Manual,
}

#[derive(Clone, Default)]
pub struct NewWithoutDefault {
    impling_types: Option<HirIdMap<DefaultType>>,
}

impl_lint_pass!(NewWithoutDefault => [NEW_WITHOUT_DEFAULT, DEFAULT_MISMATCHES_NEW]);

impl<'tcx> LateLintPass<'tcx> for NewWithoutDefault {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'_>) {
        if let hir::ItemKind::Impl(hir::Impl {
            of_trait: None,
            generics,
            self_ty: impl_self_ty,
            items,
            ..
        }) = item.kind
        {
            for assoc_item in *items {
                if assoc_item.kind == (hir::AssocItemKind::Fn { has_self: false }) {
                    let impl_item = cx.tcx.hir_impl_item(assoc_item.id);
                    if impl_item.span.in_external_macro(cx.sess().source_map()) {
                        return;
                    }
                    let hir::ImplItemKind::Fn(ref sig, body_id) = impl_item.kind else {
                        continue;
                    };

                    let name = impl_item.ident.name;
                    let id = impl_item.owner_id;
                    if sig.header.is_unsafe() {
                        // can't be implemented for unsafe new
                        return;
                    }
                    if cx.tcx.is_doc_hidden(impl_item.owner_id.def_id) {
                        // shouldn't be implemented when it is hidden in docs
                        return;
                    }
                    if !impl_item.generics.params.is_empty() {
                        // when the result of `new()` depends on a parameter we should not require
                        // an impl of `Default`
                        return;
                    }
                    if sig.decl.inputs.is_empty()
                        && name == sym::new
                        && let self_def_id = cx.tcx.hir_get_parent_item(id.into())
                        && let self_ty = cx.tcx.type_of(self_def_id).instantiate_identity()
                        && self_ty == return_ty(cx, id)
                        && let Some(default_trait_id) = cx.tcx.get_diagnostic_item(sym::Default)
                    {
                        if self.impling_types.is_none() {
                            let mut impls = HirIdMap::default();
                            for &d in cx.tcx.local_trait_impls(default_trait_id) {
                                let ty = cx.tcx.type_of(d).instantiate_identity();
                                if let Some(ty_def) = ty.ty_adt_def()
                                    && let Some(local_def_id) = ty_def.did().as_local()
                                {
                                    impls.insert(
                                        cx.tcx.local_def_id_to_hir_id(local_def_id),
                                        if cx.tcx.is_builtin_derived(d.into()) {
                                            DefaultType::AutoDerived
                                        } else {
                                            DefaultType::Manual
                                        },
                                    );
                                }
                            }
                            self.impling_types = Some(impls);
                        }

                        // Check if a Default implementation exists for the Self type, regardless of generics
                        let default_type = if let Some(ref impling_types) = self.impling_types
                            && let self_def = cx.tcx.type_of(self_def_id).instantiate_identity()
                            && let Some(self_def) = self_def.ty_adt_def()
                            && let Some(self_local_did) = self_def.did().as_local()
                        {
                            impling_types.get(&cx.tcx.local_def_id_to_hir_id(self_local_did))
                        } else {
                            None
                        };

                        match default_type {
                            Some(DefaultType::AutoDerived) => {
                                if let hir::ExprKind::Block(block, _) = cx.tcx.hir_body(body_id).value.kind
                                        && !is_unit_struct(cx, self_ty)
                                        // TODO: handle generics
                                        && generics.params.is_empty()
                                        // this type has an automatically derived `Default` implementation
                                        // check if `new` and `default` are equivalent
                                        && let Some(span) = check_block_calls_default(cx, block)
                                {
                                    suggest_default_mismatch_new(cx, span, id, block, self_ty, impl_self_ty);
                                }
                            },
                            Some(DefaultType::Manual) => {
                                // both `new` and `default` are manually implemented
                            },
                            None => {
                                // there are no `Default` implementations for this type
                                if !cx.effective_visibilities.is_reachable(impl_item.owner_id.def_id) {
                                    return;
                                }
                                suggest_new_without_default(cx, item, impl_item, id, self_ty, generics, impl_self_ty);
                            },
                        }
                    }
                }
            }
        }
    }
}

// Check if Self is a unit struct, and avoid any kind of suggestions
// FIXME: this was copied from DefaultConstructedUnitStructs,
//        and should be refactored into a common function
fn is_unit_struct(_cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    if let Adt(def, ..) = ty.kind()
        && def.is_struct()
        && let var @ VariantDef {
            ctor: Some((hir::def::CtorKind::Const, _)),
            ..
        } = def.non_enum_variant()
        && !var.is_field_list_non_exhaustive()
    {
        true
    } else {
        false
    }
}

/// Check if a block contains one of these:
/// - Empty block with an expr (e.g., `{ Self::default() }`)
/// - One statement (e.g., `{ return Self::default(); }`)
fn check_block_calls_default(cx: &LateContext<'_>, block: &hir::Block<'_>) -> Option<Span> {
    if let Some(expr) = block.expr
        && block.stmts.is_empty()
        && check_expr_call_default(cx, expr)
    {
        // Block only has a trailing expression, e.g. `Self::default()`
        return None;
    } else if let [hir::Stmt { kind, .. }] = block.stmts
        && let hir::StmtKind::Expr(expr) | hir::StmtKind::Semi(expr) = kind
        && let hir::ExprKind::Ret(Some(ret_expr)) = expr.kind
        && check_expr_call_default(cx, ret_expr)
    {
        // Block has a single statement, e.g. `return Self::default();`
        return None;
    }

    // trim first and last character, and trim spaces
    let mut span = block.span;
    span = span.with_lo(span.lo() + BytePos::from_usize(1));
    span = span.with_hi(span.hi() - BytePos::from_usize(1));
    span = trim_span(cx.sess().source_map(), span);

    Some(span)
}

/// Check for `Self::default()` call syntax or equivalent
fn check_expr_call_default(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> bool {
    if let hir::ExprKind::Call(callee, &[]) = expr.kind
        // FIXME: does this include `Self { }` style calls, which is equivalent,
        //        but not the same as `Self::default()`?
        // FIXME: what should the whole_call_expr (3rd arg) be?
        && is_default_equivalent_call(cx, callee, None)
    {
        true
    } else {
        false
    }
}

fn suggest_default_mismatch_new<'tcx>(
    cx: &LateContext<'tcx>,
    span: Span,
    id: rustc_hir::OwnerId,
    block: &rustc_hir::Block<'_>,
    self_ty: Ty<'tcx>,
    impl_self_ty: &rustc_hir::Ty<'_>,
) {
    let self_ty_fmt = self_ty.to_string();
    let self_type_snip = snippet(cx, impl_self_ty.span, &self_ty_fmt);
    span_lint_hir_and_then(
        cx,
        DEFAULT_MISMATCHES_NEW,
        id.into(),
        block.span,
        format!("consider delegating to the auto-derived `Default` for `{self_type_snip}`"),
        |diag| {
            // This would replace any comments, and we could work around the first comment,
            // but in case of a block of code with multiple statements and comment lines,
            // we can't do much.  For now, we always mark this as a MaybeIncorrect suggestion.
            diag.span_suggestion(span, "use", "Self::default()", Applicability::Unspecified);
        },
    );
}

fn suggest_new_without_default<'tcx>(
    cx: &LateContext<'tcx>,
    item: &hir::Item<'_>,
    impl_item: &hir::ImplItem<'_>,
    id: hir::OwnerId,
    self_ty: Ty<'tcx>,
    generics: &hir::Generics<'_>,
    impl_self_ty: &hir::Ty<'_>,
) {
    let generics_sugg = snippet(cx, generics.span, "");
    let where_clause_sugg = if generics.has_where_clause_predicates {
        format!("\n{}\n", snippet(cx, generics.where_clause_span, ""))
    } else {
        String::new()
    };
    let self_ty_fmt = self_ty.to_string();
    let self_type_snip = snippet(cx, impl_self_ty.span, &self_ty_fmt);
    span_lint_hir_and_then(
        cx,
        NEW_WITHOUT_DEFAULT,
        id.into(),
        impl_item.span,
        format!("you should consider adding a `Default` implementation for `{self_type_snip}`"),
        |diag| {
            diag.suggest_prepend_item(
                cx,
                item.span,
                "try adding this",
                &format!(
                    "impl{generics_sugg} Default for {self_type_snip}{where_clause_sugg} {{
    fn default() -> Self {{
        Self::new()
    }}
}}"
                ),
                Applicability::MachineApplicable,
            );
        },
    );
}
