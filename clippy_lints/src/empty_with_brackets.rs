use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{IntoSpan, SpanRangeExt, walk_span_to_context};
use core::mem;
use rustc_errors::{Applicability, SuggestionStyle};
use rustc_hir::def::{CtorKind, DefKind, Res};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{Expr, ExprKind, Generics, Item, ItemKind, QPath, Variant, VariantData};
use rustc_lint::{LateContext, LateLintPass, Lint};
use rustc_session::impl_lint_pass;
use rustc_span::{DUMMY_SP, Span};

declare_clippy_lint! {
    /// ### What it does
    /// Finds structs without fields (a so-called "empty struct") that are declared with brackets.
    ///
    /// ### Why restrict this?
    /// Empty brackets after a struct declaration can be omitted,
    /// and it may be desirable to do so consistently for style.
    ///
    /// However, removing the brackets also introduces a public constant named after the struct,
    /// so this is not just a syntactic simplification but an API change, and adding them back
    /// is a *breaking* API change.
    ///
    /// ### Example
    /// ```no_run
    /// struct Cookie {}
    /// struct Biscuit();
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct Cookie;
    /// struct Biscuit;
    /// ```
    #[clippy::version = "1.62.0"]
    pub EMPTY_STRUCTS_WITH_BRACKETS,
    restriction,
    "finds struct declarations with empty brackets"
}

declare_clippy_lint! {
    /// ### What it does
    /// Finds enum variants without fields that are declared with empty brackets.
    ///
    /// ### Why restrict this?
    /// Empty brackets after a enum variant declaration are redundant and can be omitted,
    /// and it may be desirable to do so consistently for style.
    ///
    /// However, removing the brackets also introduces a public constant named after the variant,
    /// so this is not just a syntactic simplification but an API change, and adding them back
    /// is a *breaking* API change.
    ///
    /// ### Example
    /// ```no_run
    /// enum MyEnum {
    ///     HasData(u8),
    ///     HasNoData(),       // redundant parentheses
    ///     NoneHereEither {}, // redundant braces
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// enum MyEnum {
    ///     HasData(u8),
    ///     HasNoData,
    ///     NoneHereEither,
    /// }
    /// ```
    #[clippy::version = "1.77.0"]
    pub EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
    restriction,
    "finds enum variants with empty brackets"
}

struct TupleDef {
    did: LocalDefId,
    lint_sp: Span,
    edit_sp: Span,
    needs_semi: bool,
    lint: &'static Lint,
    msg: &'static str,
}

struct TupleUse {
    did: LocalDefId,
    /// The span of the call parenthesis, or `DUMMY_SP` if this use can't be changed.
    edit_sp: Span,
}

#[derive(Default)]
pub struct EmptyWithBrackets {
    tuple_defs: Vec<TupleDef>,
    tuple_uses: Vec<TupleUse>,
    // Used to skip over constructor path expressions when they've already been seen as a
    // call expression.
    skip_next_expr: bool,
}

impl_lint_pass!(EmptyWithBrackets => [EMPTY_STRUCTS_WITH_BRACKETS, EMPTY_ENUM_VARIANTS_WITH_BRACKETS]);

impl<'tcx> LateLintPass<'tcx> for EmptyWithBrackets {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Struct(ident, generics, ref var_data) = item.kind {
            self.check_def(
                cx,
                var_data,
                item.owner_id.def_id,
                item.span,
                ident.span,
                Some(generics),
                EMPTY_STRUCTS_WITH_BRACKETS,
                "non-unit struct contains no fields",
            );
        }
    }

    fn check_variant(&mut self, cx: &LateContext<'_>, variant: &Variant<'_>) {
        self.check_def(
            cx,
            &variant.data,
            variant.def_id,
            variant.span,
            variant.ident.span,
            None,
            EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
            "non-unit variant contains no fields",
        );
    }

    fn check_expr(&mut self, cx: &LateContext<'_>, e: &Expr<'_>) {
        match e.kind {
            ExprKind::Call(callee, [])
                if let ExprKind::Path(QPath::Resolved(_, path)) = callee.kind
                    && let Res::Def(DefKind::Ctor(_, CtorKind::Fn), did) = path.res
                    && let Some(did) = did.as_local()
                    && let e_data = e.span.data()
                    && e_data.ctxt.is_root() =>
            {
                // The next visited expression with be `callee`. Make sure we don't add it
                // as a tuple use.
                self.skip_next_expr = true;
                let edit_sp = if let Some(callee_sp) = walk_span_to_context(callee.span, e_data.ctxt)
                    && let edit_range = (callee_sp.hi()..e_data.hi)
                    && let Some(edit_range) = edit_range.map_range(cx, |_, src, range| {
                        let src = src
                            .get(range.clone())?
                            .trim_start()
                            .strip_prefix('(')?
                            .trim_start()
                            .strip_prefix(')')?;
                        // Any trailing closing parens are from the call expression being wrapped in parens.
                        let len = src.len();
                        src.trim_start_matches(|c: char| c == ')' || c.is_whitespace())
                            .is_empty()
                            .then_some(range.start..range.end - len)
                    }) {
                    edit_range.into_span()
                } else {
                    DUMMY_SP
                };
                self.tuple_uses.push(TupleUse {
                    did: cx.tcx.local_parent(did),
                    edit_sp,
                });
            },

            ExprKind::Path(QPath::Resolved(_, path))
                if let Res::Def(DefKind::Ctor(_, CtorKind::Fn), did) = path.res
                    && let Some(did) = did.as_local()
                    && !mem::replace(&mut self.skip_next_expr, false)
                    && cx.tcx.fn_sig(did).skip_binder().skip_binder().inputs_and_output.len() == 1 =>
            {
                self.tuple_uses.push(TupleUse {
                    did: cx.tcx.local_parent(did),
                    edit_sp: DUMMY_SP,
                });
            },

            _ => {},
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        if self.tuple_defs.is_empty() {
            return;
        }
        self.tuple_uses.sort_unstable_by_key(|x| x.did.local_def_index);

        let mut replacements = Vec::with_capacity(16);
        'def_loop: for def in &self.tuple_defs {
            replacements.clear();
            replacements.push((
                def.edit_sp,
                if def.needs_semi {
                    String::from(";")
                } else {
                    String::new()
                },
            ));

            let start = self
                .tuple_uses
                .partition_point(|x| x.did.local_def_index < def.did.local_def_index);
            for u in &self.tuple_uses[start..] {
                if u.did != def.did {
                    break;
                }
                if u.edit_sp == DUMMY_SP {
                    continue 'def_loop;
                }
                replacements.push((u.edit_sp, String::new()));
            }

            span_lint_and_then(cx, def.lint, def.lint_sp, def.msg, |diag| {
                diag.multipart_suggestion_with_style(
                    "remove the parenthesis",
                    replacements.clone(),
                    Applicability::MachineApplicable,
                    SuggestionStyle::HideCodeAlways,
                );
            });
        }
    }
}

