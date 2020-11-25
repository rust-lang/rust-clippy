use crate::utils::span_lint_and_sugg;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{Arm, Expr, ExprKind, FieldPat, MatchSource, Pat, PatKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// **What it does:**
    /// As suggested in the [non_exhaustive RFC](https://github.com/rust-lang/rfcs/blob/master/text/2008-non-exhaustive.md#unresolved-questions), 
    /// when using non-exhaustive enums and structs in patterns,
    /// this lint warns the user for missing variants or fields despite having a wildcard arm or a rest pattern.
    ///
    /// **Why is this bad?**
    /// When new fields/variants are added by the upstream crate they might go unnoticed.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    /// ```rust,ignore
    /// #[non_exhaustive]
    /// # enum E {First,Second,Third}
    /// # let e = E::First;
    /// // Bad
    /// match e {
    ///         E::First => {}
    ///         E::Second => {}
    ///         _ => {}
    ///     }
    /// // Good
    /// match e {
    ///         E::First => {}
    ///         E::Second => {}
    ///         E::Third => {}
    ///     }
    /// ```
    pub NOT_EXHAUSTIVE_ENOUGH,
    pedantic,
    "missing variants or fields in a pattern despite having a wildcard arm or a rest pattern"
}

declare_lint_pass!(NotExhaustiveEnough => [NOT_EXHAUSTIVE_ENOUGH]);

impl<'tcx> LateLintPass<'tcx> for NotExhaustiveEnough {
    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx Pat<'_>) {
        if_chain! {
            if let PatKind::Struct(_, ref field_pats, ref rest_pat) = &pat.kind;
            if let ty::Adt(adt_def, _) = cx.typeck_results().pat_ty(pat).kind();
            if adt_def.is_struct();
            if is_struct_not_exhaustive(adt_def);
            if *rest_pat;
            if !field_pats.is_empty();
            if let Some(variant) = get_variant(adt_def);
            if let field_defs = &variant.fields;
            then
            {
                check_struct_pat(cx, pat, field_pats, field_defs);
            }
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if_chain! {
            if let ExprKind::Match(e, ref arms, MatchSource::Normal) = expr.kind;
            if !arms.is_empty();
            if let ExprKind::Path(..) = e.kind;
            if let ty::Adt(adt_def, _) = cx.typeck_results().expr_ty(e).kind();
            if adt_def.is_enum();
            if is_enum_not_exhaustive(adt_def);
            then
            {
                check_path_pat(cx, arms, e);
            }
        }
    }
}

fn check_path_pat<'tcx>(cx: &LateContext<'tcx>, arms: &[Arm<'_>], e: &'tcx Expr<'_>) {
    let missing_variants = get_missing_variants(cx, arms, e);
    span_lint_and_sugg(
        cx,
        NOT_EXHAUSTIVE_ENOUGH,
        e.span,
        "Enum not exhaustive enough",
        "try adding missing field/s",
        missing_variants.join(" , "),
        Applicability::MaybeIncorrect,
    );
}

fn check_struct_pat<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'_>,
    field_pats: &[FieldPat<'_>],
    field_defs: &[ty::FieldDef],
) {
    let missing_fields = get_missing_fields(field_pats, field_defs);
    span_lint_and_sugg(
        cx,
        NOT_EXHAUSTIVE_ENOUGH,
        pat.span,
        "Struct not exhaustive enough",
        "try adding missing field/s",
        missing_fields.join(" , "),
        Applicability::MaybeIncorrect,
    );
}

fn get_missing_variants<'tcx>(cx: &LateContext<'tcx>, arms: &[Arm<'_>], e: &'tcx Expr<'_>) -> Vec<String> {
    let ty = cx.typeck_results().expr_ty(e);
    let mut missing_variants = vec![];
    if let ty::Adt(def, _) = ty.kind() {
        for variant in &def.variants {
            missing_variants.push(variant);
        }
    }
    for arm in arms {
        if let PatKind::Path(ref path) = arm.pat.kind {
            if let QPath::Resolved(_, p) = path {
                missing_variants.retain(|e| e.ctor_def_id != Some(p.res.def_id()));
            }
        } else if let PatKind::TupleStruct(ref path, ref patterns, ..) = arm.pat.kind {
            if let QPath::Resolved(_, p) = path {
                let is_pattern_exhaustive =
                    |pat: &&Pat<'_>| matches!(pat.kind, PatKind::Wild | PatKind::Binding(.., None));
                if patterns.iter().all(is_pattern_exhaustive) {
                    missing_variants.retain(|e| e.ctor_def_id != Some(p.res.def_id()));
                }
            }
        }
    }
    let missing_variants = missing_variants.iter().map(|v| cx.tcx.def_path_str(v.def_id)).collect();
    missing_variants
}

// refactor - better way?
fn get_missing_fields(field_pats: &[FieldPat<'_>], field_defs: &[ty::FieldDef]) -> Vec<String> {
    let mut missing_fields = vec![];
    let mut field_match = false;

    for field_def in field_defs {
        for field_pat in field_pats {
            if field_def.ident == field_pat.ident {
                field_match = true;
                break;
            }
        }
        if !&field_match {
            missing_fields.push(field_def.ident.name.to_ident_string())
        }
        field_match = false;
    }
    missing_fields
}

fn is_enum_not_exhaustive(adt_def: &ty::AdtDef) -> bool {
    adt_def.is_variant_list_non_exhaustive()
}

fn is_struct_not_exhaustive(adt_def: &ty::AdtDef) -> bool {
    if let Some(variant) = adt_def.variants.iter().next() {
        return variant.is_field_list_non_exhaustive();
    }
    false
}

fn get_variant(adt_def: &ty::AdtDef) -> Option<&ty::VariantDef> {
    if let Some(variant) = adt_def.variants.iter().next() {
        return Some(variant);
    }
    None
}
