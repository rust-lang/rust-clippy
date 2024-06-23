// noise reduction, remove before committing!
#![allow(unused_variables)]

use rustc_errors::Applicability;
use rustc_hir::*;
use rustc_hir::def::Res;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use clippy_utils::{match_def_path, fn_def_id};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;

declare_clippy_lint! {
    /// ### What it does
    /// Detects usage of `Option::and_then` and `bool::then_some` that could
	/// be replaced with `Option::filter`.
	///
    /// ### Why is this bad?
	/// Needless complexity, uses recent and uncommon stdlib funtions instead of
	/// one older function.
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.81.0"]
    pub AND_THEN_THEN_SOME,
    nursery,
    "default lint description"
}

// note: `Option::filter` is older than `bool::then_some`,
// so no msrv check is required.
declare_lint_pass!(AndThenThenSome => [AND_THEN_THEN_SOME]);

impl<'tcx> LateLintPass<'tcx> for AndThenThenSome {
	fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
		match expr.kind {
			ExprKind::MethodCall(method_name, selfarg, [ arg ], _span) => {
				//(expr);
				//let option_id = cx.tcx.get_diagnostic_item(sym::Option);
				// TODO: check if type of reciever is diagnostic item Option.
				//let tckr = cx.typeck_results();
				//let def_id = tckr.type_dependent_def_id(expr.hir_id).unwrap();
				//dbg!(method_name, selfarg, arg);
				if is_and_then(cx, expr)
				{
					if let Some((closure_args, predicate)) = dbg!(then_some_closure_arg(cx, arg)) {
						//dbg!(predicate);
						show_sugg(cx, expr.span, selfarg, closure_args, predicate);
					}
				}
			}
			ExprKind::Call(_func, [ selfarg, arg ]) => {
				if dbg!(is_and_then(cx, expr)) {
					if let Some((closure_args, predicate)) = dbg!(then_some_closure_arg(cx, arg)) {
						//dbg!(predicate);
						show_sugg(cx, expr.span, selfarg, closure_args, predicate);
					}
				}
			}
			// TODO: check for call as associated function
			_ => {},
		}
	}
}

// `|v| X.then_some(v)` -> Some((span"|v|", X))
fn then_some_closure_arg<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>)
							  -> Option<(Span, &'tcx Expr<'tcx>)>
{
	dbg!(expr);
	match expr.kind {
		ExprKind::Closure(Closure{
			fn_decl: FnDecl{ inputs: [ Ty{ hir_id: arg_id, ..} ], .. },
			body,
			..
		}) => {
			if let Node::Expr(expr) = dbg!(cx.tcx.hir_node(body.hir_id)) {
				//dbg!(arg_id);
				if let Some(body) = dbg!(peel_closure_body(cx, expr, *arg_id)) {
					Some((cx.tcx.hir().span(*arg_id), body))
				} else {
					None
				}
			} else {
				None
			}
		},
		_ => None,
	}
}

fn peel_closure_body<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, closure_arg_id: HirId) -> Option<&'tcx Expr<'tcx>> {

	match expr.kind {
		// it would be nice if we could lift { x; y.a() } into { x; y }.a()
		ExprKind::Block(Block{ stmts: [], expr: Some(wrapped_expr), ..}, _) => {
			peel_closure_body(cx, wrapped_expr, closure_arg_id)
		}
		ExprKind::MethodCall(_path, selfarg, [ arg ], _span) => {
			if is_then_some(cx, expr) &&
				is_local_defined_at(cx, arg, closure_arg_id)
			{
				// the argument to then_some is the same as that given to the closure
				Some(selfarg)
			} else {
				None
			}
		}
		ExprKind::Call(func, [ pred, arg ]) => {
			//dbg!(func, fn_def_id(cx, expr));
			//todo!();
			if dbg!(is_then_some(cx, expr)) && dbg!(is_local_defined_at(cx, arg, closure_arg_id)) {
				//todo!("it worked!!");
				Some(pred)
				
			} else {
				//todo!("nope");
				None
			}
		}
		_ => {
			eprintln!("cannot peel {expr:#?}");
			None
		}
	}
}

fn is_local_defined_at<'tcx>(cx: &LateContext<'tcx>, local: &Expr<'_>, arg_hid: HirId) -> bool {
	dbg!(local);
	match local.kind {
		ExprKind::Path(QPath::Resolved(_, Path{ res: Res::Local(local_hid), .. })) => {
			// FIXME: this is the best way i could find to compare if
			// a local refers to a specific closure argument.
			//
			// it breaks if the closure argument has an explicitly declared type,
			// since the spans only align for TyKind::Infer
			if let Node::Pat(Pat{ span: local_span, .. }) = dbg!(cx.tcx.hir_node(*local_hid)) &&
				let Node::Ty(Ty{ span: arg_span, .. }) = dbg!(cx.tcx.hir_node(arg_hid)) &&
				local_span == arg_span
			{
				true
			} else {
				false
			}		
		}
		// is not local at all, so definitly isn't a local defined at the given position
		_ => false,
	}
}

fn show_sugg(cx: &LateContext<'_>, span: Span, selfarg: &Expr<'_>, closure_args: Span, predicate: &Expr<'_>) {
	let mut appl = Applicability::MachineApplicable;
	let sugg = format!(
		"{}.filter(|{}| {})",
		snippet_with_applicability(cx, selfarg.span, "<OPTION>", &mut appl),
		snippet_with_applicability(cx, closure_args, "<ARGS>", &mut appl),
		snippet_with_applicability(cx, predicate.span, "<PREDICATE>", &mut appl));
	span_lint_and_sugg(cx, AND_THEN_THEN_SOME, span,
					   "use of `and_then` + `then_some` is equivelent to `filter`",
					   "use `Option::filter` instead",
					   sugg, appl);
}

fn is_then_some(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
	if let Some(def_id) = fn_def_id(cx, expr) {
		dbg!(match_def_path(
			cx, dbg!(def_id),
			&["core", "bool", "<impl bool>", "then_some"]))
	} else {
		dbg!(expr);
		false
	}
}

fn is_and_then(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
	if let Some(def_id) = fn_def_id(cx, expr) {
		dbg!(match_def_path(
			cx, dbg!(def_id),
			&["core", "option", "Option", "and_then"]))
	} else {
		false
	}
}
