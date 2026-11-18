use crate::question_mark::{QUESTION_MARK, QuestionMark};
use clippy_config::types::MatchLintBehaviour;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::IfLetOrMatch;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _};
use clippy_utils::source::snippet_with_context;
use clippy_utils::{is_lint_allowed, is_never_expr, is_wild, msrvs, pat_and_expr_can_be_question_mark, peel_blocks};
use rustc_ast::BindingMode;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::def::{CtorOf, DefKind, Res};
use rustc_hir::{
    Arm, BlockCheckMode, Expr, ExprKind, MatchSource, Pat, PatExpr, PatExprKind, PatField, PatKind, QPath, Stmt,
    StmtKind,
};
use rustc_lint::{LateContext, LintContext as _};
use rustc_span::Span;
use rustc_span::symbol::{Symbol, sym};
use std::slice;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Warn of cases where `let...else` could be used
    ///
    /// ### Why is this bad?
    ///
    /// `let...else` provides a standard construct for this pattern
    /// that people can easily recognize. It's also more compact.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// # let w = Some(0);
    /// let v = if let Some(v) = w { v } else { return };
    /// ```
    ///
    /// Could be written:
    ///
    /// ```no_run
    /// # fn main () {
    /// # let w = Some(0);
    /// let Some(v) = w else { return };
    /// # }
    /// ```
    #[clippy::version = "1.67.0"]
    pub MANUAL_LET_ELSE,
    pedantic,
    "manual implementation of a let...else statement"
}

impl<'tcx> QuestionMark {
    pub(crate) fn check_manual_let_else(&self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(local) = stmt.kind
            && let Some(init) = local.init
            && local.els.is_none()
            && local.ty.is_none()
            && init.span.ctxt() == stmt.span.ctxt()
            && let Some(if_let_or_match) = IfLetOrMatch::parse(cx, init)
            && !stmt.span.in_external_macro(cx.sess().source_map())
            && self.msrv.meets(cx, msrvs::LET_ELSE)
        {
            match if_let_or_match {
                IfLetOrMatch::IfLet(if_let_expr, let_pat, if_then, if_else, ..) => {
                    if let Some(ident_map) = expr_simple_identity_map(local.pat, let_pat, if_then)
                        && let Some(if_else) = if_else
                        && is_never_expr(cx, if_else).is_some()
                        && let qm_allowed = is_lint_allowed(cx, QUESTION_MARK, stmt.hir_id)
                        && (qm_allowed || pat_and_expr_can_be_question_mark(cx, let_pat, if_else).is_none())
                    {
                        emit_manual_let_else(cx, stmt.span, if_let_expr, &ident_map, let_pat, if_else);
                    }
                },
                IfLetOrMatch::Match(match_expr, arms, source) => {
                    if self.matches_behaviour == MatchLintBehaviour::Never {
                        return;
                    }
                    if source != MatchSource::Normal {
                        return;
                    }
                    // Any other number than two arms doesn't (necessarily)
                    // have a trivial mapping to let else.
                    if arms.len() != 2 {
                        return;
                    }
                    // Guards don't give us an easy mapping either
                    if arms.iter().any(|arm| arm.guard.is_some()) {
                        return;
                    }
                    let check_types = self.matches_behaviour == MatchLintBehaviour::WellKnownTypes;
                    let diverging_arm_opt = arms.iter().enumerate().find(|(_, arm)| {
                        is_never_expr(cx, arm.body).is_some() && pat_allowed_for_else(cx, arm.pat, check_types)
                    });
                    let Some((idx, diverging_arm)) = diverging_arm_opt else {
                        return;
                    };

                    let pat_arm = &arms[1 - idx];
                    // If the non-diverging arm is the first one, its pattern can be reused in a let/else statement.
                    // However, if it arrives in second position, its pattern may cover some cases already covered
                    // by the diverging one.
                    if idx == 0 && !is_arms_disjointed(cx, diverging_arm, pat_arm) {
                        return;
                    }

                    let Some(ident_map) = expr_simple_identity_map(local.pat, pat_arm.pat, pat_arm.body) else {
                        return;
                    };

                    emit_manual_let_else(cx, stmt.span, match_expr, &ident_map, pat_arm.pat, diverging_arm.body);
                },
            }
        }
    }
}

