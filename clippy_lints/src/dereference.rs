use crate::{
    needless_borrow::NEEDLESS_BORROW,
    utils::{
        expr_sig, get_node_span, get_parent_node, in_macro, is_allowed, match_def_path, paths, peel_hir_ty_refs,
        peel_mid_ty_refs, snippet_with_context, span_lint_and_sugg,
    },
};
use rustc_ast::util::parser::PREC_PREFIX;
use rustc_errors::Applicability;
use rustc_hir::{
    self as hir, Arm, Block, BorrowKind, Destination, Expr, ExprKind, FnRetTy, GenericArg, HirId, ImplItem,
    ImplItemKind, Item, ItemKind, Local, MatchSource, Mutability, Node, Path, QPath, TraitItem, TraitItemKind, TyKind,
    UnOp,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{
    self,
    adjustment::{Adjust, Adjustment},
    Ty, TyCtxt, TyS, TypeFoldable, TypeckResults,
};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{symbol::sym, Span};

declare_clippy_lint! {
    /// **What it does:** Checks for explicit dereferencing which would be covered by
    /// auto-dereferencing.
    ///
    /// **Why is this bad?** This unnecessarily complicates the code.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// use std::ops::Deref;
    /// fn foo(_: &str) {}
    /// foo(&*String::new());
    /// foo(String::new().deref());
    /// ```
    /// Use instead:
    /// ```rust
    /// use std::ops::Deref;
    /// fn foo(_: &str) {}
    /// foo(&String::new());
    /// foo(&String::new());
    /// ```
    pub EXPLICIT_AUTO_DEREF,
    style,
    "dereferencing when the compiler would automatically dereference"
}

declare_clippy_lint! {
    /// **What it does:** Checks for explicit `deref()` or `deref_mut()` method calls.
    ///
    /// **Why is this bad?** Dereferencing by `&*x` or `&mut *x` is clearer and more concise,
    /// when not part of a method chain.
    ///
    /// **Example:**
    /// ```rust
    /// use std::ops::Deref;
    /// let a: &mut String = &mut String::from("foo");
    /// let b: &str = a.deref();
    /// ```
    /// Could be written as:
    /// ```rust
    /// let a: &mut String = &mut String::from("foo");
    /// let b = &*a;
    /// ```
    ///
    /// This lint excludes
    /// ```rust,ignore
    /// let _ = d.unwrap().deref();
    /// ```
    pub EXPLICIT_DEREF_METHODS,
    pedantic,
    "Explicit use of deref or deref_mut method while not in a method chain."
}

impl_lint_pass!(Dereferencing => [
    EXPLICIT_AUTO_DEREF,
    EXPLICIT_DEREF_METHODS,
    NEEDLESS_BORROW,
]);

#[derive(Default)]
pub struct Dereferencing {
    state: Option<(State, StateData)>,

    // While parsing a `deref` method call in ufcs form, the path to the function is itself an
    // expression. This is to store the id of that expression so it can be skipped when
    // `check_expr` is called for it.
    skip_expr: Option<HirId>,
}

struct StateData {
    /// Span of the top level expression
    span: Span,
    // HirId of the top level expression
    hir_id: HirId,
    /// The required mutability
    target_mut: Mutability,
}

enum State {
    AddrOf,
    // Any number of reference operations which auto-deref would take care of.
    // This should take priority over all others.
    AutoDeref {
        sub_span: Option<Span>,
    },
    // Any number of deref method calls.
    DerefMethod {
        // The number of calls in a sequence which changed the referenced type
        ty_changed_count: usize,
        is_final_ufcs: bool,
    },
    NeedlessBorrow {
        // The number of borrows remaining
        remaining: usize,
    },
}

// A reference operation considered by this lint pass
enum RefOp {
    Method,
    Deref,
    AddrOf,
}

impl<'tcx> LateLintPass<'tcx> for Dereferencing {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        // Skip path expressions from deref calls. e.g. `Deref::deref(e)`
        if Some(expr.hir_id) == self.skip_expr.take() {
            return;
        }

        // Stop processing sub expressions when a macro call is seen
        if in_macro(expr.span) {
            if let Some((state, data)) = self.state.take() {
                report(cx, expr, state, data);
            }
            return;
        }

        let typeck = cx.typeck_results();
        let (kind, sub_expr) = match try_parse_ref_op(cx.tcx, typeck, expr) {
            Some(x) => x,
            None => {
                // The whole chain of reference operations has been seen
                if let Some((state, data)) = self.state.take() {
                    report(cx, expr, state, data);
                }
                return;
            },
        };

        self.state = match (self.state.take(), kind) {
            (None, kind) => {
                let parent = get_parent_node(cx.tcx, expr.hir_id);
                // format_args will insert an address of expression into the wrong span.
                if parent
                    .and_then(get_node_span)
                    .and_then(|s| s.ctxt().outer_expn_data().macro_def_id)
                    .map_or(false, |id| {
                        match_def_path(cx, id, &paths::FORMAT_ARGS_MACRO)
                            || match_def_path(cx, id, &paths::FORMAT_ARGS_NL_MACRO)
                    })
                {
                    return;
                }

                let expr_adjustments = find_adjustments(cx.tcx, typeck, expr);
                let expr_ty = typeck.expr_ty(expr);
                let data = StateData {
                    span: expr.span,
                    hir_id: expr.hir_id,
                    target_mut: if let ty::Ref(_, _, mutability) =
                        *expr_adjustments.last().map_or(expr_ty, |a| a.target).kind()
                    {
                        mutability
                    } else {
                        Mutability::Not
                    },
                };

                match (kind, expr_adjustments) {
                    (
                        RefOp::AddrOf,
                        [Adjustment {
                            kind: Adjust::Deref(None),
                            target,
                        }, Adjustment {
                            kind: Adjust::Deref(None),
                            ..
                        }, adjustments @ ..],
                    ) if target.is_ref() => {
                        let count = adjustments
                            .iter()
                            .take_while(|&a| matches!(a.kind, Adjust::Deref(None)) && a.target.is_ref())
                            .count();
                        Some((
                            State::NeedlessBorrow {
                                remaining: if matches!(
                                    adjustments.get(count),
                                    Some(Adjustment {
                                        kind: Adjust::Borrow(_),
                                        ..
                                    })
                                ) {
                                    count
                                } else {
                                    count + 1
                                },
                            },
                            data,
                        ))
                    },
                    (RefOp::AddrOf, _) if !typeck.expr_ty(sub_expr).is_ref() => Some((State::AddrOf, data)),
                    // Only check for auto-deref when the resulting type is a reference to a
                    // non-reference type.
                    (RefOp::Deref | RefOp::Method, _)
                        if expr_ty.is_ref()
                            && !is_allowed(cx, EXPLICIT_AUTO_DEREF, expr.hir_id)
                            && is_stable_auto_deref_position(cx.tcx, typeck, expr.hir_id) =>
                    {
                        Some((State::AutoDeref { sub_span: None }, data))
                    },
                    (RefOp::Method, _)
                        if !is_allowed(cx, EXPLICIT_DEREF_METHODS, expr.hir_id)
                            && is_linted_explicit_deref_position(parent, expr.hir_id) =>
                    {
                        Some((
                            State::DerefMethod {
                                ty_changed_count: if deref_method_same_type(expr_ty, typeck.expr_ty(sub_expr)) {
                                    0
                                } else {
                                    1
                                },
                                is_final_ufcs: matches!(expr.kind, ExprKind::Call(..)),
                            },
                            data,
                        ))
                    },
                    _ => None,
                }
            },
            (Some((State::AddrOf, data)), RefOp::Deref | RefOp::Method)
                if !is_allowed(cx, EXPLICIT_AUTO_DEREF, data.hir_id)
                    && is_stable_auto_deref_position(cx.tcx, typeck, data.hir_id) =>
            {
                Some((
                    State::AutoDeref {
                        sub_span: Some(expr.span),
                    },
                    data,
                ))
            },
            (state @ Some((State::AutoDeref { .. }, _)), _) => state,
            (Some((State::DerefMethod { ty_changed_count, .. }, data)), RefOp::Method) => Some((
                State::DerefMethod {
                    ty_changed_count: if deref_method_same_type(typeck.expr_ty(expr), typeck.expr_ty(sub_expr)) {
                        ty_changed_count
                    } else {
                        ty_changed_count + 1
                    },
                    is_final_ufcs: matches!(expr.kind, ExprKind::Call(..)),
                },
                data,
            )),
            (Some((State::NeedlessBorrow { remaining }, data)), RefOp::AddrOf) if remaining != 0 => Some((
                State::NeedlessBorrow {
                    remaining: remaining - 1,
                },
                data,
            )),
            (Some((state, data)), _) => {
                report(cx, expr, state, data);
                None
            },
        };
    }
}

