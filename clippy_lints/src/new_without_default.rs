use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::source::{indent_of, reindent_multiline, snippet, snippet_with_applicability, trim_span};
use clippy_utils::sugg::DiagExt;
use clippy_utils::{is_default_equivalent_call, return_ty};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::attrs::AttributeKind;
use rustc_hir::{Attribute, HirIdMap};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{Adt, AssocKind, Ty, VariantDef};
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
    #[clippy::version = "1.88.0"]
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
    #[expect(clippy::too_many_lines)]
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'_>) {
        let hir::ItemKind::Impl(hir::Impl {
            of_trait: None,
            generics,
            self_ty: impl_self_ty,
            ..
        }) = item.kind
        else {
            return;
        };

        for assoc_item in cx
            .tcx
            .associated_items(item.owner_id.def_id)
            .filter_by_name_unhygienic(sym::new)
        {
            if let AssocKind::Fn { has_self: false, .. } = assoc_item.kind
                && let assoc_item_hir_id = cx.tcx.local_def_id_to_hir_id(assoc_item.def_id.expect_local())
                && let impl_item = cx.tcx.hir_node(assoc_item_hir_id).expect_impl_item()
                && !impl_item.span.in_external_macro(cx.sess().source_map())
                && let hir::ImplItemKind::Fn(ref sig, body_id) = impl_item.kind
                && let id = impl_item.owner_id
                // can't be implemented for unsafe new
                && !sig.header.is_unsafe()
                // shouldn't be implemented when it is hidden in docs
                && !cx.tcx.is_doc_hidden(impl_item.owner_id.def_id)
                // when the result of `new()` depends on a parameter we should not require
                // an impl of `Default`
                && impl_item.generics.params.is_empty()
                && sig.decl.inputs.is_empty()
                && let self_ty = cx.tcx.type_of(item.owner_id).instantiate_identity()
                && self_ty == return_ty(cx, impl_item.owner_id)
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

                // Check if a Default implementation exists for the Self type, regardless of
                // generics
                let default_type = if let Some(ref impling_types) = self.impling_types
                    && let self_def = cx.tcx.type_of(item.owner_id).instantiate_identity()
                    && let Some(self_def) = self_def.ty_adt_def()
                    && let Some(self_local_did) = self_def.did().as_local()
                {
                    impling_types
                        .get(&cx.tcx.local_def_id_to_hir_id(self_local_did))
                        .copied()
                } else {
                    None
                };

                match default_type {
                    Some(DefaultType::AutoDerived) => {
                        if let hir::ExprKind::Block(block, _) = cx.tcx.hir_body(body_id).value.kind
                            && !is_unit_struct(cx, self_ty)
                            && generics.params.is_empty()
                            && let Some(span) = check_block_calls_default(cx, block)
                        {
                            suggest_default_mismatch_new(cx, span, id, block, self_ty, impl_self_ty);
                        }
                    },
                    Some(DefaultType::Manual) => {
                        // both `new` and `default` are manually implemented
                    },
                    None => {
                        if !cx.effective_visibilities.is_exported(impl_item.owner_id.def_id) {
                            return;
                        }

                        let mut app = Applicability::MachineApplicable;
                        let attrs_sugg = {
                            let mut sugg = String::new();
                            for attr in cx.tcx.hir_attrs(assoc_item_hir_id) {
                                let Attribute::Parsed(AttributeKind::CfgTrace(attrs)) = attr else {
                                    // This might be some other attribute that the `impl Default` ought to inherit.
                                    // But it could also be one of the many attributes that:
                                    // - can't be put on an impl block -- like `#[inline]`
                                    // - we can't even build a suggestion for, since `Attribute::span` may panic.
                                    //
                                    // Because of all that, remain on the safer side -- don't inherit this attr, and
                                    // just reduce the applicability
                                    app = Applicability::MaybeIncorrect;
                                    continue;
                                };

                                for (_, attr_span) in attrs {
                                    sugg.push_str(&snippet_with_applicability(cx.sess(), *attr_span, "_", &mut app));
                                    sugg.push('\n');
                                }
                            }
                            sugg
                        };
                        let generics_sugg = snippet_with_applicability(cx, generics.span, "", &mut app);
                        let where_clause_sugg = if generics.has_where_clause_predicates {
                            let where_clause_sugg =
                                snippet_with_applicability(cx, generics.where_clause_span, "", &mut app).to_string();
                            let mut where_clause_sugg = reindent_multiline(&where_clause_sugg, true, Some(4));
                            if impl_item.generics.has_where_clause_predicates {
                                if !where_clause_sugg.ends_with(',') {
                                    where_clause_sugg.push(',');
                                }

                                let additional_where_preds =
                                    snippet_with_applicability(cx, impl_item.generics.where_clause_span, "", &mut app);
                                let ident = indent_of(cx, generics.where_clause_span).unwrap_or(0);
                                // Remove the leading `where ` keyword
                                let additional_where_preds =
                                    additional_where_preds.trim_start_matches("where").trim_start();
                                where_clause_sugg.push('\n');
                                where_clause_sugg.extend(std::iter::repeat_n(' ', ident));
                                where_clause_sugg.push_str(additional_where_preds);
                            }
                            format!("\n{where_clause_sugg}\n")
                        } else if impl_item.generics.has_where_clause_predicates {
                            let where_clause_sugg =
                                snippet_with_applicability(cx, impl_item.generics.where_clause_span, "", &mut app);
                            let where_clause_sugg = reindent_multiline(&where_clause_sugg, true, Some(4));
                            format!("\n{}\n", where_clause_sugg.trim_start())
                        } else {
                            String::new()
                        };
                        let self_ty_fmt = self_ty.to_string();
                        let self_type_snip = snippet_with_applicability(cx, impl_self_ty.span, &self_ty_fmt, &mut app);
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
                                    &create_new_without_default_suggest_msg(
                                        &attrs_sugg,
                                        &self_type_snip,
                                        &generics_sugg,
                                        &where_clause_sugg,
                                    ),
                                    app,
                                );
                            },
                        );
                    },
                }
            }
        }
    }
}

fn create_new_without_default_suggest_msg(
    attrs_sugg: &str,
    self_type_snip: &str,
    generics_sugg: &str,
    where_clause_sugg: &str,
) -> String {
    #[rustfmt::skip]
    format!(
"{attrs_sugg}impl{generics_sugg} Default for {self_type_snip}{where_clause_sugg} {{
    fn default() -> Self {{
        Self::new()
    }}
}}"
    )
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
        && is_default_equivalent_call(cx, callee, Some(expr))
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
        format!("`new()` may produce different values than the auto-derived `Default` for `{self_type_snip}`"),
        |diag| {
            diag.help(
                "when a type derives `Default`, users expect `new()` and `default()` to be equivalent. \
                Consider delegating to `Self::default()` for consistency, or rename `new` if the \
                behavior is intentionally different",
            );
            diag.span_suggestion(span, "use", "Self::default()", Applicability::MaybeIncorrect);
        },
    );
}