/// Checks if the patterns of the arms are disjointed. Currently, we only support patterns of simple
/// enum variants without nested patterns or bindings.
///
/// TODO: Support more complex patterns.
fn is_arms_disjointed(cx: &LateContext<'_>, arm1: &Arm<'_>, arm2: &Arm<'_>) -> bool {
    if arm1.guard.is_some() || arm2.guard.is_some() {
        return false;
    }

    is_enum_variant(cx, arm1.pat) && is_enum_variant(cx, arm2.pat)
}

/// Returns `true` if the given pattern is a variant of an enum.
pub fn is_enum_variant(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    let path = match pat.kind {
        PatKind::Struct(ref qpath, fields, _)
            if fields
                .iter()
                .all(|field| is_wild(field.pat) || matches!(field.pat.kind, PatKind::Binding(..))) =>
        {
            (qpath, pat.hir_id)
        },
        PatKind::TupleStruct(ref qpath, pats, _)
            if pats
                .iter()
                .all(|pat| is_wild(pat) || matches!(pat.kind, PatKind::Binding(..))) =>
        {
            (qpath, pat.hir_id)
        },
        PatKind::Expr(e) if let Some((qpath, id)) = e.opt_qpath() => (qpath, id),
        _ => return false,
    };
    let res = path.res(cx);
    matches!(
        res,
        Res::Def(DefKind::Variant, ..) | Res::Def(DefKind::Ctor(CtorOf::Variant, _), _)
    )
}

