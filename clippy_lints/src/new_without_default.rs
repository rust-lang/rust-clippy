use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::return_ty;
use clippy_utils::source::{indent_of, reindent_multiline, snippet_opt, snippet_with_applicability};
use clippy_utils::sugg::DiagExt;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::attrs::AttributeKind;
use rustc_hir::{Attribute, HirIdSet};
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::AssocKind;
use rustc_session::impl_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for public types with a `pub fn new() -> Self` method and no
    /// implementation of
    /// [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html).
    ///
    /// ### Why is this bad?
    /// The user might expect to be able to use
    /// [`Default`](https://doc.rust-lang.org/std/default/trait.Default.html) as the
    /// type can be constructed without arguments.
    ///
    /// ### Example
    /// ```ignore
    /// pub struct Foo(Bar);
    ///
    /// impl Foo {
    ///     pub fn new() -> Self {
    ///         Foo(Bar::new())
    ///     }
    /// }
    /// ```
    ///
    /// To fix the lint, add a `Default` implementation that delegates to `new`:
    ///
    /// ```ignore
    /// pub struct Foo(Bar);
    ///
    /// impl Default for Foo {
    ///     fn default() -> Self {
    ///         Foo::new()
    ///     }
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub NEW_WITHOUT_DEFAULT,
    style,
    "`pub fn new() -> Self` method without `Default` implementation"
}

impl_lint_pass!(NewWithoutDefault => [NEW_WITHOUT_DEFAULT]);

#[derive(Clone, Default)]
pub struct NewWithoutDefault {
    impling_types: Option<HirIdSet>,
}

impl<'tcx> LateLintPass<'tcx> for NewWithoutDefault {
    #[expect(clippy::too_many_lines)]
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'_>) {
        let hir::ItemKind::Impl(hir::Impl {
            of_trait: None,
            generics,
            self_ty: impl_self_ty,
            ..
        }) = item.kind
        else {
            return;
        };

