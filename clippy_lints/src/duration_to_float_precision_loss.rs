use clippy_config::msrvs::{self, Msrv};
use clippy_utils::consts::{constant, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_from_proc_macro;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_middle::ty::{self, FloatTy};
use rustc_session::impl_lint_pass;
use rustc_span::{sym, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for conversions of a `Duration` to a floating point number where
    /// precision is lost.
    ///
    /// ### Why is this bad?
    /// Retaining the full precision of a duration is usually desired.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::time::Duration;
    /// let duration = Duration::from_nanos(1234500000);
    /// let _ = duration.as_millis() as f64;
    /// ```
    ///
    /// Use instead:
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// let duration = Duration::from_nanos(1234500000);
    /// let _ = duration.as_secs_f64() * 1000.0;
    /// ```
    ///
    /// Another motivating example happens when calculating number of seconds as a float with millisecond precision:
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// let duration = Duration::from_nanos(1234500000);
    /// let _ = duration.as_millis() as f64 / 1000.0;
    /// ```
    ///
    /// Use instead:
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// let duration = Duration::from_nanos(1234500000);
    /// let _ = duration.as_secs_f64();
    /// ```
    #[clippy::version = "1.79.0"]
    pub DURATION_TO_FLOAT_PRECISION_LOSS,
    style,
    "conversion from duration to float that cause loss of precision"
}

/// This struct implements the logic needed to apply the lint
#[derive(Debug)]
pub struct DurationToFloatPrecisionLoss {
    /// This vector is used to prevent applying the lint to a sub-expression
    lint_applications: Vec<Span>,
    /// `as_secs_f64` isn't applicable until 1.38.0
    msrv: Msrv,
}

impl DurationToFloatPrecisionLoss {
    /// Create a new instance of this lint
    pub fn new(msrv: Msrv) -> Self {
        Self {
            lint_applications: Vec::new(),
            msrv,
        }
    }

    fn should_emit_lint(&self, site: &LintApplicableSite) -> bool {
        let Some(last) = self.lint_applications.last() else {
            // if the stack is empty, then there is no outer expression
            return true;
        };

        // don't emit this instance of the lint, if there is a previous instance
        // which already covers this span
        !last.contains(site.expr_span)
    }
}

impl_lint_pass!(DurationToFloatPrecisionLoss => [DURATION_TO_FLOAT_PRECISION_LOSS]);

impl<'tcx> LateLintPass<'tcx> for DurationToFloatPrecisionLoss {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if in_external_macro(cx.sess(), expr.span) {
            // We should ignore macro from a foreign crate.
            return;
        }

        let site = match expr.kind {
            // This branch covers expressions like
            // ```rust
            // DURATION.as_secs() as f64;
            // DURATION.as_millis() as f32;
            // ```
            // where a duration is converted to floating point time units
            ExprKind::Cast(duration_expr, _) => {
                let Some(lint_site) = check_cast(cx, expr, duration_expr) else {
                    // does not match criteria - cast didn't have right contents
                    return;
                };

                lint_site
            },
            // This branch covers expressions like
            // ```rust
            // DURATION.as_micros() as f64 * 1e-6;
            // DURATION.as_nanos() as f64 / 1_000_000.0;
            // ```
            // where a duration is converted to floating point time units and then scaled by a constant factor
            ExprKind::Binary(op, lhs, rhs) => {
                if let ExprKind::Cast(duration_expr, _) = lhs.kind {
                    let Some(mut lint_site) = check_cast(cx, expr, duration_expr) else {
                        // does not match criteria - cast didn't have right contents
                        return;
                    };

                    let constant_factor = constant(cx, cx.typeck_results(), rhs);
                    let Some(adjustment) = check_adjustment(op.node, constant_factor) else {
                        // does not match criteria - adjustment was not shaped correctly
                        return;
                    };

                    lint_site.adjustment = Some(adjustment);
                    lint_site
                } else {
                    // does not match criteria - lhs of binary op was not a cast
                    return;
                }
            },
            _ => {
                // does not match criteria - expression was not a cast or a binary op
                return;
            },
        };

        if !self.msrv.meets(msrvs::DURATION_AS_SECS_FLOAT) {
            // rust version doesn't have required methods
            return;
        }

        // check to make sure this lint site is not already covered
        if self.should_emit_lint(&site) {
            site.emit_lint(cx);
            self.lint_applications.push(site.expr_span);
        }
    }

    fn check_expr_post(&mut self, _: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // clean up the stack
        if self
            .lint_applications
            .last()
            .is_some_and(|expr_span| expr_span == &expr.span)
        {
            self.lint_applications.pop();
        }
    }

    extract_msrv_attr!(LateContext);
}

