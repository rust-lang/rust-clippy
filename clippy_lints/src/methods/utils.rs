use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::{get_parent_expr, usage};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{BorrowKind, Expr, ExprKind, HirId, Mutability, Pat, PatKind, Path, QPath, Stmt, StmtKind};
use rustc_lint::LateContext;
use rustc_middle::hir::nested_filter;
use rustc_middle::ty::{self, GenericArgKind, Ty};
use rustc_span::Span;
use rustc_span::symbol::{Ident, sym};
use std::iter;
use std::ops::Not;

/// Checks if `expr`, of type `ty`, corresponds to a slice or can be dereferenced to a slice, or if
/// `expr` is a method call to `.iter()` on such a type. In these cases, return the slice-like
/// expression.
pub(super) fn derefs_to_slice<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    ty: Ty<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    fn may_slice<'a>(cx: &LateContext<'a>, ty: Ty<'a>) -> bool {
        match ty.kind() {
            ty::Slice(_) => true,
            ty::Adt(..) if let Some(boxed) = ty.boxed_ty() => may_slice(cx, boxed),
            ty::Adt(..) => ty.is_diag_item(cx, sym::Vec),
            ty::Array(_, size) => size.try_to_target_usize(cx.tcx).is_some(),
            ty::Ref(_, inner, _) => may_slice(cx, *inner),
            _ => false,
        }
    }

    if let ExprKind::MethodCall(path, self_arg, ..) = &expr.kind {
        if path.ident.name == sym::iter && may_slice(cx, cx.typeck_results().expr_ty(self_arg)) {
            Some(self_arg)
        } else {
            None
        }
    } else {
        match ty.kind() {
            ty::Slice(_) => Some(expr),
            _ if ty.boxed_ty().is_some_and(|boxed| may_slice(cx, boxed)) => Some(expr),
            ty::Ref(_, inner, _) => {
                if may_slice(cx, *inner) {
                    Some(expr)
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}

/// The core logic of `check_for_loop_iter` in `unnecessary_iter_cloned.rs`, this function wraps a
/// use of `CloneOrCopyVisitor`.
pub(super) fn clone_or_copy_needed<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &Pat<'tcx>,
    body: &'tcx Expr<'tcx>,
) -> (bool, Vec<(Span, String)>) {
    let mut visitor = CloneOrCopyVisitor {
        cx,
        binding_hir_ids: pat_bindings(pat),
        clone_or_copy_needed: false,
        references_to_binding: Vec::new(),
    };
    visitor.visit_expr(body);
    (visitor.clone_or_copy_needed, visitor.references_to_binding)
}

/// Returns a vector of all `HirId`s bound by the pattern.
fn pat_bindings(pat: &Pat<'_>) -> Vec<HirId> {
    let mut collector = usage::ParamBindingIdCollector {
        binding_hir_ids: Vec::new(),
    };
    collector.visit_pat(pat);
    collector.binding_hir_ids
}

/// `clone_or_copy_needed` will be false when `CloneOrCopyVisitor` is done visiting if the only
/// operations performed on `binding_hir_ids` are:
/// * to take non-mutable references to them
/// * to use them as non-mutable `&self` in method calls
///
/// If any of `binding_hir_ids` is used in any other way, then `clone_or_copy_needed` will be true
/// when `CloneOrCopyVisitor` is done visiting.
struct CloneOrCopyVisitor<'cx, 'tcx> {
    cx: &'cx LateContext<'tcx>,
    binding_hir_ids: Vec<HirId>,
    clone_or_copy_needed: bool,
    references_to_binding: Vec<(Span, String)>,
}

impl<'tcx> Visitor<'tcx> for CloneOrCopyVisitor<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        walk_expr(self, expr);
        if self.is_binding(expr) {
            if let Some(parent) = get_parent_expr(self.cx, expr) {
                match parent.kind {
                    ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, referent) => {
                        if !parent.span.from_expansion() {
                            self.references_to_binding
                                .push((parent.span.until(referent.span), String::new()));
                        }
                        return;
                    },
                    ExprKind::MethodCall(.., args, _) => {
                        if args.iter().all(|arg| !self.is_binding(arg))
                            && let Some(method_def_id) = self.cx.typeck_results().type_dependent_def_id(parent.hir_id)
                            && let method_ty = self
                                .cx
                                .tcx
                                .type_of(method_def_id)
                                .instantiate_identity()
                                .skip_norm_wip()
                            && let self_ty = method_ty.fn_sig(self.cx.tcx).input(0).skip_binder()
                            && matches!(self_ty.kind(), ty::Ref(_, _, Mutability::Not))
                        {
                            return;
                        }
                    },
                    _ => {},
                }
            }
            self.clone_or_copy_needed = true;
        }
    }
}

impl<'tcx> CloneOrCopyVisitor<'_, 'tcx> {
    fn is_binding(&self, expr: &Expr<'tcx>) -> bool {
        self.binding_hir_ids
            .iter()
            .any(|&hir_id| expr.res_local_id() == Some(hir_id))
    }
}