fn try_parse_ref_op(
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx TypeckResults<'_>,
    expr: &'tcx Expr<'_>,
) -> Option<(RefOp, &'tcx Expr<'tcx>)> {
    let (def_id, arg) = match expr.kind {
        ExprKind::MethodCall(_, _, [arg], _) => (typeck.type_dependent_def_id(expr.hir_id)?, arg),
        ExprKind::Call(
            Expr {
                kind: ExprKind::Path(path),
                hir_id,
                ..
            },
            [arg],
        ) => (typeck.qpath_res(path, *hir_id).opt_def_id()?, arg),
        ExprKind::Unary(UnOp::Deref, sub_expr) if !typeck.expr_ty(sub_expr).is_unsafe_ptr() => {
            return Some((RefOp::Deref, sub_expr));
        },
        ExprKind::AddrOf(BorrowKind::Ref, _, sub_expr) => return Some((RefOp::AddrOf, sub_expr)),
        _ => return None,
    };
    if tcx.is_diagnostic_item(sym::deref_method, def_id)
        || tcx.trait_of_item(def_id)? == tcx.lang_items().deref_mut_trait()?
    {
        Some((RefOp::Method, arg))
    } else {
        return None;
    }
}

// Checks whether the type for a deref call actually changed the type, not just the mutability of
// the reference.
fn deref_method_same_type(result_ty: Ty<'tcx>, arg_ty: Ty<'tcx>) -> bool {
    match (result_ty.kind(), arg_ty.kind()) {
        (ty::Ref(_, result_ty, _), ty::Ref(_, arg_ty, _)) => TyS::same_type(result_ty, arg_ty),

        // The result type for a deref method is always a reference
        // Not matching the previous pattern means the argument type is not a reference
        // This means that the type did change
        _ => false,
    }
}