#[allow(clippy::enum_glob_use)]
fn check_adjustment(
    bin_op: BinOpKind,
    constant_factor: Option<Constant<'_>>,
) -> Option<(AdjustmentDirection, AdjustmentScale)> {
    use AdjustmentDirection::*;
    use AdjustmentScale::*;

    fn check_adjustment_f64(bin_op: BinOpKind, factor: f64) -> Option<(AdjustmentDirection, AdjustmentScale)> {
        let scale = match factor {
            // initially assuming multiply
            1e3 => Thousand,
            1e6 => Million,
            1e9 => Billion,
            1e-3 => Thousand,
            1e-6 => Million,
            1e-9 => Billion,
            _ => return None, // does not match criteria - the factor is not in the predefined list
        };

        let direction = if factor > 1.0 { Positive } else { Negative };

        let direction = match bin_op {
            // swap direction when dividing
            BinOpKind::Div => direction.negate(),
            // keep mul the same
            BinOpKind::Mul => direction,
            _ => return None, // does not match criteria - the binary operation is not scaling the expression
        };

        Some((direction, scale))
    }

    match constant_factor? {
        Constant::F32(val) => check_adjustment_f64(bin_op, f64::from(val)),
        Constant::F64(val) => check_adjustment_f64(bin_op, val),
        _ => None, // does not match criteria - constant is not a float of the expected sizes
    }
}

fn check_cast<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    duration_expr: &'tcx Expr<'_>,
) -> Option<LintApplicableSite> {
    if let ExprKind::MethodCall(method_path, method_receiver_expr, [], _) = duration_expr.kind
        && !is_from_proc_macro(cx, expr)
    {
        let method_receiver_ty = cx.typeck_results().expr_ty(method_receiver_expr);
        if is_type_diagnostic_item(cx, method_receiver_ty.peel_refs(), sym::Duration) {
            let cast_expr_ty = cx.typeck_results().expr_ty(expr);

            let precision = match cast_expr_ty.kind() {
                ty::Float(FloatTy::F32) => FloatPrecision::F32,
                ty::Float(FloatTy::F64) => FloatPrecision::F64,
                _ => {
                    // does not match criteria - not the right kind of float
                    return None;
                },
            };

            let duration_method = match method_path.ident.as_str() {
                "as_secs" => DurationMethod::AsSeconds,
                "as_millis" => DurationMethod::AsMillis,
                "as_micros" => DurationMethod::AsMicros,
                "as_nanos" => DurationMethod::AsNanos,
                _ => {
                    // does not match criteria - not the type of duration methods we're interested in
                    return None;
                },
            };

            Some(LintApplicableSite {
                duration_method,
                precision,
                expr_span: expr.span,
                adjustment: None,
                duration_expr_span: method_receiver_expr.span,
            })
        } else {
            // does not match criteria - method receiver type is not Duration
            None
        }
    } else {
        // does not match criteria - not a method call expression in the expression of the cast
        None
    }
}

#[derive(Debug, Copy, Clone)]
enum FloatPrecision {
    F32,
    F64,
}

