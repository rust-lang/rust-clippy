use std::ops::ControlFlow;

use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::visitors::for_each_expr_without_closures;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::def::Res;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, ExprKind, FnDecl, HirId, Node, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::LocalDefId;
use rustc_span::{Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for functions that access all fields of a struct individually
    /// and suggests using destructuring instead.
    ///
    /// ### Why is this bad?
    /// Destructuring makes field usage explicit and ensures the compiler
    /// errors when new fields are added to the struct, helping keep code
    /// in sync with struct definitions.
    ///
    /// ### Configuration
    /// This lint has the following configuration variables:
    ///
    /// - `use-destructuring-min-fields`: The minimum number of struct fields
    ///   required for the lint to trigger (default: `3`).
    ///
    /// ### Example
    /// ```no_run
    /// use std::fmt;
    ///
    /// struct Point {
    ///     x: f32,
    ///     y: f32,
    ///     z: f32,
    /// }
    ///
    /// impl fmt::Display for Point {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         write!(f, "({}, {}, {})", self.x, self.y, self.z)
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::fmt;
    ///
    /// struct Point {
    ///     x: f32,
    ///     y: f32,
    ///     z: f32,
    /// }
    ///
    /// impl fmt::Display for Point {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         let Self { x, y, z } = self;
    ///         write!(f, "({x}, {y}, {z})")
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.89.0"]
    pub USE_DESTRUCTURING,
    pedantic,
    "accessing all fields of a struct individually instead of destructuring"
}

impl_lint_pass!(UseDestructuring => [USE_DESTRUCTURING]);

pub struct UseDestructuring {
    min_fields: u64,
}

impl UseDestructuring {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            min_fields: conf.use_destructuring_min_fields,
        }
    }
}

/// Tracks field accesses on a single local variable within a function body.
struct LocalInfo<'tcx> {
    /// The type of the local after peeling references (i.e. the struct type).
    ty: Ty<'tcx>,

    /// The set of field names (or tuple indices) accessed on this local.
    accessed_fields: FxHashSet<Symbol>,

    /// Span of the first field access, used for the lint diagnostic.
    first_field_access_span: Span,
}

impl<'tcx> LateLintPass<'tcx> for UseDestructuring {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        _span: Span,
        _def_id: LocalDefId,
    ) {
        let typeck_results = cx.typeck_results();

        // Map from local HirId -> info about field accesses on that local
        let mut locals: FxHashMap<HirId, LocalInfo<'tcx>> = FxHashMap::default();

        // Locals that are used in non-field-access contexts (e.g. passed to a function)
        let mut non_field_uses: FxHashSet<HirId> = FxHashSet::default();

        // Locals that have field mutations (e.g. self.x = 5)
        let mut mutated_locals: FxHashSet<HirId> = FxHashSet::default();

        for_each_expr_without_closures(body.value, |expr| {
            match expr.kind {
                ExprKind::Field(base, ident) => {
                    // Check if the base is a direct local variable reference
                    if let ExprKind::Path(QPath::Resolved(_, path)) = base.kind
                        && let Res::Local(local_hir_id) = path.res
                        && !expr.span.from_expansion()
                    {
                        let base_ty = typeck_results.expr_ty_adjusted(base).peel_refs();
                        let info = locals.entry(local_hir_id).or_insert_with(|| LocalInfo {
                            ty: base_ty,
                            accessed_fields: FxHashSet::default(),
                            first_field_access_span: expr.span,
                        });
                        info.accessed_fields.insert(ident.name);

                        // Check if this field access is on the LHS of an assignment
                        if let Node::Expr(parent) = cx.tcx.parent_hir_node(expr.hir_id) {
                            let is_assign_target = matches!(
                                parent.kind,
                                ExprKind::Assign(lhs, _, _)
                                    | ExprKind::AssignOp(_, lhs, _)
                                    if lhs.hir_id == expr.hir_id
                            );
                            if is_assign_target {
                                mutated_locals.insert(local_hir_id);
                            }
                        }
                    }
                },
                ExprKind::Path(QPath::Resolved(_, path)) => {
                    // A direct use of a local variable (not as part of field access)
                    if let Res::Local(local_hir_id) = path.res {
                        // Check if the parent expression is a field access on this local.
                        // If so, this is already handled above — not a "non-field use".
                        let is_field_base = matches!(
                            cx.tcx.parent_hir_node(expr.hir_id),
                            Node::Expr(parent_expr) if matches!(parent_expr.kind, ExprKind::Field(_, _))
                        );
                        if !is_field_base {
                            non_field_uses.insert(local_hir_id);
                        }
                    }
                },
                _ => {},
            }
            ControlFlow::<!, _>::Continue(())
        });

        #[expect(rustc::potential_query_instability)] // order-independent
        for (local_hir_id, info) in &locals {
            if non_field_uses.contains(local_hir_id) || mutated_locals.contains(local_hir_id) {
                continue;
            }
            self.lint_local(cx, *local_hir_id, info);
        }
    }
}

impl UseDestructuring {
    fn lint_local<'tcx>(&self, cx: &LateContext<'tcx>, local_hir_id: HirId, info: &LocalInfo<'tcx>) {
        let ty::Adt(adt_def, _) = info.ty.kind() else {
            return;
        };

        if !adt_def.is_struct() {
            return;
        }

        let variant = adt_def.non_enum_variant();

        if variant.is_field_list_non_exhaustive() {
            return;
        }

        let struct_fields = &variant.fields;

        if (struct_fields.len() as u64) < self.min_fields {
            return;
        }

        // Check that ALL fields are accessed
        // TODO(emilk): future improvement is to emit this lint if the _majority_
        // of fields are accessed. This could catch bugs, where the writer forgot about a field!
        // But the cutoff for this needs to be configurable in `clippy.toml`
        if info.accessed_fields.len() != struct_fields.len() {
            return;
        }

        if !struct_fields.iter().all(|f| info.accessed_fields.contains(&f.name)) {
            return;
        }

        // Build the suggestion string
        let type_name = cx.tcx.item_name(adt_def.did());

        let var_name = if let Node::Pat(pat) = cx.tcx.hir_node(local_hir_id)
            && let Some(ident) = pat.simple_ident()
        {
            ident.name.to_string()
        } else {
            "..".to_string()
        };

        let is_tuple_struct = variant.ctor.is_some();
        let help_msg = if is_tuple_struct {
            let binding_names: Vec<String> = (0..struct_fields.len()).map(|i| format!("field_{i}")).collect();
            let bindings_str = binding_names.join(", ");
            format!("consider using `let {type_name}({bindings_str}) = {var_name};`")
        } else {
            let field_names: Vec<&str> = struct_fields.iter().map(|f| f.name.as_str()).collect();
            let fields_str = field_names.join(", ");
            format!("consider using `let {type_name} {{ {fields_str} }} = {var_name};`")
        };

        span_lint_and_help(
            cx,
            USE_DESTRUCTURING,
            info.first_field_access_span,
            format!(
                "all {} fields of `{type_name}` are accessed individually; consider destructuring",
                struct_fields.len()
            ),
            None,
            help_msg,
        );
    }
}