// Adjustments are sometimes made in the parent block rather than the expression itself.
fn find_adjustments(
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx TypeckResults<'_>,
    expr: &'tcx Expr<'_>,
) -> &'tcx [Adjustment<'tcx>] {
    let map = tcx.hir();
    let mut iter = map.parent_iter(expr.hir_id);
    let mut prev = expr;

    loop {
        match typeck.expr_adjustments(prev) {
            [] => (),
            a => break a,
        };

        match iter.next().map(|(_, x)| x) {
            Some(Node::Block(_)) => {
                if let Some((_, Node::Expr(e))) = iter.next() {
                    prev = e;
                } else {
                    // This shouldn't happen. Blocks are always contained in an expression.
                    break &[];
                }
            },
            Some(Node::Expr(&Expr {
                kind: ExprKind::Break(Destination { target_id: Ok(id), .. }, _),
                ..
            })) => {
                if let Some(Node::Expr(e)) = map.find(id) {
                    prev = e;
                    iter = map.parent_iter(id);
                    continue;
                } else {
                    // This shouldn't happen. The destination should definitely exist at this point.
                    break &[];
                }
            },
            _ => break &[],
        }
    }
}

// Checks whether the parent node is a suitable context for switching from a deref method to the
// deref operator.
fn is_linted_explicit_deref_position(parent: Option<Node<'_>>, child_id: HirId) -> bool {
    let parent = match parent {
        Some(Node::Expr(e)) => e,
        _ => return true,
    };
    match parent.kind {
        // Leave deref calls in the middle of a method chain.
        // e.g. x.deref().foo()
        ExprKind::MethodCall(_, _, [self_arg, ..], _) if self_arg.hir_id == child_id => false,

        // Leave deref calls resulting in a called function
        // e.g. (x.deref())()
        ExprKind::Call(func_expr, _) if func_expr.hir_id == child_id => false,

        // Leave deref calls for try and await expressions
        // e.g. x.deref()?
        ExprKind::Match(_, _, MatchSource::TryDesugar | MatchSource::AwaitDesugar) => false,

        // Makes an ugly suggestion
        // e.g. *x.deref() => *&*x
        ExprKind::Unary(UnOp::Deref, _) => false,

        ExprKind::Field(..) | ExprKind::Index(..) | ExprKind::Err => false,

        ExprKind::Box(..)
        | ExprKind::ConstBlock(..)
        | ExprKind::Array(_)
        | ExprKind::Call(..)
        | ExprKind::MethodCall(..)
        | ExprKind::Tup(..)
        | ExprKind::Binary(..)
        | ExprKind::Unary(..)
        | ExprKind::Lit(..)
        | ExprKind::Cast(..)
        | ExprKind::Type(..)
        | ExprKind::DropTemps(..)
        | ExprKind::If(..)
        | ExprKind::Loop(..)
        | ExprKind::Match(..)
        | ExprKind::Closure(..)
        | ExprKind::Block(..)
        | ExprKind::Assign(..)
        | ExprKind::AssignOp(..)
        | ExprKind::Path(..)
        | ExprKind::AddrOf(..)
        | ExprKind::Break(..)
        | ExprKind::Continue(..)
        | ExprKind::Ret(..)
        | ExprKind::InlineAsm(..)
        | ExprKind::LlvmInlineAsm(..)
        | ExprKind::Struct(..)
        | ExprKind::Repeat(..)
        | ExprKind::Yield(..) => true,
    }
}