impl EmptyWithBrackets {
    #[expect(clippy::too_many_arguments)]
    fn check_def(
        &mut self,
        cx: &LateContext<'_>,
        data: &VariantData<'_>,
        did: LocalDefId,
        item_sp: Span,
        name_sp: Span,
        struct_generics: Option<&Generics<'_>>,
        lint: &'static Lint,
        msg: &'static str,
    ) {
        // Start by normalizing the various variant forms into:
        // * Does the replacement need a semicolon
        // * Which brace characters to use
        // * The span that should only contain braces and whitespace.
        let (needs_semi, start_char, end_char, start_sp, end_pos, ctxt) = match *data {
            VariantData::Struct { fields: [], .. } => {
                if let Some(g) = struct_generics {
                    // `struct Name<generics> where {}`
                    let data = item_sp.data();
                    (true, '{', '}', g.where_clause_span, data.hi, data.ctxt)
                } else {
                    // `Variant {}`
                    let data = item_sp.data();
                    (false, '{', '}', name_sp, data.hi, data.ctxt)
                }
            },
            VariantData::Tuple([], _, _) => {
                if let Some(g) = struct_generics {
                    // `struct Name<generics>() where;`
                    let data = g.where_clause_span.data();
                    (false, '(', ')', g.span, data.lo, data.ctxt)
                } else {
                    // `Variant()`
                    let data = item_sp.data();
                    (false, '(', ')', name_sp, data.hi, data.ctxt)
                }
            },
            VariantData::Struct { .. } | VariantData::Tuple(..) | VariantData::Unit(..) => return,
        };

        let start_data = start_sp.data();
        if start_data.ctxt == ctxt
            && !ctxt.in_external_macro(cx.tcx.sess.source_map())
            && let Some(lint_range) = (start_data.hi..end_pos).clone().map_range(cx, |_, src, range| {
                // Check that the source text only contains whitespace and the braces.
                // Anything else (e.g. comments, cfgs, macro vars, etc.) should stop this
                // lint from triggering.
                let src = src.get(range.clone())?;
                let src2 = src.trim_start();
                let start = range.start + (src.len() - src2.len());
                let src3 = src2.trim_end();
                let end = range.end - (src2.len() - src3.len());
                src3.strip_prefix(start_char)?
                    .strip_suffix(end_char)?
                    .trim_start()
                    .is_empty()
                    .then_some(start..end)
            })
        {
            // Don't edit the out any trailing whitespace to avoid problems with
            // where clauses.
            let edit_sp = Span::new(start_data.hi, lint_range.end, ctxt, None);
            let lint_sp = Span::new(lint_range.start, lint_range.end, ctxt, None);
            if start_char == '{' {
                span_lint_and_then(cx, lint, lint_sp, msg, |diagnostic| {
                    diagnostic.span_suggestion_hidden(
                        edit_sp,
                        "remove the braces",
                        if needs_semi { String::from(";") } else { String::new() },
                        Applicability::MaybeIncorrect,
                    );
                });
            } else if !cx.effective_visibilities.is_exported(did) {
                self.tuple_defs.push(TupleDef {
                    did,
                    lint_sp,
                    edit_sp,
                    needs_semi,
                    lint,
                    msg,
                });
            }
        }
    }
}
