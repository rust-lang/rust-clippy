use clippy_config::msrvs::{self, Msrv};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::root_macro_call;
use clippy_utils::sugg::Sugg;
use clippy_utils::{higher, in_constant, path_to_local, peel_ref_operators};
use rustc_ast::ast::RangeLimits;
use rustc_ast::LitKind::{self, Byte, Char};
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::{BodyId, Closure, Expr, ExprKind, FnDecl, HirId, PatKind, RangeEnd, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;
use rustc_span::def_id::DefId;
use rustc_span::{sym, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Suggests to use dedicated built-in methods,
    /// `is_ascii_(lowercase|uppercase|digit|hexdigit)` for checking on corresponding
    /// ascii range
    ///
    /// ### Why is this bad?
    /// Using the built-in functions is more readable and makes it
    /// clear that it's not a specific subset of characters, but all
    /// ASCII (lowercase|uppercase|digit|hexdigit) characters.
    /// ### Example
    /// ```no_run
    /// fn main() {
    ///     assert!(matches!('x', 'a'..='z'));
    ///     assert!(matches!(b'X', b'A'..=b'Z'));
    ///     assert!(matches!('2', '0'..='9'));
    ///     assert!(matches!('x', 'A'..='Z' | 'a'..='z'));
    ///     assert!(matches!('C', '0'..='9' | 'a'..='f' | 'A'..='F'));
    ///
    ///     ('0'..='9').contains(&'0');
    ///     ('a'..='z').contains(&'a');
    ///     ('A'..='Z').contains(&'A');
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn main() {
    ///     assert!('x'.is_ascii_lowercase());
    ///     assert!(b'X'.is_ascii_uppercase());
    ///     assert!('2'.is_ascii_digit());
    ///     assert!('x'.is_ascii_alphabetic());
    ///     assert!('C'.is_ascii_hexdigit());
    ///
    ///     '0'.is_ascii_digit();
    ///     'a'.is_ascii_lowercase();
    ///     'A'.is_ascii_uppercase();
    /// }
    /// ```
    #[clippy::version = "1.67.0"]
    pub MANUAL_IS_ASCII_CHECK,
    style,
    "use dedicated method to check ascii range"
}
impl_lint_pass!(ManualIsAsciiCheck => [MANUAL_IS_ASCII_CHECK]);

pub struct ManualIsAsciiCheck {
    msrv: Msrv,
    closure_params: FxHashMap<HirId, Span>,
}

impl ManualIsAsciiCheck {
    #[must_use]
    pub fn new(msrv: Msrv) -> Self {
        Self {
            msrv,
            closure_params: FxHashMap::default(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum CharRange {
    /// 'a'..='z' | b'a'..=b'z'
    LowerChar,
    /// 'A'..='Z' | b'A'..=b'Z'
    UpperChar,
    /// `AsciiLower` | `AsciiUpper`
    FullChar,
    /// '0..=9'
    Digit,
    /// 'a..=f'
    LowerHexLetter,
    /// 'A..=F'
    UpperHexLetter,
    /// '0..=9' | 'a..=f' | 'A..=F'
    HexDigit,
    Otherwise,
}

impl<'tcx> LateLintPass<'tcx> for ManualIsAsciiCheck {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if !self.msrv.meets(msrvs::IS_ASCII_DIGIT) {
            return;
        }

        if in_constant(cx, expr.hir_id) && !self.msrv.meets(msrvs::IS_ASCII_DIGIT_CONST) {
            return;
        }

        if let ExprKind::Closure(Closure { fn_decl, body, .. }) = expr.kind {
            collect_params_with_inferred_ty(cx, *body, fn_decl, &mut self.closure_params);
        }

        if let Some(macro_call) = root_macro_call(expr.span)
            && is_matches_macro(cx, macro_call.def_id)
        {
            if let ExprKind::Match(recv, [arm, ..], _) = expr.kind {
                let range = check_pat(&arm.pat.kind);
                check_is_ascii(cx, macro_call.span, recv, &range, None);
            }
        } else if let ExprKind::MethodCall(path, receiver, [arg], ..) = expr.kind
            && path.ident.name == sym!(contains)
            && let Some(higher::Range {
                start: Some(start),
                end: Some(end),
                limits: RangeLimits::Closed,
            }) = higher::Range::hir(receiver)
        {
            let arg_ty = cx.typeck_results().expr_ty(arg).peel_refs();
            if matches!(arg_ty.kind(), ty::Param(_)) {
                return;
            }

            let arg = peel_ref_operators(cx, arg);
            let ty_sugg = get_ty_sugg(arg, &self.closure_params, start);
            let range = check_range(start, end);
            check_is_ascii(cx, expr.span, peel_ref_operators(cx, arg), &range, ty_sugg);
        }
    }

    fn check_mod(&mut self, _: &LateContext<'tcx>, _: &'tcx rustc_hir::Mod<'tcx>, _: HirId) {
        self.closure_params = FxHashMap::default();
    }

    extract_msrv_attr!(LateContext);
}

fn get_ty_sugg(arg: &Expr<'_>, map: &FxHashMap<HirId, Span>, range_expr: &Expr<'_>) -> Option<(Span, &'static str)> {
    let ExprKind::Lit(lit) = range_expr.kind else {
        return None;
    };
    let sugg_ty_span = path_to_local(arg).and_then(|id| map.get(&id)).copied()?;
    let sugg_ty_str = match lit.node {
        LitKind::Char(_) => "char",
        LitKind::Byte(_) => "u8",
        _ => return None,
    };

    Some((sugg_ty_span, sugg_ty_str))
}

/// Collect closure params' `HirId` and `Span` pairs into a map,
/// if they have implicit (inferred) `ty`.
fn collect_params_with_inferred_ty(
    cx: &LateContext<'_>,
    body_id: BodyId,
    fn_decl: &FnDecl<'_>,
    map: &mut FxHashMap<HirId, Span>,
) {
    let params = cx.tcx.hir().body(body_id).params.iter();
    let fn_decl_input_tys = fn_decl.inputs.iter();
    for (id, span) in params.zip(fn_decl_input_tys).filter_map(|(param, ty)| {
        if let TyKind::Infer = ty.kind {
            Some((param.pat.hir_id, param.span))
        } else {
            None
        }
    }) {
        map.insert(id, span);
    }
}

fn check_is_ascii(cx: &LateContext<'_>, span: Span, recv: &Expr<'_>, range: &CharRange, ty_sugg: Option<(Span, &str)>) {
    if let Some(sugg) = match range {
        CharRange::UpperChar => Some("is_ascii_uppercase"),
        CharRange::LowerChar => Some("is_ascii_lowercase"),
        CharRange::FullChar => Some("is_ascii_alphabetic"),
        CharRange::Digit => Some("is_ascii_digit"),
        CharRange::HexDigit => Some("is_ascii_hexdigit"),
        CharRange::Otherwise | CharRange::LowerHexLetter | CharRange::UpperHexLetter => None,
    } {
        let default_snip = "..";
        let mut app = Applicability::MachineApplicable;
        let recv = Sugg::hir_with_context(cx, recv, span.ctxt(), default_snip, &mut app).maybe_par();

        span_lint_and_then(
            cx,
            MANUAL_IS_ASCII_CHECK,
            span,
            "manual check for common ascii range",
            |diag| {
                diag.span_suggestion(span, "try", format!("{recv}.{sugg}()"), app);
                if let Some((ty_span, ty_str)) = ty_sugg {
                    diag.span_suggestion(
                        ty_span,
                        "also make sure to label the correct type",
                        format!("{recv}: {ty_str}"),
                        app,
                    );
                }
            },
        );
    }
}

fn check_pat(pat_kind: &PatKind<'_>) -> CharRange {
    match pat_kind {
        PatKind::Or(pats) => {
            let ranges = pats.iter().map(|p| check_pat(&p.kind)).collect::<Vec<_>>();

            if ranges.len() == 2 && ranges.contains(&CharRange::UpperChar) && ranges.contains(&CharRange::LowerChar) {
                CharRange::FullChar
            } else if ranges.len() == 3
                && ranges.contains(&CharRange::Digit)
                && ranges.contains(&CharRange::LowerHexLetter)
                && ranges.contains(&CharRange::UpperHexLetter)
            {
                CharRange::HexDigit
            } else {
                CharRange::Otherwise
            }
        },
        PatKind::Range(Some(start), Some(end), kind) if *kind == RangeEnd::Included => check_range(start, end),
        _ => CharRange::Otherwise,
    }
}

fn check_range(start: &Expr<'_>, end: &Expr<'_>) -> CharRange {
    if let ExprKind::Lit(start_lit) = &start.kind
        && let ExprKind::Lit(end_lit) = &end.kind
    {
        match (&start_lit.node, &end_lit.node) {
            (Char('a'), Char('z')) | (Byte(b'a'), Byte(b'z')) => CharRange::LowerChar,
            (Char('A'), Char('Z')) | (Byte(b'A'), Byte(b'Z')) => CharRange::UpperChar,
            (Char('a'), Char('f')) | (Byte(b'a'), Byte(b'f')) => CharRange::LowerHexLetter,
            (Char('A'), Char('F')) | (Byte(b'A'), Byte(b'F')) => CharRange::UpperHexLetter,
            (Char('0'), Char('9')) | (Byte(b'0'), Byte(b'9')) => CharRange::Digit,
            _ => CharRange::Otherwise,
        }
    } else {
        CharRange::Otherwise
    }
}

fn is_matches_macro(cx: &LateContext<'_>, macro_def_id: DefId) -> bool {
    if let Some(name) = cx.tcx.get_diagnostic_name(macro_def_id) {
        return sym::matches_macro == name;
    }

    false
}
