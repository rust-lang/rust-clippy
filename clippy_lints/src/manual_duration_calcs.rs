use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, TyKind, QPath, Path, PrimTy, PathSegment, Ty};
use rustc_hir::def::Res;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::source_map::Spanned;
use rustc_ast::ast::{LitKind, FloatTy};

use crate::consts::{constant, Constant};
use crate::utils::paths;
use crate::utils::{match_type, snippet_with_applicability, span_lint_and_sugg};

declare_clippy_lint! {
    /// **What it does:** Checks for calculation of subsecond microseconds or milliseconds
    /// from other `Duration` methods.
    ///
    /// **Why is this bad?** It's more concise to call `Duration::subsec_micros()` or
    /// `Duration::subsec_millis()` than to calculate them.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust
    /// # use std::time::Duration;
    /// let dur = Duration::new(5, 0);
    ///
    /// // Bad
    /// let _micros = dur.subsec_nanos() / 1_000;
    /// let _millis = dur.subsec_nanos() / 1_000_000;
    /// let secs_f64 = diff.as_secs() as f64 + diff.subsec_milis() as f64 / 1_000.0;
    /// let secs_f64 = diff.as_secs() as f64 + diff.subsec_nanos() as f64 / 1_000_000_000.0;
    ///
    /// // Good
    /// let _micros = dur.subsec_micros();
    /// let _millis = dur.subsec_millis();
    /// let secs_f64 = diff.as_secs_f64();
    /// let secs_f64 = diff.as_secs_f64();
    /// ```
    pub MANUAL_DURATION_CALCS,
    complexity,
    "checks for calculation of subsecond microseconds or milliseconds"
}

declare_lint_pass!(ManualDurationCalcs => [MANUAL_DURATION_CALCS]);

