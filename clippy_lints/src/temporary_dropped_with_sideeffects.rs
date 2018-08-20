use super::utils;
use super::utils::match_def_path;
use super::utils::paths::*;
use rustc::hir::map::Node;
use rustc::hir::{def_id::DefId, Expr, ExprKind, Item, ItemKind, Pat, PatKind};
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::ty::{subst::UnpackedKind, TyCtxt, TyS, TypeVariants};
use rustc::{declare_lint, lint_array};

/// **What it does:**
/// Checks for temporaries containing values where dropping has a side effect.
///
/// **Why is this bad?**
///
/// Temporaries are generally dropped at the trailing semicolon, but there are some
/// [exceptions that are not really well known](https://doc.rust-lang.org/stable/reference/expressions.html#temporary-lifetimes)
/// and that can lead to surprising results and bugs.
///
/// **Known problems:**
///
/// 1. Bound values are considered as temporaries when an unrelated wildcard is used in pattern
/// matching. For instance, the following example will trigger the lint, while
/// `mutex.lock()` is not a temporary:
///
/// ```rust
///     let (_, a) = function_that_returns_tuple_with_first_element_mutexguard();
/// ```
///
/// 2. Suppose a function or method
///   a) takes a value that will have side effects when being dropped and
///   b) produces a value that will have side effects when being dropped.
/// When we pass a temporary to such a function or method, the lint will not be triggered for the
/// temporary passed to the function or method. Example:
///
/// ```rust
/// fn f<'a, 'b>(mutex: &'a Mutex<u8>, mutex_guard: MutexGuard<'b, u8>) -> MutexGuard<'a, u8> {
///     mutex.lock().unwrap()
/// }
///
/// fn lint_should_be_triggered_but_is_not() {
///     let m1 = Mutex::new(4);
///     let m2 = Mutex::new(5);
///     let c = f(&m1, m2.lock().unwrap());
///     //             ^^^^^^^^^^^^^^^^^^ is a temporary that is dropped at the trailing semicolon,
///     //                                but the lint "thinks" it is moved into `c`.
///     }
/// ```
///
/// **Examples:**
///
/// Code that triggers the lint:
///
/// ``rust
/// let mutex: Mutex<bool> = Mutex::new(true);
///
/// // The following is confusing because it may not be clear that the lock is already released
/// // before the if-block. This code triggers the lint.
/// if *mutex.lock().unwrap() {
///     // Do something.
/// }
/// ```
///
/// How this can be solved:
/// ```rust
/// let mutex = Mutex::new(true);
/// // In the following piece of code, it is clear that the lock is released at the end
/// // of the scope block. This does not trigger the lint.
/// let must_do_something : bool;
/// {
///     let value = mutex.lock().unwrap();
///     must_do_something = *value;
/// }
/// if must_do_something {
///     // Do something.
/// }
/// ```

declare_clippy_lint! {
    pub DROPPING_TEMPORARY_WITH_SIDE_EFFECT,
    correctness,
    "temporary_dropped_with_sideeffects"
}

const KNOWN_TYPES_WHERE_DROPPING_HAS_SIDE_EFFECT: [&[&str]; 5] = [
    // This list does not include `LockResult` and `TryLockResult`
    // because these types do not necessarily hold a lock.
    // In case they do, this lint will typically catch that because then a type parameter is
    // in this list.
    &SYNC_MUTEXGUARD,
    &SYNC_RWLOCKREADGUARD,
    &SYNC_RWLOCKWRITEGUARD,
    &CELL_REF,
    &CELL_REFMUT,
];

pub struct TemporaryWithDroppingSideEffects;

impl LintPass for TemporaryWithDroppingSideEffects {
    fn get_lints(&self) -> LintArray {
        lint_array!(DROPPING_TEMPORARY_WITH_SIDE_EFFECT)
    }
}