fn emit_manual_let_else(
    cx: &LateContext<'_>,
    span: Span,
    expr: &Expr<'_>,
    ident_map: &FxHashMap<Symbol, (&Pat<'_>, BindingMode)>,
    pat: &Pat<'_>,
    else_body: &Expr<'_>,
) {
    // This is far from perfect, for example there needs to be:
    // * renamings of the bindings for many `PatKind`s like slices, etc.
    // * limitations in the existing replacement algorithms
    // * unused binding collision detection with existing ones
    // for this to be machine applicable.
    let mut app = Applicability::HasPlaceholders;
    let (sn_expr, _) = snippet_with_context(cx, expr.span, span.ctxt(), "", &mut app);
    let (sn_else, else_is_mac_call) = snippet_with_context(cx, else_body.span, span.ctxt(), "", &mut app);

    let else_bl = if let ExprKind::Block(block, None) = else_body.kind
        && matches!(block.rules, BlockCheckMode::DefaultBlock)
        && !else_is_mac_call
    {
        sn_else.into_owned()
    } else {
        format!("{{ {sn_else} }}")
    };
    let Some(sn_bl) = replace_in_pattern(cx, span, ident_map, pat, &mut app, true) else {
        return;
    };
    let sugg = if sn_expr.ends_with('}') {
        // let-else statement expressions are not allowed to end with `}`
        // https://rust-lang.github.io/rfcs/3137-let-else.html#let-pattern--if--else--else-
        format!("let {sn_bl} = ({sn_expr}) else {else_bl};")
    } else {
        format!("let {sn_bl} = {sn_expr} else {else_bl};")
    };

    span_lint_and_then(
        cx,
        MANUAL_LET_ELSE,
        span,
        "this could be rewritten as `let...else`",
        |diag| {
            diag.span_suggestion(span, "consider writing", sugg, app);
        },
    );
}

/// Replaces the locals in the pattern
///
/// For this example:
///
/// ```ignore
/// let (a, FooBar { b, c }) = if let Bar { Some(a_i), b_i } = ex { (a_i, b_i) } else { return };
/// ```
///
/// We have:
///
/// ```ignore
/// pat: Bar { Some(a_i), b_i }
/// ident_map: (a_i) -> (a), (b_i) -> (FooBar { b, c })
/// ```
///
/// We return:
///
/// ```ignore
/// Bar { Some(a), b_i: FooBar { b, c } }
/// ```
fn replace_in_pattern(
    cx: &LateContext<'_>,
    span: Span,
    ident_map: &FxHashMap<Symbol, (&Pat<'_>, BindingMode)>,
    pat: &Pat<'_>,
    app: &mut Applicability,
    top_level: bool,
) -> Option<String> {
    // The labeled block lets unsupported cases fall back to the original pattern snippet.
    'a: {
        // If the ident map is empty, there is no replacement to do.
        // The code following this if assumes a non-empty ident_map.
        if ident_map.is_empty() {
            break 'a;
        }

        match pat.kind {
            PatKind::Binding(ann, _id, binding_name, opt_subpt) => {
                match (ident_map.get(&binding_name.name), opt_subpt) {
                    (Some((pat_to_put, binding_mode)), opt_subpt) => {
                        let sn_pfx = binding_mode.prefix_str();
                        let (sn_ptp, _) = snippet_with_context(cx, pat_to_put.span, span.ctxt(), "", app);
                        if let Some(subpt) = opt_subpt {
                            if !sn_pfx.is_empty() {
                                return None;
                            }
                            if matches!(pat_to_put.kind, PatKind::Binding(_, _, _, None)) {
                                let subpt = replace_in_pattern(cx, span, ident_map, subpt, app, false)?;
                                return Some(format!("{sn_ptp} @ {subpt}"));
                            }
                            return merge_patterns(cx, span, ident_map, pat_to_put, subpt, app);
                        }
                        return Some(format!("{sn_pfx}{sn_ptp}"));
                    },
                    (None, Some(subpt)) => {
                        let subpt = replace_in_pattern(cx, span, ident_map, subpt, app, false)?;
                        // scanning for a value that matches is not sensitive to order
                        #[expect(rustc::potential_query_instability)]
                        if ident_map.values().any(|(other_pat, _)| {
                            if let PatKind::Binding(_, _, other_name, _) = other_pat.kind {
                                other_name == binding_name
                            } else {
                                false
                            }
                        }) {
                            // this name is shadowed, and, therefore, not usable
                            return Some(subpt);
                        }
                        let binding_pfx = ann.prefix_str();
                        return Some(format!("{binding_pfx}{binding_name} @ {subpt}"));
                    },
                    (None, None) => break 'a,
                }
            },
            PatKind::Or(pats) => {
                let patterns = pats
                    .iter()
                    .map(|pat| replace_in_pattern(cx, span, ident_map, pat, app, false))
                    .collect::<Option<Vec<_>>>()?;
                let or_pat = patterns.join(" | ");
                if top_level {
                    return Some(format!("({or_pat})"));
                }
                return Some(or_pat);
            },
            PatKind::Struct(path, fields, dot_dot) => {
                return Some(replace_in_struct_pattern(
                    cx,
                    span,
                    ident_map,
                    path,
                    fields,
                    dot_dot.is_some(),
                    app,
                ));
            },
            // Replace the variable name iff `TupleStruct` has one argument like `Variant(v)`.
            PatKind::TupleStruct(ref w, args, dot_dot_pos) => {
                let mut args = args
                    .iter()
                    .map(|pat| replace_in_pattern(cx, span, ident_map, pat, app, false))
                    .collect::<Option<Vec<_>>>()?;
                if let Some(pos) = dot_dot_pos.as_opt_usize() {
                    args.insert(pos, "..".to_owned());
                }
                let args = args.join(", ");
                let sn_wrapper = cx.sess().source_map().span_to_snippet(w.span()).unwrap_or_default();
                return Some(format!("{sn_wrapper}({args})"));
            },
            PatKind::Tuple(args, dot_dot_pos) => {
                let mut args = args
                    .iter()
                    .map(|pat| replace_in_pattern(cx, span, ident_map, pat, app, false))
                    .collect::<Option<Vec<_>>>()?;
                if let Some(pos) = dot_dot_pos.as_opt_usize() {
                    args.insert(pos, "..".to_owned());
                }
                let args = args.join(", ");
                return Some(format!("({args})"));
            },
            _ => {},
        }
    }
    let (sn_pat, _) = snippet_with_context(cx, pat.span, span.ctxt(), "", app);
    Some(sn_pat.into_owned())
}

fn replace_in_struct_pattern(
    cx: &LateContext<'_>,
    span: Span,
    ident_map: &FxHashMap<Symbol, (&Pat<'_>, BindingMode)>,
    path: QPath<'_>,
    fields: &[PatField<'_>],
    has_rest: bool,
    app: &mut Applicability,
) -> String {
    let fields = fields
        .iter()
        .map(|field| {
            if let PatKind::Binding(_, _, name, None) = field.pat.kind
                && let Some((replacement, binding_mode)) = ident_map.get(&name.name)
            {
                let prefix = binding_mode.prefix_str();
                let (field_name, _) = snippet_with_context(cx, field.ident.span, span.ctxt(), "", app);
                let (replacement, _) = snippet_with_context(cx, replacement.span, span.ctxt(), "", app);
                if field_name == replacement {
                    return format!("{prefix}{field_name}");
                }
                return format!("{field_name}: {prefix}{replacement}");
            }
            let (field, _) = snippet_with_context(cx, field.span, span.ctxt(), "", app);
            field.into_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    let rest = if has_rest { ", .." } else { "" };
    let (path, _) = snippet_with_context(cx, path.span(), span.ctxt(), "", app);
    format!("{path} {{ {fields}{rest} }}")
}

/// Combines an irrefutable binding pattern with the refutable subpattern of an `@` binding.
///
/// This only handles combinations whose structure can be aligned without type-dependent pattern
/// reasoning. Returning `None` suppresses the lint instead of producing an invalid `@`
/// pattern with a non-binding left-hand side.
fn merge_patterns(
    cx: &LateContext<'_>,
    span: Span,
    ident_map: &FxHashMap<Symbol, (&Pat<'_>, BindingMode)>,
    binding_pat: &Pat<'_>,
    subpat: &Pat<'_>,
    app: &mut Applicability,
) -> Option<String> {
    if matches!(subpat.kind, PatKind::Wild) {
        let (snippet, _) = snippet_with_context(cx, binding_pat.span, span.ctxt(), "", app);
        return Some(snippet.into_owned());
    }

    match (binding_pat.kind, subpat.kind) {
        (PatKind::Wild, _) => replace_in_pattern(cx, span, ident_map, subpat, app, false),
        (PatKind::Binding(_, _, _, None), _) => {
            let (binding, _) = snippet_with_context(cx, binding_pat.span, span.ctxt(), "", app);
            let subpat = replace_in_pattern(cx, span, ident_map, subpat, app, true)?;
            Some(format!("{binding} @ {subpat}"))
        },
        (PatKind::Tuple(binding_pats, binding_rest), PatKind::Tuple(subpats, subpat_rest))
            if binding_rest.as_opt_usize().is_none()
                && subpat_rest.as_opt_usize().is_none()
                && binding_pats.len() == subpats.len() =>
        {
            let pats = binding_pats
                .iter()
                .zip(subpats)
                .map(|(binding_pat, subpat)| merge_patterns(cx, span, ident_map, binding_pat, subpat, app))
                .collect::<Option<Vec<_>>>()?;
            let trailing_comma = if pats.len() == 1 { "," } else { "" };
            Some(format!("({}{trailing_comma})", pats.join(", ")))
        },
        (PatKind::Slice(binding_before, None, binding_after), PatKind::Slice(subpat_before, None, subpat_after))
            if binding_before.len() + binding_after.len() == subpat_before.len() + subpat_after.len() =>
        {
            let binding_pats = binding_before.iter().chain(binding_after);
            let subpats = subpat_before.iter().chain(subpat_after);
            let pats = binding_pats
                .zip(subpats)
                .map(|(binding_pat, subpat)| merge_patterns(cx, span, ident_map, binding_pat, subpat, app))
                .collect::<Option<Vec<_>>>()?;
            Some(format!("[{}]", pats.join(", ")))
        },
        (PatKind::TupleStruct(lhs_path, lhs_args, lhs_rest), PatKind::TupleStruct(rhs_path, rhs_args, rhs_rest))
            if cx.qpath_res(&lhs_path, binding_pat.hir_id) == cx.qpath_res(&rhs_path, subpat.hir_id)
                && lhs_rest.as_opt_usize().is_none()
                && rhs_rest.as_opt_usize().is_none()
                && lhs_args.len() == rhs_args.len() =>
        {
            let merged = lhs_args
                .iter()
                .zip(rhs_args)
                .map(|(binding_pat, subpat)| merge_patterns(cx, span, ident_map, binding_pat, subpat, app))
                .collect::<Option<Vec<_>>>()?;
            let (path_snippet, _) = snippet_with_context(cx, rhs_path.span(), span.ctxt(), "", app);
            Some(format!("{path_snippet}({})", merged.join(", ")))
        },
        (PatKind::Struct(lhs_path, lhs_fields, lhs_rest), PatKind::Struct(rhs_path, rhs_fields, rhs_rest))
            if cx.qpath_res(&lhs_path, binding_pat.hir_id) == cx.qpath_res(&rhs_path, subpat.hir_id) =>
        {
            let mut fields = Vec::with_capacity(lhs_fields.len().max(rhs_fields.len()));

            for subpat_field in rhs_fields {
                let field_name = subpat_field.ident.name;
                let merged_field =
                    if let Some(binding_field) = lhs_fields.iter().find(|field| field.ident.name == field_name) {
                        merge_patterns(cx, span, ident_map, binding_field.pat, subpat_field.pat, app)?
                    } else {
                        replace_in_pattern(cx, span, ident_map, subpat_field.pat, app, false)?
                    };
                let field = subpat_field.ident.to_string();
                fields.push(if merged_field == field {
                    merged_field
                } else {
                    format!("{field}: {merged_field}")
                });
            }

            for binding_field in lhs_fields {
                if rhs_fields
                    .iter()
                    .all(|field| field.ident.name != binding_field.ident.name)
                {
                    let (pat, _) = snippet_with_context(cx, binding_field.pat.span, span.ctxt(), "", app);
                    let field = binding_field.ident.to_string();
                    fields.push(if pat == field {
                        pat.into_owned()
                    } else {
                        format!("{field}: {pat}")
                    });
                }
            }

            let rest = if lhs_rest.is_some() && rhs_rest.is_some() {
                if fields.is_empty() { ".." } else { ", .." }
            } else {
                ""
            };
            let (path_snippet, _) = snippet_with_context(cx, rhs_path.span(), span.ctxt(), "", app);
            Some(format!("{path_snippet} {{ {}{rest} }}", fields.join(", ")))
        },
        _ => None,
    }
}

fn pat_allowed_for_else(cx: &LateContext<'_>, pat: &'_ Pat<'_>, check_types: bool) -> bool {
    // Check whether the pattern contains any bindings, as the
    // binding might potentially be used in the body.
    // TODO: only look for *used* bindings.
    let mut has_bindings = false;
    pat.each_binding_or_first(&mut |_, _, _, _| has_bindings = true);
    if has_bindings {
        return false;
    }

    // If we shouldn't check the types, exit early.
    if !check_types {
        return true;
    }

    // Check whether any possibly "unknown" patterns are included,
    // because users might not know which values some enum has.
    // Well-known enums are excepted, as we assume people know them.
    // We do a deep check, to be able to disallow Err(En::Foo(_))
    // for usage of the En::Foo variant, as we disallow En::Foo(_),
    // but we allow Err(_).
    let typeck_results = cx.typeck_results();
    let mut has_disallowed = false;
    pat.walk_always(|pat| {
        // Only do the check if the type is "spelled out" in the pattern
        if !matches!(
            pat.kind,
            PatKind::Struct(..)
                | PatKind::TupleStruct(..)
                | PatKind::Expr(PatExpr {
                    kind: PatExprKind::Path(..),
                    ..
                },)
        ) {
            return;
        }
        let ty = typeck_results.pat_ty(pat);
        // Option and Result are allowed, everything else isn't.
        if !matches!(ty.opt_diag_name(cx), Some(sym::Option | sym::Result)) {
            has_disallowed = true;
        }
    });
    !has_disallowed
}

/// Checks if the passed block is a simple identity referring to bindings created by the pattern,
/// and if yes, returns a mapping between the relevant sub-pattern and the identifier it corresponds
/// to.
///
/// We support patterns with multiple bindings and tuples, e.g.:
///
/// ```ignore
/// let (foo_o, bar_o) = if let (Some(foo), bar) = g() { (foo, bar) } else { ... }
/// ```
///
/// The expected params would be:
///
/// ```ignore
/// local_pat: (foo_o, bar_o)
/// let_pat: (Some(foo), bar)
/// expr: (foo, bar)
/// ```
///
/// We build internal `sub_pats` so that it looks like `[foo_o, bar_o]` and `paths` so that it looks
/// like `[foo, bar]`. Then we turn that into `FxHashMap [(foo) -> (foo_o), (bar) -> (bar_o)]` which
/// we return.
fn expr_simple_identity_map<'a, 'hir>(
    local_pat: &'a Pat<'hir>,
    let_pat: &'_ Pat<'hir>,
    expr: &'_ Expr<'hir>,
) -> Option<FxHashMap<Symbol, (&'a Pat<'hir>, BindingMode)>> {
    let peeled = peel_blocks(expr);
    let (sub_pats, paths) = match (local_pat.kind, peeled.kind) {
        (PatKind::Tuple(pats, _), ExprKind::Tup(exprs)) | (PatKind::Slice(pats, ..), ExprKind::Array(exprs)) => {
            (pats, exprs)
        },
        (_, ExprKind::Path(_)) => (slice::from_ref(local_pat), slice::from_ref(peeled)),
        _ => return None,
    };

    // There is some length mismatch, which indicates usage of .. in the patterns above e.g.:
    // let (a, ..) = if let [a, b, _c] = ex { (a, b) } else { ... };
    // We bail in these cases as they should be rare.
    if paths.len() != sub_pats.len() {
        return None;
    }

    let mut pat_bindings = FxHashMap::default();
    let_pat.each_binding_or_first(&mut |binding_mode, _hir_id, _sp, ident| {
        pat_bindings.insert(ident, binding_mode);
    });
    if pat_bindings.len() < paths.len() {
        // This rebinds some bindings from the outer scope, or it repeats some copy-able bindings multiple
        // times. We don't support these cases so we bail here. E.g.:
        // let foo = 0;
        // let (new_foo, bar, bar_copied) = if let Some(bar) = Some(0) { (foo, bar, bar) } else { .. };
        return None;
    }
    let mut ident_map = FxHashMap::default();
    for (sub_pat, path) in sub_pats.iter().zip(paths.iter()) {
        if let ExprKind::Path(QPath::Resolved(_ty, path)) = path.kind
            && let [path_seg] = path.segments
            && let ident = path_seg.ident
            && let Some(let_binding_mode) = pat_bindings.remove(&ident)
        {
            ident_map.insert(ident.name, (sub_pat, let_binding_mode));
        } else {
            return None;
        }
    }
    Some(ident_map)
}