impl<'tcx> ManualDurationCalcs {
    pub fn duration_subsec(&self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            if let ExprKind::Binary(Spanned { node: BinOpKind::Div, .. }, ref left, ref right) = expr.kind;
            if let ExprKind::MethodCall(ref method_path, _ , ref args, _) = left.kind;
            if match_type(cx, cx.typeck_results().expr_ty(&args[0]).peel_refs(), &paths::DURATION);
            if let Some((Constant::Int(divisor), _)) = constant(cx, cx.typeck_results(), right);
            then {
                let suggested_fn = match (method_path.ident.as_str().as_ref(), divisor) {
                    ("subsec_micros", 1_000) | ("subsec_nanos", 1_000_000) => "subsec_millis",
                    ("subsec_nanos", 1_000) => "subsec_micros",
                    _ => return,
                };
                let mut applicability = Applicability::MachineApplicable;
                span_lint_and_sugg(
                    cx,
                    MANUAL_DURATION_CALCS,
                    expr.span,
                    &format!("calling `{}()` is more concise than this calculation", suggested_fn),
                    "try",
                    format!(
                        "{}.{}()",
                        snippet_with_applicability(cx, args[0].span, "_", &mut applicability),
                        suggested_fn
                    ),
                    applicability,
                );
            }
        }
    }

    pub fn manual_re_implementation_as_secs_f64_for_div(&self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            // Extraction necessary expression(left)
            // let secs_f64 = diff.as_secs() as f64 + diff.subsec_nanos() as f64 / 1_000_000_000.0;
            //                ^^^^^^^^^^^^^^^^^^^^^
            if let ExprKind::Binary(Spanned { node: BinOpKind::Add, .. }, ref plus_cast_left_expr, ref rest)  = expr.kind;
            if let ExprKind::Cast( Expr { kind: ExprKind::MethodCall(PathSegment { ident: left_ident, .. }, _,  [left_receiver], _), .. }, Ty { kind: TyKind::Path(QPath::Resolved(None, Path { res: Res::PrimTy(left_type), .. })), .. } )= plus_cast_left_expr.kind;

            // Extraction necessary expression(right)
            // let secs_f64 = diff.as_secs() as f64 + diff.subsec_nanos() as f64 / 1_000_000_000.0;
            //                                        ^^^^^^^^^^^^^^^^^^^^^^^^^^
            if let ExprKind::Binary( Spanned { node: BinOpKind::Div, .. }, ref plus_cast_right_expr, ref divisor_expr) = rest.kind;
            if let ExprKind::Cast( Expr { kind: ExprKind::MethodCall(PathSegment { ident: right_ident, .. }, _, [right_receiver], _), .. }, Ty { kind: TyKind::Path(QPath::Resolved(None, Path { res: Res::PrimTy(right_type), .. })), .. } ) = plus_cast_right_expr.kind;

            // Extraction necessary expression(divisor literal)
            // let secs_f64 = diff.as_secs() as f64 + diff.subsec_nanos() as f64 / 1_000_000_000.0;
            //                                                                     ^^^^^^^^^^^^^^^
            if let ExprKind::Lit(Spanned { node: LitKind::Float(ref divisor_lit, _), .. }) = divisor_expr.kind;


            // Extraction necessary expression(method receiver)
            // let secs_f64 = diff.as_secs() as f64 + diff.subsec_nanos() as f64 / 1_000_000_000.0;
            //                ^^^^                    ^^^^
            if let Expr { kind: ExprKind::Path(QPath::Resolved(None, Path { segments: [PathSegment { ident: left_receiver_ident, .. }], .. })),.. } = left_receiver;
            if let Expr { kind: ExprKind::Path(QPath::Resolved(None, Path { segments: [PathSegment { ident: right_receiver_ident, .. }], .. })),.. } = right_receiver;

            // same receiver guard
            if left_receiver_ident == right_receiver_ident;

            then {
                let divisor: f64 = divisor_lit.as_str().parse().unwrap();

                let left_method_name: &str = &left_ident.as_str().to_string();
                let right_method_name: &str = &right_ident.as_str().to_string();

                let suggested_fn = match (left_method_name, left_type, right_method_name, right_type, divisor) {
                    ("as_secs", PrimTy::Float(FloatTy::F64), "subsec_nanos", PrimTy::Float(FloatTy::F64), div) if div == 1_000_000_000.0 => "as_secs_f64",
                    ("as_secs", PrimTy::Float(FloatTy::F32), "subsec_nanos", PrimTy::Float(FloatTy::F32), div) if div == 1_000_000_000.0 => "as_secs_f32",
                    ("subsec_nanos", PrimTy::Float(FloatTy::F64), "as_secs", PrimTy::Float(FloatTy::F64), div) if div == 1_000_000_000.0 => "as_secs_f64",
                    ("subsec_nanos", PrimTy::Float(FloatTy::F32), "as_secs", PrimTy::Float(FloatTy::F32), div) if div == 1_000_000_000.0 => "as_secs_f32",
                    ("as_secs", PrimTy::Float(FloatTy::F64), "subsec_millis", PrimTy::Float(FloatTy::F64), div ) if div == 1_000.0 => "as_secs_f64",
                    ("as_secs", PrimTy::Float(FloatTy::F32), "subsec_millis", PrimTy::Float(FloatTy::F32), div) if div == 1_000.0 => "as_secs_f32",
                    ("subsec_millis", PrimTy::Float(FloatTy::F64), "as_secs", PrimTy::Float(FloatTy::F64), div) if div == 1_000.0 => "as_secs_f64",
                    ("subsec_millis", PrimTy::Float(FloatTy::F32), "as_secs", PrimTy::Float(FloatTy::F32), div) if div == 1_000.0 => "as_secs_f32",
                    _ => return,
                };


                let mut applicability = Applicability::MachineApplicable;
                span_lint_and_sugg(
                    cx,
                    MANUAL_DURATION_CALCS,
                    expr.span,
                    &format!("no manual re-implementationa of the {}", suggested_fn),
                    "try",
                    format!(
                        "{}.{}()",
                        snippet_with_applicability(cx, left_receiver.span, "_", &mut applicability),
                        suggested_fn
                    ),
                    applicability,
                );
            }

        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualDurationCalcs {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        self.manual_re_implementation_as_secs_f64_for_div(cx, expr);
        self.duration_subsec(cx, expr);
    }
}