impl<'a, 'tcx> LateLintPass<'a, 'tcx> for TemporaryWithDroppingSideEffects {
    fn check_expr(&mut self, cx: &LateContext<'a, 'tcx>, expr: &'tcx Expr) {
        // Check the expression type and return if there is no need to inspect the type
        // more closely.
        match expr.node {
            ExprKind::Lit(_)
            | ExprKind::While(_, _, _)
            | ExprKind::Break(_, _)
            | ExprKind::Continue(_)
            | ExprKind::Ret(_)
            | ExprKind::Assign(_, _)
            | ExprKind::AssignOp(_, _, _) => {
                // The type of the expression is the unit type or the never type (`!`),
                // so dropping has no side-effec.
                return;
            }
            ExprKind::Cast(_, _) => {
                // Casting can currently only be done with primitive types.
                return;
            }
            ExprKind::Closure(_, _, _, _, _) => {
                // No way to inspect this type.
                return;
            }
            ExprKind::Box(_)
            | ExprKind::Array(_)
            | ExprKind::AddrOf(_, _)
            | ExprKind::Field(_, _)
            | ExprKind::Index(_, _)
            | ExprKind::Struct(_, _, _)
            | ExprKind::Unary(_, _)
            | ExprKind::Tup(_) => {
                // If there is a problem, it should be raised with a sub-expression.
                return;
            }
            ExprKind::Path(_) => {
                // This covers local variables.
                return;
            }
            ExprKind::Call(_, _)
            | ExprKind::MethodCall(_, _, _)
            | ExprKind::Binary(_, _, _)
            | ExprKind::Type(_, _)
            | ExprKind::If(_, _, _)
            | ExprKind::Loop(_, _, _)
            | ExprKind::Match(_, _, _)
            | ExprKind::Block(_, _)
            | ExprKind::InlineAsm(_, _, _)
            | ExprKind::Repeat(_, _)
            | ExprKind::Yield(_) => (), // Needs to be inspected
        };

        let self_ty = cx.tables.expr_ty(expr);
        if !dropping_has_side_effects(cx.tcx, self_ty) {
            return;
        }

        let parent_node_id = cx.tcx.hir.get_parent_node(expr.id);
        let parent = cx.tcx.hir.find(parent_node_id);
        if let Some(p) = parent {
            if node_is_function_or_method_call(&p) {
                return;
            }
            match p {
                Node::NodeBlock(b) => {
                    if let Some(ref trailing_expr) = b.expr {
                        if expr.id == trailing_expr.id {
                            let grand_parent_id = cx.tcx.hir.get_parent_node(b.id);
                            // Grand parent is an expr wrapping the block, so need to go to
                            // the grand-grand parent
                            let grand_grand_parent_id = cx.tcx.hir.get_parent_node(grand_parent_id);
                            let grand_grand_parent = cx.tcx.hir.find(grand_grand_parent_id);
                            if let Some(gp) = grand_grand_parent {
                                if node_is_function_or_method_call(&gp) {
                                    return;
                                }
                            }
                        }
                    }
                }
                Node::NodeLocal(local) => {
                    if pattern_binds_everything(&local.pat) {
                        return;
                    }
                }
                Node::NodeExpr(parent_expr) => {
                    match parent_expr.node {
                        // We want to allow expressions like `let a = mutex.lock().unwrap();`.
                        ExprKind::Call(_, _) | ExprKind::MethodCall(_, _, _) => {
                            let parent_ty = cx.tables.expr_ty(parent_expr);
                            if dropping_has_side_effects(cx.tcx, parent_ty) {
                                // Here we assume that the parent will need to take care
                                // of being dropped at the right time.
                                return;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        utils::span_lint(
			cx, 
			DROPPING_TEMPORARY_WITH_SIDE_EFFECT, 
			expr.span, 
			"Expression results in an anonymous temporary variable that will have side effects when being dropped."
		);
    }
}

fn node_is_function_or_method_call(node: &Node<'_>) -> bool {
    match node {
        Node::NodeItem(Item {
            node: ItemKind::Fn(_, _, _, _),
            ..
        }) => true,
        Node::NodeImplItem(_) => true,
        _ => false,
    }
}

fn pattern_binds_everything(pattern: &Pat) -> bool {
    match pattern.node {
        PatKind::Wild => false,
        PatKind::Binding(_, _, _, _) => true,
        PatKind::Struct(_, ref field_pats, has_dot_dot) => {
            if has_dot_dot {
                return false;
            }
            for field_pattern in field_pats.iter() {
                if !pattern_binds_everything(&field_pattern.node.pat) {
                    return false;
                }
            }
            true
        }
        PatKind::Path(_) => true,
        PatKind::TupleStruct(_, _, Some(_)) | PatKind::Tuple(_, Some(_)) => false,
        PatKind::TupleStruct(_, ref sub_patterns, None)
        | PatKind::Tuple(ref sub_patterns, None) => {
            for sub_pattern in sub_patterns.iter() {
                if !pattern_binds_everything(sub_pattern) {
                    return false;
                }
            }
            true
        }
        PatKind::Box(ref sub_pattern) | PatKind::Ref(ref sub_pattern, _) => {
            pattern_binds_everything(&sub_pattern)
        }
        PatKind::Lit(_) | PatKind::Range(_, _, _) => {
            // Question for the reviewer: I don't know if this is possible in this context...
            true
        }
        PatKind::Slice(ref start_patterns, ref optional_middle_pattern, ref end_patterns) => {
            for start_pattern in start_patterns.iter() {
                if !pattern_binds_everything(start_pattern) {
                    return false;
                }
            }
            if let Some(middle_pattern) = optional_middle_pattern {
                if !pattern_binds_everything(&middle_pattern) {
                    return false;
                }
            }
            for end_pattern in end_patterns.iter() {
                if !pattern_binds_everything(end_pattern) {
                    return false;
                }
            }
            true
        }
    }
}

/// Check if type is struct, enum or union type with given def path.
fn type_is_known_to_have_side_effect_when_dropped(
    tcx: TyCtxt<'_, '_, '_>,
    definition_id: DefId,
) -> bool {
    for known_type_path in &KNOWN_TYPES_WHERE_DROPPING_HAS_SIDE_EFFECT {
        if match_def_path(tcx, definition_id, known_type_path) {
            return true;
        }
    }
    false
}

fn dropping_has_side_effects<'tcx>(tcx: TyCtxt<'_, '_, '_>, ty: &TyS<'tcx>) -> bool {
    match ty.sty {
		TypeVariants::TyAdt(adt, substs) => {
			if type_is_known_to_have_side_effect_when_dropped(tcx, adt.did) {
				return true;
			}
			
			for subst in substs.iter() {
				if let UnpackedKind::Type(parameter_ty) = subst.unpack() {
					if dropping_has_side_effects(tcx, parameter_ty) {
						return true;
					}
				}
			}
			false
		},
		TypeVariants::TyArray(parameter_type, _) => dropping_has_side_effects(tcx, parameter_type),
		TypeVariants::TyTuple(parameter_types) =>
			parameter_types
				.iter()
				.any(|parameter_type| {dropping_has_side_effects(tcx, parameter_type)}),
		TypeVariants::TyRawPtr(_) 
		  | TypeVariants::TyRef(_, _, _) 
		  | TypeVariants::TySlice(_) 
		  | TypeVariants::TyStr 
			=> false, // This is only a reference, does not implement drop.
		TypeVariants::TyBool
		  | TypeVariants::TyChar
		  | TypeVariants::TyInt(_)
		  | TypeVariants::TyUint(_)
		  | TypeVariants::TyFloat(_)
  		  | TypeVariants::TyNever
		    => false, // Primitive types
		TypeVariants::TyForeign(_)
		  | TypeVariants::TyFnDef(_, _)
		  | TypeVariants::TyDynamic(_, _)
		    => false, // Type of a function, trait or a forein type 
		TypeVariants::TyClosure(_, _)
		  | TypeVariants::TyFnPtr(_)
		  | TypeVariants::TyGenerator(_, _, _)
		  | TypeVariants::TyGeneratorWitness(_) // Question for the reviewer: is this correct?
		    => false, // Closures etc, not entirely clear what to do with this.
		TypeVariants::TyProjection(_)  // Question for the reviewer: is this correct?
          | TypeVariants::TyAnon(_, _) // Question for the reviewer: is this correct?
          | TypeVariants::TyParam(_)
		  | TypeVariants::TyInfer(_)
		  | TypeVariants::TyError
		    => false // Not a concrete type
	}
}