impl FloatPrecision {
    fn as_secs_float_method(self) -> &'static str {
        match self {
            FloatPrecision::F32 => "as_secs_f32",
            FloatPrecision::F64 => "as_secs_f64",
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[allow(clippy::enum_variant_names)]
enum DurationMethod {
    AsSeconds,
    AsMillis,
    AsMicros,
    AsNanos,
}

#[derive(Debug, Copy, Clone)]
enum AdjustmentScale {
    Thousand,
    Million,
    Billion,
}

#[derive(Debug, Copy, Clone)]
enum AdjustmentDirection {
    /// Decrease the number - multiply by factor < 1, or divide by factor > 1
    Negative,
    /// Increase the number - multiply by factor > 1, or divide by factor < 1
    Positive,
}

impl AdjustmentDirection {
    fn negate(self) -> Self {
        match self {
            AdjustmentDirection::Positive => AdjustmentDirection::Negative,
            AdjustmentDirection::Negative => AdjustmentDirection::Positive,
        }
    }
}

#[derive(Debug, Clone)]
struct LintApplicableSite {
    precision: FloatPrecision,
    expr_span: Span,
    duration_expr_span: Span,
    duration_method: DurationMethod,
    // When the adjustment is missing from the site, set this to None
    adjustment: Option<(AdjustmentDirection, AdjustmentScale)>,
}

impl LintApplicableSite {
    /// Return a string with the suggestion or None if this lint is not applicable.
    #[allow(clippy::enum_glob_use)]
    fn suggestion(&self) -> Option<String> {
        use AdjustmentScale::*;

        let (method_name, scale) = apply_adjustment(self.precision, self.duration_method, self.adjustment)?;
        let scale = match scale {
            None => "",
            Some(Thousand) => " * 1e3",
            Some(Million) => " * 1e6",
            Some(Billion) => " * 1e9",
        };

        let suggestion = format!("{method_name}(){scale}");
        Some(suggestion)
    }

    fn emit_lint(&self, cx: &LateContext<'_>) {
        let mut applicability = Applicability::MachineApplicable;
        let Some(suggested_expr) = self.suggestion() else {
            // lint not applicable
            return;
        };

        if !self.expr_span.eq_ctxt(self.duration_expr_span) {
            // different ctxt indicates a macro in the mix, can't apply lint
            return;
        }

        span_lint_and_sugg(
            cx,
            DURATION_TO_FLOAT_PRECISION_LOSS,
            self.expr_span,
            format!("calling `{suggested_expr}` is more precise than this calculation"),
            "try",
            format!(
                "{}.{suggested_expr}",
                snippet_with_applicability(cx, self.duration_expr_span, "_", &mut applicability)
            ),
            applicability,
        );
    }
}

#[allow(clippy::enum_glob_use, clippy::match_same_arms)]
fn apply_adjustment(
    precision: FloatPrecision,
    duration_method: DurationMethod,
    adjustment: Option<(AdjustmentDirection, AdjustmentScale)>,
) -> Option<(&'static str, Option<AdjustmentScale>)> {
    use AdjustmentDirection::*;
    use AdjustmentScale::*;
    use DurationMethod::*;

    let secs_method = precision.as_secs_float_method();

    let result = match (duration_method, adjustment) {
        // `as_{secs,millis,micros,nanos}() as f64`
        (AsSeconds, None) => (secs_method, None),
        (AsMillis, None) => (secs_method, Some(Thousand)),
        (AsMicros, None) => (secs_method, Some(Million)),
        (AsNanos, None) => return None, // `as_nanos() as f{32,64}` is already max precision
        // `as_secs() as f64 * scale`
        (AsSeconds, Some((Positive, scale))) => (secs_method, Some(scale)),
        (AsSeconds, Some((Negative, _))) => return None, // lint isn't applicable to prefixes over unit (1)
        // `as_millis() as f64 * scale`
        (AsMillis, Some((Positive, Thousand))) => (secs_method, Some(Million)),
        (AsMillis, Some((Positive, Million))) => (secs_method, Some(Billion)),
        (AsMillis, Some((Positive, _))) => return None, // lint isn't applicable to prefixes below nanos
        (AsMillis, Some((Negative, Thousand))) => (secs_method, None),
        (AsMillis, Some((Negative, _))) => return None, // lint isn't applicable to prefixes over unit (1)
        // `as_micros() as f64 * scale`
        (AsMicros, Some((Positive, Thousand))) => (secs_method, Some(Billion)),
        (AsMicros, Some((Positive, _))) => return None, // lint isn't applicable to prefixes below nanos
        (AsMicros, Some((Negative, Thousand))) => (secs_method, Some(Thousand)),
        (AsMicros, Some((Negative, Million))) => (secs_method, None),
        (AsMicros, Some((Negative, _))) => return None, // lint isn't applicable to prefixes over unit (1)
        // `as_nanos() as f64 * scale`
        (AsNanos, Some((Positive, _))) => return None, // lint isn't applicable to prefixes below nanos
        (AsNanos, Some((Negative, Thousand | Million))) => {
            // if the expression is converting nanos to micros or millis, its already at max precision
            return None;
        },
        (AsNanos, Some((Negative, Billion))) => {
            // this suggestion won't improve the precision, but it is more succint
            (secs_method, None)
        },
    };

    Some(result)
}