        for assoc_item in cx
            .tcx
            .associated_items(item.owner_id.def_id)
            .filter_by_name_unhygienic(sym::new)
        {
            if let AssocKind::Fn { has_self: false, .. } = assoc_item.kind
                && let assoc_item_hir_id = cx.tcx.local_def_id_to_hir_id(assoc_item.def_id.expect_local())
                && let impl_item = cx.tcx.hir_node(assoc_item_hir_id).expect_impl_item()
                && !impl_item.span.in_external_macro(cx.sess().source_map())
                && let hir::ImplItemKind::Fn(ref sig, _) = impl_item.kind
                && let id = impl_item.owner_id
                // can't be implemented for unsafe new
                && !sig.header.is_unsafe()
                // shouldn't be implemented when it is hidden in docs
                && !cx.tcx.is_doc_hidden(impl_item.owner_id.def_id)
                // when the result of `new()` depends on a parameter we should not require
                // an impl of `Default`
                && impl_item.generics.params.is_empty()
                && sig.decl.inputs.is_empty()
                && cx.effective_visibilities.is_exported(impl_item.owner_id.def_id)
                && let self_ty = cx.tcx.type_of(item.owner_id).instantiate_identity().skip_norm_wip()
                && self_ty == return_ty(cx, impl_item.owner_id)
                && let Some(default_trait_id) = cx.tcx.get_diagnostic_item(sym::Default)
            {
                if self.impling_types.is_none() {
                    let mut impls = HirIdSet::default();
                    for &d in cx.tcx.local_trait_impls(default_trait_id) {
                        let ty = cx.tcx.type_of(d).instantiate_identity().skip_norm_wip();
                        if let Some(ty_def) = ty.ty_adt_def()
                            && let Some(local_def_id) = ty_def.did().as_local()
                        {
                            impls.insert(cx.tcx.local_def_id_to_hir_id(local_def_id));
                        }
                    }
                    self.impling_types = Some(impls);
                }

                // Check if a Default implementation exists for the Self type, regardless of
                // generics
                if let Some(ref impling_types) = self.impling_types
                    && let self_def = cx.tcx.type_of(item.owner_id).instantiate_identity().skip_norm_wip()
                    && let Some(self_def) = self_def.ty_adt_def()
                    && let Some(self_local_did) = self_def.did().as_local()
                    && let self_id = cx.tcx.local_def_id_to_hir_id(self_local_did)
                    && impling_types.contains(&self_id)
                {
                    return;
                }

                let mut app = if impl_has_hidden_attr(cx, item) {
                    Applicability::MaybeIncorrect
                } else {
                    Applicability::MachineApplicable
                };
                let attrs_sugg = {
                    let mut sugg = String::new();
                    for attr in cx.tcx.hir_attrs(assoc_item_hir_id) {
                        let Attribute::Parsed(AttributeKind::CfgTrace(attrs)) = attr else {
                            // This might be some other attribute that the `impl Default` ought to inherit.
                            // But it could also be one of the many attributes that:
                            // - can't be put on an impl block -- like `#[inline]`
                            // - we can't even build a suggestion for, since `Attribute::span` may panic.
                            //
                            // Because of all that, remain on the safer side -- don't inherit this attr, and just
                            // reduce the applicability
                            app = Applicability::MaybeIncorrect;
                            continue;
                        };

                        for (_, attr_span) in attrs {
                            sugg.push_str(&snippet_with_applicability(cx.sess(), *attr_span, "_", &mut app));
                            sugg.push('\n');
                        }
                    }
                    sugg
                };
                let generics_sugg = snippet_with_applicability(cx, generics.span, "", &mut app);
                let where_clause_sugg = if generics.has_where_clause_predicates {
                    let where_clause_sugg =
                        snippet_with_applicability(cx, generics.where_clause_span, "", &mut app).to_string();
                    let mut where_clause_sugg = reindent_multiline(&where_clause_sugg, true, Some(4));
                    if impl_item.generics.has_where_clause_predicates {
                        if !where_clause_sugg.ends_with(',') {
                            where_clause_sugg.push(',');
                        }

                        let additional_where_preds =
                            snippet_with_applicability(cx, impl_item.generics.where_clause_span, "", &mut app);
                        let ident = indent_of(cx, generics.where_clause_span).unwrap_or(0);
                        // Remove the leading `where ` keyword
                        let additional_where_preds = additional_where_preds.trim_start_matches("where").trim_start();
                        where_clause_sugg.push('\n');
                        where_clause_sugg.extend(std::iter::repeat_n(' ', ident));
                        where_clause_sugg.push_str(additional_where_preds);
                    }
                    format!("\n{where_clause_sugg}\n")
                } else if impl_item.generics.has_where_clause_predicates {
                    let where_clause_sugg =
                        snippet_with_applicability(cx, impl_item.generics.where_clause_span, "", &mut app);
                    let where_clause_sugg = reindent_multiline(&where_clause_sugg, true, Some(4));
                    format!("\n{}\n", where_clause_sugg.trim_start())
                } else {
                    String::new()
                };
                let self_ty_fmt = self_ty.to_string();
                let self_type_snip = snippet_with_applicability(cx, impl_self_ty.span, &self_ty_fmt, &mut app);
                span_lint_hir_and_then(
                    cx,
                    NEW_WITHOUT_DEFAULT,
                    id.into(),
                    impl_item.span,
                    format!("you should consider adding a `Default` implementation for `{self_type_snip}`"),
                    |diag| {
                        diag.suggest_prepend_item(
                            cx,
                            item.span,
                            "try adding this",
                            &create_new_without_default_suggest_msg(
                                &attrs_sugg,
                                &self_type_snip,
                                &generics_sugg,
                                &where_clause_sugg,
                            ),
                            app,
                        );
                    },
                );
            }
        }
    }
}