pub(super) fn get_last_chain_binding_hir_id(mut hir_id: HirId, statements: &[Stmt<'_>]) -> Option<HirId> {
    for stmt in statements {
        if let StmtKind::Let(local) = stmt.kind
            && let Some(init) = local.init
            && let ExprKind::Path(QPath::Resolved(_, path)) = init.kind
            && let rustc_hir::def::Res::Local(local_hir_id) = path.res
            && local_hir_id == hir_id
        {
            hir_id = local.pat.hir_id;
        } else {
            return None;
        }
    }
    Some(hir_id)
}

// Core logic for `unnecessary_sort_by.rs` and `unnecessary_dedup_by.rs`
/// Detect if the two expressions are mirrored (identical, except one
/// contains a and the other replaces it with b)
pub(super) fn mirrored_exprs(
    a_expr: &Expr<'_>,
    b_expr: &Expr<'_>,
    binding_map: &BindingMap,
    binding_source: BindingSource,
) -> bool {
    match (a_expr.kind, b_expr.kind) {
        // Two arrays with mirrored contents
        (ExprKind::Array(left_exprs), ExprKind::Array(right_exprs)) => iter::zip(left_exprs, right_exprs)
            .all(|(left, right)| mirrored_exprs(left, right, binding_map, binding_source)),
        // The two exprs are function calls.
        // Check to see that the function itself and its arguments are mirrored
        (ExprKind::Call(left_expr, left_args), ExprKind::Call(right_expr, right_args)) => {
            mirrored_exprs(left_expr, right_expr, binding_map, binding_source)
                && iter::zip(left_args, right_args)
                    .all(|(left, right)| mirrored_exprs(left, right, binding_map, binding_source))
        },
        // The two exprs are method calls.
        // Check to see that the function is the same and the arguments and receivers are mirrored
        (
            ExprKind::MethodCall(left_segment, left_receiver, left_args, _),
            ExprKind::MethodCall(right_segment, right_receiver, right_args, _),
        ) => {
            left_segment.ident == right_segment.ident
                && iter::zip(left_args, right_args)
                    .all(|(left, right)| mirrored_exprs(left, right, binding_map, binding_source))
                && mirrored_exprs(left_receiver, right_receiver, binding_map, binding_source)
        },
        // Two tuples with mirrored contents
        (ExprKind::Tup(left_exprs), ExprKind::Tup(right_exprs)) => iter::zip(left_exprs, right_exprs)
            .all(|(left, right)| mirrored_exprs(left, right, binding_map, binding_source)),
        // Two binary ops, which are the same operation and which have mirrored arguments
        (ExprKind::Binary(left_op, left_left, left_right), ExprKind::Binary(right_op, right_left, right_right)) => {
            left_op.node == right_op.node
                && mirrored_exprs(left_left, right_left, binding_map, binding_source)
                && mirrored_exprs(left_right, right_right, binding_map, binding_source)
        },
        // Two unary ops, which are the same operation and which have the same argument
        (ExprKind::Unary(left_op, left_expr), ExprKind::Unary(right_op, right_expr)) => {
            left_op == right_op && mirrored_exprs(left_expr, right_expr, binding_map, binding_source)
        },
        // The two exprs are literals of some kind
        (ExprKind::Lit(left_lit), ExprKind::Lit(right_lit)) => left_lit.node == right_lit.node,
        (ExprKind::Cast(left, _), ExprKind::Cast(right, _)) => mirrored_exprs(left, right, binding_map, binding_source),
        (ExprKind::DropTemps(left_block), ExprKind::DropTemps(right_block)) => {
            mirrored_exprs(left_block, right_block, binding_map, binding_source)
        },
        (ExprKind::Field(left_expr, left_ident), ExprKind::Field(right_expr, right_ident)) => {
            left_ident.name == right_ident.name && mirrored_exprs(left_expr, right_expr, binding_map, binding_source)
        },
        // Two paths: either one is a and the other is b, or they're identical to each other
        (
            ExprKind::Path(QPath::Resolved(
                _,
                &Path {
                    segments: left_segments,
                    ..
                },
            )),
            ExprKind::Path(QPath::Resolved(
                _,
                &Path {
                    segments: right_segments,
                    ..
                },
            )),
        ) => {
            (iter::zip(left_segments, right_segments).all(|(left, right)| left.ident == right.ident)
                && left_segments.iter().all(|seg| {
                    !binding_map.contains_key(&BindingKey {
                        ident: seg.ident,
                        source: BindingSource::Left,
                    }) && !binding_map.contains_key(&BindingKey {
                        ident: seg.ident,
                        source: BindingSource::Right,
                    })
                }))
                || (left_segments.len() == 1
                    && right_segments.len() == 1
                    && binding_map
                        .get(&BindingKey {
                            ident: left_segments[0].ident,
                            source: binding_source,
                        })
                        .is_some_and(|value| value.mirrored.ident == right_segments[0].ident))
        },
        // Matching expressions, but one or both is borrowed
        (
            ExprKind::AddrOf(left_kind, Mutability::Not, left_expr),
            ExprKind::AddrOf(right_kind, Mutability::Not, right_expr),
        ) => left_kind == right_kind && mirrored_exprs(left_expr, right_expr, binding_map, binding_source),
        (_, ExprKind::AddrOf(_, Mutability::Not, right_expr)) => {
            mirrored_exprs(a_expr, right_expr, binding_map, binding_source)
        },
        (ExprKind::AddrOf(_, Mutability::Not, left_expr), _) => {
            mirrored_exprs(left_expr, b_expr, binding_map, binding_source)
        },
        _ => false,
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(super) enum BindingSource {
    Left,
    Right,
}

impl Not for BindingSource {
    type Output = BindingSource;

    fn not(self) -> Self::Output {
        match self {
            BindingSource::Left => BindingSource::Right,
            BindingSource::Right => BindingSource::Left,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct BindingKey {
    /// The identifier of the binding.
    pub ident: Ident,
    /// The source of the binding.
    pub source: BindingSource,
}

pub(super) struct BindingValue {
    /// The mirrored binding.
    pub mirrored: BindingKey,
    /// The number of refs the binding is wrapped in.
    pub n_refs: usize,
}

/// A map from binding info to the number of refs the binding is wrapped in.
pub(super) type BindingMap = FxHashMap<BindingKey, BindingValue>;
/// Extract the binding pairs, if the two patterns are mirrored. The pats are assumed to be used in
/// closure inputs and thus irrefutable.
pub(super) fn mapping_of_mirrored_pats(a_pat: &Pat<'_>, b_pat: &Pat<'_>) -> Option<BindingMap> {
    fn mapping_of_mirrored_pats_inner(
        a_pat: &Pat<'_>,
        b_pat: &Pat<'_>,
        mapping: &mut BindingMap,
        n_refs: usize,
    ) -> bool {
        match (&a_pat.kind, &b_pat.kind) {
            (PatKind::Tuple(a_pats, a_dots), PatKind::Tuple(b_pats, b_dots)) => {
                a_dots == b_dots
                    && a_pats.len() == b_pats.len()
                    && iter::zip(a_pats.iter(), b_pats.iter())
                        .all(|(a, b)| mapping_of_mirrored_pats_inner(a, b, mapping, n_refs))
            },
            (PatKind::Binding(_, _, a_ident, _), PatKind::Binding(_, _, b_ident, _)) => {
                let a_key = BindingKey {
                    ident: *a_ident,
                    source: BindingSource::Left,
                };
                let b_key = BindingKey {
                    ident: *b_ident,
                    source: BindingSource::Right,
                };
                let a_value = BindingValue {
                    mirrored: b_key,
                    n_refs,
                };
                let b_value = BindingValue {
                    mirrored: a_key,
                    n_refs,
                };
                mapping.insert(a_key, a_value);
                mapping.insert(b_key, b_value);
                true
            },
            (PatKind::Wild, PatKind::Wild) => true,
            (PatKind::TupleStruct(_, a_pats, a_dots), PatKind::TupleStruct(_, b_pats, b_dots)) => {
                a_dots == b_dots
                    && a_pats.len() == b_pats.len()
                    && iter::zip(a_pats.iter(), b_pats.iter())
                        .all(|(a, b)| mapping_of_mirrored_pats_inner(a, b, mapping, n_refs))
            },
            (PatKind::Struct(_, a_fields, a_rest), PatKind::Struct(_, b_fields, b_rest)) => {
                a_rest == b_rest
                    && a_fields.len() == b_fields.len()
                    && iter::zip(a_fields.iter(), b_fields.iter()).all(|(a_field, b_field)| {
                        a_field.ident == b_field.ident
                            && mapping_of_mirrored_pats_inner(a_field.pat, b_field.pat, mapping, n_refs)
                    })
            },
            (PatKind::Ref(a_inner, _, _), PatKind::Ref(b_inner, _, _)) => {
                mapping_of_mirrored_pats_inner(a_inner, b_inner, mapping, n_refs + 1)
            },
            (PatKind::Slice(a_elems, None, a_rest), PatKind::Slice(b_elems, None, b_rest)) => {
                a_elems.len() == b_elems.len()
                    && iter::zip(a_elems.iter(), b_elems.iter())
                        .all(|(a, b)| mapping_of_mirrored_pats_inner(a, b, mapping, n_refs))
                    && a_rest.len() == b_rest.len()
                    && iter::zip(a_rest.iter(), b_rest.iter())
                        .all(|(a, b)| mapping_of_mirrored_pats_inner(a, b, mapping, n_refs))
            },
            _ => false,
        }
    }

    let mut mapping = FxHashMap::default();
    if mapping_of_mirrored_pats_inner(a_pat, b_pat, &mut mapping, 0) {
        return Some(mapping);
    }

    None
}

pub(super) fn expr_borrows(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Ref(..)) || ty.walk().any(|arg| matches!(arg.kind(), GenericArgKind::Lifetime(_)))
}

pub(super) fn expr_is_field_access(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Field(_, _) => true,
        ExprKind::AddrOf(_, Mutability::Not, inner) => expr_is_field_access(inner),
        _ => false,
    }
}
