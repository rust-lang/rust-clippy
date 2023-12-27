use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::macro_backtrace;
use clippy_utils::ty::expr_sig;
use clippy_utils::{get_parent_node, is_default_equivalent, path_def_id};
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::intravisit::{walk_ty, Visitor};
use rustc_hir::{Block, Expr, ExprKind, Local, Node, QPath, TyKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::ty::print::with_forced_trimmed_paths;
use rustc_middle::ty::IsSuggestable;
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// checks for `Box::new(T::default())`, which is better written as
    /// `Box::<T>::default()`.
    ///
    /// ### Why is this bad?
    /// First, it's more complex, involving two calls instead of one.
    /// Second, `Box::default()` can be faster
    /// [in certain cases](https://nnethercote.github.io/perf-book/standard-library-types.html#box).
    ///
    /// ### Example
    /// ```no_run
    /// let x: Box<String> = Box::new(Default::default());
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: Box<String> = Box::default();
    /// ```
    #[clippy::version = "1.66.0"]
    pub BOX_DEFAULT,
    perf,
    "Using Box::new(T::default()) instead of Box::default()"
}

declare_lint_pass!(BoxDefault => [BOX_DEFAULT]);

impl LateLintPass<'_> for BoxDefault {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        if let ExprKind::Call(box_new, [arg]) = expr.kind
            && let ExprKind::Path(QPath::TypeRelative(ty, seg)) = box_new.kind
            && let ExprKind::Call(arg_path, ..) = arg.kind
            && !in_external_macro(cx.sess(), expr.span)
            && (expr.span.eq_ctxt(arg.span) || is_local_vec_expn(cx, arg, expr))
            && seg.ident.name == sym::new
            && path_def_id(cx, ty).map_or(false, |id| Some(id) == cx.tcx.lang_items().owned_box())
            && is_default_equivalent(cx, arg)
        {
            span_lint_and_sugg(
                cx,
                BOX_DEFAULT,
                expr.span,
                "`Box::new(_)` of default value",
                "try",
                if is_plain_default(cx, arg_path) || given_type(cx, expr) {
                    "Box::default()".into()
                } else if let Some(arg_ty) = cx.typeck_results().expr_ty(arg).make_suggestable(cx.tcx, true) {
                    with_forced_trimmed_paths!(format!("Box::<{arg_ty}>::default()"))
                } else {
                    return;
                },
                Applicability::MachineApplicable,
            );
        }
    }
}

fn is_plain_default(cx: &LateContext<'_>, arg_path: &Expr<'_>) -> bool {
    // we need to match the actual path so we don't match e.g. "u8::default"
    if let ExprKind::Path(QPath::Resolved(None, path)) = &arg_path.kind
        && let Res::Def(_, def_id) = path.res
    {
        // avoid generic parameters
        cx.tcx.is_diagnostic_item(sym::default_fn, def_id) && path.segments.iter().all(|seg| seg.args.is_none())
    } else {
        false
    }
}

fn is_local_vec_expn(cx: &LateContext<'_>, expr: &Expr<'_>, ref_expr: &Expr<'_>) -> bool {
    macro_backtrace(expr.span).next().map_or(false, |call| {
        cx.tcx.is_diagnostic_item(sym::vec_macro, call.def_id) && call.span.eq_ctxt(ref_expr.span)
    })
}

#[derive(Default)]
struct InferVisitor(bool);

impl<'tcx> Visitor<'tcx> for InferVisitor {
    fn visit_ty(&mut self, t: &rustc_hir::Ty<'_>) {
        self.0 |= matches!(t.kind, TyKind::Infer | TyKind::OpaqueDef(..) | TyKind::TraitObject(..));
        if !self.0 {
            walk_ty(self, t);
        }
    }
}

fn given_type(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    match get_parent_node(cx.tcx, expr.hir_id) {
        Some(Node::Local(Local { ty: Some(ty), .. })) => {
            let mut v = InferVisitor::default();
            v.visit_ty(ty);
            !v.0
        },
        Some(
            Node::Expr(Expr {
                kind: ExprKind::Call(path, args),
                ..
            })
            | Node::Block(Block {
                expr:
                    Some(Expr {
                        kind: ExprKind::Call(path, args),
                        ..
                    }),
                ..
            }),
        ) => {
            if let Some(index) = args.iter().position(|arg| arg.hir_id == expr.hir_id)
                && let Some(sig) = expr_sig(cx, path)
                && let Some(input) = sig.input(index)
                && !cx.typeck_results().expr_ty_adjusted(expr).boxed_ty().is_trait()
            {
                input.no_bound_vars().is_some()
            } else {
                false
            }
        },
        _ => false,
    }
}