/// Checks whether `suggest_prepend_item` would misplace an attribute that belongs to `item`
/// itself, rather than to the newly suggested `impl Default`.
///
/// Two cases are covered:
/// - An attribute that is still visible in the HIR (e.g. `#[cfg(..)]`, `#[allow(..)]`, a doc
///   comment, ...): inserting the suggestion right before `item.span` would leave that attribute
///   attached to the *new* impl instead of the original one.
/// - An outer attribute *macro* (e.g. `PyO3`'s `#[pymethods]`) that gets fully consumed during
///   macro expansion and therefore leaves nothing behind in `hir_attrs`, even though the source
///   still has it written immediately above the impl block. Blindly applying the suggestion then
///   ends up wrapping the wrong item in that macro, which can fail to compile (see
///   rust-lang/rust-clippy#17361).
fn impl_has_hidden_attr(cx: &LateContext<'_>, item: &hir::Item<'_>) -> bool {
    if !cx.tcx.hir_attrs(item.hir_id()).is_empty() {
        return true;
    }

    // `hir_attrs` is empty, but that's exactly what a fully-consumed outer attribute macro looks
    // like: the macro rewrites/consumes the whole item during expansion, so nothing is left in
    // the HIR, even though the source still has it written immediately above the impl block.
    if item.span.from_expansion() {
        return false;
    }

    let tcx = cx.tcx;

    // Rather than classifying the raw text above the impl in isolation -- which can't reliably
    // tell a consumed attribute macro apart from ordinary code that just happens to sit right
    // above the impl, the exact bug that kept reappearing here (see rust-lang/rust-clippy#17361)
    // -- ask the HIR directly what the previous sibling item in this same scope is, and only look
    // at the source gap between that sibling and this impl. A *visible* attribute belonging to
    // either neighbour is already included in that neighbour's own span (checked above for this
    // item; the same holds for whatever came before it), so it can never leak into that gap: the
    // only things a real gap can ever contain are whitespace, comments, and leftover text from a
    // fully-consumed attribute macro.
    // Not a plain module (e.g. an impl nested inside a function body): there's no cheap way to
    // enumerate its lexical siblings here, so stay on the safe side rather than assume there's
    // nothing hidden above this impl.
    let (hir::OwnerNode::Item(&hir::Item {
        kind: hir::ItemKind::Mod(_, siblings),
        ..
    })
    | hir::OwnerNode::Crate(siblings)) = tcx.hir_owner_node(tcx.hir_get_parent_item(item.hir_id()))
    else {
        return true;
    };

    let prev_end = siblings
        .item_ids
        .iter()
        .filter(|id| id.owner_id != item.owner_id)
        .map(|id| tcx.hir_item(*id).span.hi())
        .filter(|hi| *hi <= item.span.lo())
        .max()
        // No previous sibling: the impl is the first item in this module (or the crate root), so
        // the gap starts right at the module's own body.
        .unwrap_or_else(|| siblings.spans.inner_span.lo());

    if prev_end > item.span.lo() {
        // Should never happen for well-formed HIR, but don't assume anything is safe if it does.
        return true;
    }

    let gap = item.span.shrink_to_lo().with_lo(prev_end);
    let Some(snippet) = snippet_opt(cx, gap) else {
        // Can't recover the source snippet to inspect: stay on the safe side rather than assume
        // there's nothing hidden above this impl.
        return true;
    };

    gap_contains_attr_residue(&snippet)
}

/// Checks whether `snippet` contains a `#` token outside of any comment or string literal -- the
/// one piece of source text a fully-consumed outer attribute macro invocation always leaves
/// behind, even though the attribute itself disappears from the HIR entirely (see
/// `impl_has_hidden_attr`).
///
/// This only needs to answer "is there an attribute-shaped token here at all", not fully parse
/// one: `snippet` is always the gap between two known HIR items (or a module/crate start and the
/// first item in it), so any `#` found in it cannot belong to either neighbour and must be a
/// leftover attribute -- unlike the raw text directly above an item in isolation, which can be
/// ordinary code that merely happens to look attribute-adjacent (the source of the false
/// positives this function's predecessors kept running into, see rust-lang/rust-clippy#17361).
///
/// Tokenizing with `rustc_lexer` (instead of a hand-rolled scan) is what makes "outside of any
/// comment or string" correct for free: every string form (plain, raw, byte, C-string) and every
/// comment form is each already lexed as one coherent token, so a `#` that's merely part of one
/// (e.g. a raw string's `r#".."#` delimiters, or a `#` typed inside a `// comment`) is never
/// mistaken for a real attribute's `#`.
fn gap_contains_attr_residue(snippet: &str) -> bool {
    tokenize(snippet, FrontmatterAllowed::No).any(|t| t.kind == TokenKind::Pound)
}