// Checks if the expression for the given id occurs in a position which auto dereferencing applies.
// Note that the target type must not be inferred in a way that may cause auto-deref to select a
// different type, nor may the position be the result of a macro expansion.
//
// e.g. the following should not linted
// macro_rules! foo { ($e:expr) => { let x: &str = $e; }}
// foo!(&*String::new());
// fn foo<T>(_: &T) {}
// foo(&*String::new())
fn is_stable_auto_deref_position(tcx: TyCtxt<'tcx>, typeck: &'tcx TypeckResults<'_>, id: HirId) -> bool {
    let map = tcx.hir();
    // break expressions can a jump to a different node, so for loops won't work here.
    let mut iter = map.parent_iter(id);
    let mut child_id = id;
    while let Some((parent_id, parent)) = iter.next() {
        match parent {
            // Local binding
            Node::Local(&Local { ty: Some(ty), span, .. }) if !in_macro(span) => {
                return is_binding_ty_auto_deref_stable(ty);
            },

            // Static and const bindings. The type of these have to be fully defined.
            Node::Item(&Item {
                kind: ItemKind::Static(..),
                span,
                ..
            })
            | Node::Item(&Item {
                kind: ItemKind::Const(..),
                span,
                ..
            })
            | Node::TraitItem(&TraitItem {
                kind: TraitItemKind::Const(..),
                span,
                ..
            })
            | Node::ImplItem(&ImplItem {
                kind: ImplItemKind::Const(..),
                span,
                ..
            }) if !in_macro(span) => return true,

            // Implicit return from a function. Determine whether auto-deref might change the type.
            Node::Item(&Item {
                kind: ItemKind::Fn(..),
                span,
                ..
            })
            | Node::TraitItem(&TraitItem {
                kind: TraitItemKind::Fn(..),
                span,
                ..
            })
            | Node::ImplItem(&ImplItem {
                kind: ImplItemKind::Fn(..),
                span,
                ..
            }) if !in_macro(span) => {
                let output = tcx
                    .fn_sig(tcx.hir().local_def_id(parent_id).to_def_id())
                    .skip_binder()
                    .output();

                return !(output.has_placeholders() || output.has_opaque_types() || output.has_projections());
            },

            Node::Arm(&Arm { span, body, .. }) if body.hir_id == child_id && !in_macro(span) => (),

            Node::Block(&Block {
                span, expr: Some(expr), ..
            }) if expr.hir_id == child_id && !in_macro(span) => (),

            Node::Expr(parent) if !in_macro(parent.span) => match parent.kind {
                // Determine whether auto-deref might change the return type of the function.
                ExprKind::Ret(Some(_)) if !in_macro(map.span(map.local_def_id_to_hir_id(typeck.hir_owner))) => {
                    let output = tcx.fn_sig(typeck.hir_owner).skip_binder().output();
                    return !(output.has_placeholders() || output.has_opaque_types() || output.has_projections());
                },

                // Determine which argument the child is and whether auto-deref might cause it's
                // type to change.
                ExprKind::Call(func, args) => {
                    let arg_pos = if let Some(arg_pos) = args.iter().position(|x| x.hir_id == child_id) {
                        arg_pos
                    } else {
                        return false;
                    };

                    return expr_sig(tcx, typeck, func).map_or(false, |sig| {
                        is_param_auto_deref_stable(sig.input(arg_pos).skip_binder())
                    });
                },

                // Determine which argument, ignoring the self parameter, the child is and whether
                // auto-deref might cause it's type to change.
                ExprKind::MethodCall(_, _, [_, args @ ..], _) => {
                    let id = typeck.type_dependent_def_id(parent.hir_id).unwrap();
                    return if let Some(arg) = args.iter().position(|x| x.hir_id == child_id) {
                        let arg = &tcx.fn_sig(id).skip_binder().inputs()[arg];
                        is_param_auto_deref_stable(arg)
                    } else {
                        false
                    };
                },

                // Determine which field is being assigned and whether auto-deref might cause the
                // type to change
                ExprKind::Struct(path, fields, _) => {
                    let res = typeck.qpath_res(path, parent.hir_id);
                    let id = res.def_id();
                    return if let Some(field) = fields.iter().find(|f| f.expr.hir_id == child_id) {
                        if let Some(field) = tcx
                            .adt_def(id)
                            .variant_of_res(res)
                            .fields
                            .iter()
                            .find(|f| f.ident == field.ident)
                        {
                            let field_ty = tcx.type_of(field.did);
                            is_param_auto_deref_stable(field_ty)
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                },

                // Continue to the parent expression
                ExprKind::Block(..) => (),

                ExprKind::If(cond_expr, ..) if cond_expr.hir_id != child_id => (),

                ExprKind::Match(scrutinee_expr, ..) if scrutinee_expr.hir_id != child_id => (),

                // Continue to the loop's parent.
                ExprKind::Break(Destination { target_id: Ok(id), .. }, _) => {
                    iter = map.parent_iter(id);
                    child_id = id;
                    continue;
                },

                // All other expressions aren't auto-deref contexts.
                _ => return false,
            },

            // All other nodes aren't auto-deref contexts.
            _ => return false,
        }

        child_id = parent_id;
    }
    false
}

// Checks whether auto-dereferencing any type into a binding of the given type will definitely
// produce the same result.
//
// e.g.
// let x = Box::new(Box::new(0u32));
// let y1: &Box<_> = x.deref();
// let y2: &Box<_> = &x;
//
// Here `y1` and `y2` would resolve to different types, so the type `&Box<_>` is not stable when
// switching to auto-dereferencing.
fn is_binding_ty_auto_deref_stable(ty: &hir::Ty<'_>) -> bool {
    let (ty, count) = peel_hir_ty_refs(ty);
    if count != 1 {
        return false;
    }

    match &ty.kind {
        TyKind::Slice(_)
        | TyKind::Array(..)
        | TyKind::BareFn(_)
        | TyKind::Never
        | TyKind::Typeof(..)
        | TyKind::Tup(_)
        | TyKind::Ptr(_)
        | TyKind::TraitObject(..) => true,
        TyKind::OpaqueDef(..) | TyKind::Infer | TyKind::Err => false,
        TyKind::Rptr(_, ty) => is_binding_ty_auto_deref_stable(&ty.ty),
        &TyKind::Path(QPath::Resolved(
            _,
            Path {
                segments: [.., path], ..
            },
        ))
        | &TyKind::Path(QPath::TypeRelative(_, path)) => {
            if let Some(args) = path.args {
                args.args.iter().all(|arg| {
                    if let GenericArg::Type(ty) = arg {
                        !ty_contains_infer(ty)
                    } else {
                        true
                    }
                })
            } else {
                true
            }
        },
        TyKind::Path(_) => true,
    }
}

// Checks whether a type is inferred at some point.
// e.g. _, Box<_>, [_]
fn ty_contains_infer(ty: &hir::Ty<'_>) -> bool {
    match &ty.kind {
        TyKind::OpaqueDef(..) | TyKind::Infer | TyKind::Err => true,
        TyKind::Never | TyKind::Typeof(_) | TyKind::TraitObject(..) => false,
        TyKind::Slice(ty) | TyKind::Array(ty, _) => ty_contains_infer(ty),
        TyKind::Ptr(ty) | TyKind::Rptr(_, ty) => ty_contains_infer(&ty.ty),
        TyKind::Tup(tys) => tys.iter().any(ty_contains_infer),
        TyKind::BareFn(ty) => {
            if ty.decl.inputs.iter().any(ty_contains_infer) {
                return true;
            }
            if let FnRetTy::Return(ty) = &ty.decl.output {
                ty_contains_infer(ty)
            } else {
                false
            }
        },
        &TyKind::Path(QPath::Resolved(
            _,
            Path {
                segments: [.., path], ..
            },
        ))
        | &TyKind::Path(QPath::TypeRelative(_, path)) => {
            if let Some(args) = path.args {
                args.args.iter().any(|arg| {
                    if let GenericArg::Type(ty) = arg {
                        ty_contains_infer(ty)
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        },
        TyKind::Path(_) => true,
    }
}

// Checks whether a type is stable when switching to auto dereferencing,
fn is_param_auto_deref_stable(ty: Ty<'_>) -> bool {
    let (ty, count) = peel_mid_ty_refs(ty);
    if count != 1 {
        return false;
    }

    match ty.peel_refs().kind() {
        ty::Bool
        | ty::Char
        | ty::Int(_)
        | ty::Uint(_)
        | ty::Float(_)
        | ty::Foreign(_)
        | ty::Str
        | ty::Array(..)
        | ty::Slice(..)
        | ty::RawPtr(..)
        | ty::FnDef(..)
        | ty::FnPtr(_)
        | ty::Closure(..)
        | ty::Generator(..)
        | ty::GeneratorWitness(..)
        | ty::Never
        | ty::Tuple(_)
        | ty::Ref(..) => true,
        ty::Placeholder(_)
        | ty::Infer(_)
        | ty::Error(_)
        | ty::Param(_)
        | ty::Bound(..)
        | ty::Opaque(..)
        | ty::Projection(_)
        | ty::Dynamic(..) => false,
        ty::Adt(_, subs) => subs
            .types()
            .all(|ty| !(ty.has_placeholders() || ty.has_param_types_or_consts() || ty.has_projections())),
    }
}

fn report(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, state: State, data: StateData) {
    match state {
        State::AddrOf => (),
        State::AutoDeref { sub_span } => {
            let (span, needs_ref) = match (sub_span, cx.typeck_results().expr_ty(expr).is_ref()) {
                (Some(_), true) => (data.span, None),
                (Some(sub_span), false) => (sub_span, None),
                (None, is_ref) => (data.span, (!is_ref).then(|| data.target_mut)),
            };

            let mut app = Applicability::MachineApplicable;
            let (expr_str, _) = snippet_with_context(cx, expr.span, data.span.ctxt(), "..", &mut app);

            span_lint_and_sugg(
                cx,
                EXPLICIT_AUTO_DEREF,
                span,
                "explicit dereference in auto-deref context",
                "try this",
                match needs_ref {
                    None => expr_str.into(),
                    Some(Mutability::Not) => format!("&{}", expr_str),
                    Some(Mutability::Mut) => format!("&mut {}", expr_str),
                },
                app,
            );
        },
        State::DerefMethod {
            ty_changed_count,
            is_final_ufcs,
        } => {
            let mut app = Applicability::MachineApplicable;
            let (expr_str, expr_is_macro_call) = snippet_with_context(cx, expr.span, data.span.ctxt(), "..", &mut app);
            let ty = cx.typeck_results().expr_ty(expr);
            let (_, ref_count) = peel_mid_ty_refs(ty);
            let deref_str = if ty_changed_count >= ref_count && ref_count != 0 {
                // a deref call changing &T -> &U requires two deref operators the first time
                // this occurs. One to remove the reference, a second to call the deref impl.
                "*".repeat(ty_changed_count + 1)
            } else {
                "*".repeat(ty_changed_count)
            };
            let addr_of_str = if ty_changed_count < ref_count {
                // Check if a reborrow from &mut T -> &T is required.
                if data.target_mut == Mutability::Not && matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut)) {
                    "&*"
                } else {
                    ""
                }
            } else if data.target_mut == Mutability::Mut {
                "&mut "
            } else {
                "&"
            };

            let expr_str = if !expr_is_macro_call && is_final_ufcs && expr.precedence().order() < PREC_PREFIX {
                format!("({})", expr_str)
            } else {
                expr_str.into_owned()
            };

            span_lint_and_sugg(
                cx,
                EXPLICIT_DEREF_METHODS,
                data.span,
                "explicit `deref` method call",
                "try this",
                format!("{}{}{}", addr_of_str, deref_str, expr_str),
                app,
            );
        },
        State::NeedlessBorrow { .. } => {
            let ty = cx.typeck_results().expr_ty(expr);
            let mut app = Applicability::MachineApplicable;
            let (expr_str, _) = snippet_with_context(cx, expr.span, data.span.ctxt(), "..", &mut app);

            span_lint_and_sugg(
                cx,
                NEEDLESS_BORROW,
                data.span,
                &format!(
                    "this expression borrows a reference (`{}`) that is immediately dereferenced \
                            by the compiler",
                    ty,
                ),
                "change this to",
                expr_str.into(),
                app,
            );
        },
    }
}
