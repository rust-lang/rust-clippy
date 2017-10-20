use rustc::lint::{LintArray, LintPass, EarlyLintPass, EarlyContext};
use syntax::ast::{Ty, TyKind, Item, ItemKind, Path, Expr, ExprKind, LitKind};
use syntax_pos::symbol::InternedString;
use syntax_pos::Span;
use utils::{in_macro, span_lint_and_then};
use std::f64;
use std::f32;
const F32_MAX_SIGNIFICANT_DIGITS: usize = f32::DIGITS as usize;
const F64_MAX_SIGNIFICANT_DIGITS: usize = f64::DIGITS as usize;

declare_lint! {
    pub FLOAT_EXCESSIVE_PRECISION,
    Warn,
    "declaring a float with too much precision"
}

pub struct FloatExcessivePrecision;

impl LintPass for FloatExcessivePrecision {
    fn get_lints(&self) -> LintArray {
        lint_array!(FLOAT_EXCESSIVE_PRECISION)
    }
}


impl EarlyLintPass for FloatExcessivePrecision {
    fn check_item(&mut self, cx: &EarlyContext, item: &Item) {
        if !in_macro(item.span) {
            // Match only constants...
            if let ItemKind::Const(ref var_type, ref expr) = item.node {
                // println!("{:?}", expr);
                self.visit_constant_declaration(var_type, expr, cx);
            }

        }
    }
}

impl FloatExcessivePrecision {
    fn is_f32(path: &Path) -> bool {
        for p in &path.segments {
            if p.identifier.name == "f32" {
                return true;
            }
        }
        return false;
    }

    fn is_f64(path: &Path) -> bool {
        for p in &path.segments {
            if p.identifier.name == "f64" {
                return true;
            }
        }
        return false;
    }

    fn is_float(path: &Path) -> bool {
        FloatExcessivePrecision::is_f32(path) || FloatExcessivePrecision::is_f64(path)
    }

    // Take an expression, and return a return a float in the form of a string if
    // that expression was a literal.
    fn strip_float(expr: &Expr) -> Option<InternedString> {
        match expr.node {
            ExprKind::Lit(ref lit) => {
                match lit.node {
                    LitKind::Float(symbol, _) => Some(symbol.as_str()),
                    LitKind::FloatUnsuffixed(symbol) => Some(symbol.as_str()),
                    _ => {
                        println!("Other : {:?}", lit.node);
                        None
                    },
                }
            },
            _ => {
                println!("Expr kind :\n {:?}", expr.node);
                None
            },
        }
    }

    // Take a mangled float (aka 0.123_456_289f32) and return a simpler float
    // (0.123456789).
    fn standardize_float(float: &InternedString) -> String {
        float.replace('_', "").replace("f32", "").replace("f64", "")
    }

    // Take a float in the form of a string and analyze it to see if it is too
    // precise.
    fn analyze_float(float: &InternedString, is_f32: bool, cx: &EarlyContext, current_span: &Span) {
        let real_float = FloatExcessivePrecision::standardize_float(float);
        let string_slice = real_float.rsplit('.');
        let substrings: Vec<&str> = string_slice.collect::<Vec<&str>>();
        let decimals = substrings[0];
        if (decimals.len() > F32_MAX_SIGNIFICANT_DIGITS) && is_f32 {
            let mut truncated_float = String::new();
            truncated_float.push_str(substrings[1]);
            truncated_float.push_str(".");
            truncated_float.push_str(substrings[0].split_at(F32_MAX_SIGNIFICANT_DIGITS).0);
            span_lint_and_then(
                cx,
                FLOAT_EXCESSIVE_PRECISION,
                *current_span,
                "This f32 is too big to be represented accuratly.",
                |db| {db.span_suggestion(*current_span, "consider reducing it's size",truncated_float);}
            );
        }
        if !is_f32 && (decimals.len() > F64_MAX_SIGNIFICANT_DIGITS) {
            let mut truncated_float = String::new();
            truncated_float.push_str(substrings[1]);
            truncated_float.push_str(".");
            truncated_float.push_str(substrings[0].split_at(F64_MAX_SIGNIFICANT_DIGITS).0);
            span_lint_and_then(
                cx,
                FLOAT_EXCESSIVE_PRECISION,
                *current_span,
                "This f32 is too big to be represented accuratly.",
                |db| {db.span_suggestion(*current_span, "consider reducing it's size",truncated_float);}
            );
        }
    }

    fn visit_constant_declaration(&self, ty: &Ty, expr: &Expr, cx: &EarlyContext) {
        if let TyKind::Path(_, ref path) = ty.node {
            if FloatExcessivePrecision::is_float(&path) {
                if let Some(float) = FloatExcessivePrecision::strip_float(expr) {
                    FloatExcessivePrecision::analyze_float(
                        &float,
                        FloatExcessivePrecision::is_f32(&path),
                        cx,
                        &expr.span,
                    );

                }
            }
        }
    }
}