fn create_new_without_default_suggest_msg(
    attrs_sugg: &str,
    self_type_snip: &str,
    generics_sugg: &str,
    where_clause_sugg: &str,
) -> String {
    #[rustfmt::skip]
    format!(
"{attrs_sugg}impl{generics_sugg} Default for {self_type_snip}{where_clause_sugg} {{
    fn default() -> Self {{
        Self::new()
    }}
}}")
}

#[cfg(test)]
mod tests_for_gap_contains_attr_residue {
    use super::gap_contains_attr_residue;

    #[test]
    fn bare_attr_is_residue() {
        assert!(gap_contains_attr_residue("#[pymethods]"));
    }

    #[test]
    fn inner_attr_is_residue() {
        assert!(gap_contains_attr_residue("#![pymethods]"));
    }

    #[test]
    fn multiple_attrs_are_residue() {
        assert!(gap_contains_attr_residue("#[foo]\n#[bar]"));
    }

    #[test]
    fn multiline_attr_with_closing_bracket_on_own_line_is_residue() {
        // The attribute's own `)]` lands on a line by itself: it's still one token stream to
        // `rustc_lexer`, and the leading `#` is found regardless of where the rest of the
        // attribute's tokens land.
        assert!(gap_contains_attr_residue("#[gate_attr::pymethods(\n    note\n)]"));
    }

    #[test]
    fn attr_with_surrounding_comments_and_blanks_is_residue() {
        assert!(gap_contains_attr_residue("#[pymethods]\n\n// note about the impl\n"));
    }

    #[test]
    fn attr_with_block_comment_is_residue() {
        assert!(gap_contains_attr_residue("#[pymethods]\n/* multi\n   line\n   comment */"));
    }

    #[test]
    fn empty_gap_is_not_residue() {
        assert!(!gap_contains_attr_residue(""));
    }

    #[test]
    fn whitespace_only_gap_is_not_residue() {
        assert!(!gap_contains_attr_residue("\n\n    \n"));
    }

    #[test]
    fn line_comment_only_gap_is_not_residue() {
        assert!(!gap_contains_attr_residue("// just a comment, nothing special\n"));
    }

    #[test]
    fn block_comment_only_gap_is_not_residue() {
        assert!(!gap_contains_attr_residue("/* just a comment */\n"));
    }

    #[test]
    fn pound_inside_line_comment_is_not_residue() {
        // A `#` typed inside a `//` comment is part of the `LineComment` token, not a real
        // attribute -- the historical bug class this whole mechanism exists to avoid (see
        // rust-lang/rust-clippy#17361).
        assert!(!gap_contains_attr_residue("// see issue #17361\n"));
    }

    #[test]
    fn pound_inside_plain_string_is_not_residue() {
        assert!(!gap_contains_attr_residue(r#"let _ = "contains a # sign";"#));
    }

    #[test]
    fn pound_inside_raw_string_delimiters_is_not_residue() {
        // The `#` characters that delimit a raw string (`r#".."#`) are part of that single
        // `RawStr` token, not a standalone `Pound` token.
        assert!(!gap_contains_attr_residue(r##"let _ = r#"contains a # sign"#;"##));
    }

    #[test]
    fn plain_code_is_not_residue() {
        assert!(!gap_contains_attr_residue("pub struct Foo;"));
    }
}
