use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::in_automatically_derived;
use clippy_utils::source::{IntoSpan as _, SpanExt as _};
use core::mem;
use rustc_errors::{Applicability, SuggestionStyle};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{
    self as hir, Expr, ExprKind, Generics, HirId, Item, Pat, PatKind, QPath, StructTailExpr, Variant, VariantData,
};
use rustc_lexer::is_whitespace;
use rustc_lint::{LateContext, LateLintPass, Lint, impl_lint_pass};
use rustc_middle::span_bug;
use rustc_span::{DUMMY_SP, Span};

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

impl_lint_pass!(EmptyWithBrackets => [
    EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
    EMPTY_STRUCTS_WITH_BRACKETS,
]);

struct Def {
    did: LocalDefId,
    lint_sp: Span,
    edit_sp: Span,
    needs_semi: bool,
    item_kind: ItemKind,
    var_kind: VarKind,
}

struct Use {
    did: LocalDefId,
    /// The span of the call parenthesis, or `DUMMY_SP` if this use can't be changed.
    edit_sp: Span,
}

#[derive(Clone, Copy)]
enum VarKind {
    Tuple,
    Struct,
}
impl VarKind {
    fn chars(self) -> (char, char) {
        match self {
            Self::Tuple => ('(', ')'),
            Self::Struct => ('{', '}'),
        }
    }

    fn is_parens(self, s: &str) -> bool {
        let (start, end) = self.chars();
        if let Some(s) = s.strip_prefix(start)
            && let Some(s) = s.strip_suffix(end)
        {
            s.chars().all(is_whitespace)
        } else {
            false
        }
    }

    fn strip_leading_parens(self, s: &str) -> Option<&str> {
        let (start, end) = match self {
            Self::Tuple => ('(', ')'),
            Self::Struct => ('{', '}'),
        };
        let s = s
            .trim_start_matches(is_whitespace)
            .strip_prefix(start)?
            .trim_start_matches(is_whitespace);
        s.strip_prefix("..")
            .map_or(s, |s| s.trim_start_matches(is_whitespace))
            .strip_prefix(end)
    }

    fn sugg_msg(self) -> &'static str {
        match self {
            Self::Tuple => "remove the parenthesis",
            Self::Struct => "remove the brackets",
        }
    }
}

#[derive(Clone, Copy)]
enum ItemKind {
    Struct,
    Enum,
}
impl ItemKind {
    fn lint(self) -> &'static Lint {
        match self {
            Self::Struct => EMPTY_STRUCTS_WITH_BRACKETS,
            Self::Enum => EMPTY_ENUM_VARIANTS_WITH_BRACKETS,
        }
    }

    fn msg(self) -> &'static str {
        match self {
            Self::Struct => "non-unit struct contains no fields",
            Self::Enum => "non-unit variant contains no fields",
        }
    }
}

#[derive(Default)]
pub struct EmptyWithBrackets {
    defs: Vec<Def>,
    uses: Vec<Use>,
    // Used to skip over constructor path expressions when they've already been seen as a
    // call expression.
    skip_next_expr: bool,
}

impl<'tcx> LateLintPass<'tcx> for EmptyWithBrackets {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let hir::ItemKind::Struct(ident, generics, ref var_data) = item.kind {
            self.check_def(
                cx,
                var_data,
                item.owner_id.def_id,
                item.span,
                ident.span,
                Some(generics),
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
        );
    }

    fn check_expr(&mut self, cx: &LateContext<'_>, e: &Expr<'_>) {
        match e.kind {
            ExprKind::Call(callee, []) if let ExprKind::Path(qpath) = &callee.kind => {
                // The next visited expression will be `callee`. Make sure we don't add it
                // as a tuple use.
                self.skip_next_expr = true;
                self.push_use(cx, VarKind::Tuple, e.span, qpath, callee.hir_id);
            },
            ExprKind::Path(ref qpath)
                if !mem::replace(&mut self.skip_next_expr, false)
                    && let Res::Def(DefKind::Ctor(..), did) = cx.typeck_results().qpath_res(qpath, e.hir_id)
                    && let Some(did) = did.as_local()
                    && cx.tcx.fn_sig(did).skip_binder().skip_binder().inputs_and_output.len() == 1
                    && let Some(did) = cx.tcx.opt_local_parent(did) =>
            {
                self.uses.push(Use { did, edit_sp: DUMMY_SP });
            },
            ExprKind::Struct(qpath, [], StructTailExpr::None | StructTailExpr::DefaultFields(_)) => {
                self.push_use(cx, VarKind::Struct, e.span, qpath, e.hir_id);
            },
            _ => {},
        }
    }

    fn check_pat(&mut self, cx: &LateContext<'_>, p: &Pat<'tcx>) {
        match p.kind {
            PatKind::TupleStruct(ref qpath, [], _) => {
                self.push_use(cx, VarKind::Tuple, p.span, qpath, p.hir_id);
            },
            PatKind::Struct(ref qpath, [], _) => {
                self.push_use(cx, VarKind::Struct, p.span, qpath, p.hir_id);
            },
            _ => {},
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        if self.defs.is_empty() {
            return;
        }
        self.uses.sort_unstable_by_key(|x| x.did.local_def_index);

        let mut replacements = Vec::with_capacity(16);
        'def_loop: for def in &self.defs {
            replacements.clear();
            replacements.push((
                def.edit_sp,
                if def.needs_semi {
                    String::from(";")
                } else {
                    String::new()
                },
            ));
            let mut app = if def.lint_sp.from_expansion() {
                Applicability::MaybeIncorrect
            } else {
                Applicability::MachineApplicable
            };

            let start = self
                .uses
                .partition_point(|x| x.did.local_def_index < def.did.local_def_index);
            for u in &self.uses[start..] {
                if u.did != def.did {
                    break;
                }
                if u.edit_sp == DUMMY_SP {
                    continue 'def_loop;
                }
                if u.edit_sp.from_expansion() {
                    app = Applicability::MaybeIncorrect;
                }
                replacements.push((u.edit_sp, String::new()));
            }

            span_lint_and_then(cx, def.item_kind.lint(), def.lint_sp, def.item_kind.msg(), |diag| {
                diag.multipart_suggestion_with_style(
                    def.var_kind.sugg_msg(),
                    replacements.clone(),
                    app,
                    SuggestionStyle::HideCodeAlways,
                );
            });
        }
    }
}

impl EmptyWithBrackets {
    fn check_def(
        &mut self,
        cx: &LateContext<'_>,
        data: &VariantData<'_>,
        did: LocalDefId,
        item_sp: Span,
        name_sp: Span,
        struct_generics: Option<&Generics<'_>>,
    ) {
        // Start by normalizing the various variant forms into:
        // * Does the replacement need a semicolon.
        // * What kind of variant it is.
        // * The span that should only contain braces and whitespace.
        let (needs_semi, var_kind, start_sp, end_pos, ctxt) = match *data {
            VariantData::Struct { fields: [], .. } => {
                let data = item_sp.data();
                if let Some(g) = struct_generics {
                    // `struct Name<generics> where {}`
                    (true, VarKind::Struct, g.where_clause_span, data.hi, data.ctxt)
                } else {
                    // `Variant {}`
                    (false, VarKind::Struct, name_sp, data.hi, data.ctxt)
                }
            },
            VariantData::Tuple([], _, _) => {
                if let Some(g) = struct_generics {
                    // `struct Name<generics>() where;`
                    let data = g.where_clause_span.data();
                    (false, VarKind::Tuple, g.span, data.lo, data.ctxt)
                } else {
                    // `Variant()`
                    let data = item_sp.data();
                    (false, VarKind::Tuple, name_sp, data.hi, data.ctxt)
                }
            },
            VariantData::Struct { .. } | VariantData::Tuple(..) | VariantData::Unit(..) => return,
        };

        let start_data = start_sp.data();
        if start_data.ctxt == ctxt
            && name_sp.ctxt() == ctxt
            && !ctxt.in_external_macro(cx.tcx.sess.source_map())
            && let Some(lint_range) = (start_data.hi..end_pos).clone().map_range(cx, |_, src, range| {
                // Check that the source text only contains whitespace and the braces.
                // Anything else (e.g. comments, cfgs, macro vars, etc.) should stop this
                // lint from triggering.
                let src = src.get(range.clone())?;
                let src_trimmed = src.trim_matches(is_whitespace);
                var_kind.is_parens(src_trimmed).then(|| {
                    let offset = src_trimmed.as_ptr().addr() - src.as_ptr().addr();
                    range.start + offset..range.start + offset + src_trimmed.len()
                })
            })
        {
            // Don't edit the out any trailing whitespace to avoid problems with
            // where clauses.
            let edit_sp = Span::new(start_data.hi, lint_range.end, ctxt, None);
            let lint_sp = Span::new(lint_range.start, lint_range.end, ctxt, None);
            if matches!(var_kind, VarKind::Struct) || !cx.effective_visibilities.is_exported(did) {
                self.defs.push(Def {
                    did,
                    lint_sp,
                    edit_sp,
                    needs_semi,
                    item_kind: if struct_generics.is_some() {
                        ItemKind::Struct
                    } else {
                        ItemKind::Enum
                    },
                    var_kind,
                });
            }
        }
    }

    fn push_use(&mut self, cx: &LateContext<'_>, kind: VarKind, sp: Span, qpath: &QPath<'_>, hir_id: HirId) {
        if let Res::Def(def_kind, did) = cx.typeck_results().qpath_res(qpath, hir_id)
            && let Some(did) = did.as_local()
        {
            let did = match def_kind {
                DefKind::Ctor(..) => cx.tcx.local_parent(did),
                // 2026-08-20: `<T>::X {}` incorrectly resolves to the variant.
                DefKind::Variant | DefKind::Struct => did,
                _ => return,
            };
            let sp_data = sp.data();
            let (in_ctxt, path_sp) = match *qpath {
                QPath::Resolved(_, path) if let [.., seg] = path.segments => {
                    (seg.ident.span.ctxt() == sp_data.ctxt, path.span)
                },
                QPath::TypeRelative(ty, seg) => (ty.span.ctxt() == sp_data.ctxt, seg.ident.span),
                QPath::Resolved(_, path) => span_bug!(path.span, "path with no segments"),
            };
            let path_data = path_sp.data();
            let edit_sp = if in_ctxt
                && sp_data.ctxt == path_data.ctxt
                && let Some(edit_range) = (path_data.hi..sp_data.hi).map_range(cx, |_, src, range| {
                    let s = kind.strip_leading_parens(src.get(range.clone())?)?;
                    s.chars()
                        .all(|c| c == ')' || is_whitespace(c))
                        .then_some(range.start..range.end - s.len())
                }) {
                edit_range.with_ctxt(sp_data.ctxt)
            } else if let VarKind::Tuple = kind
                && !in_automatically_derived(cx.tcx, hir_id)
            {
                // Parenthesis have to be removed for the suggestion to be valid.
                // Use the dummy span to mark this as unfixable.
                DUMMY_SP
            } else {
                return;
            };
            self.uses.push(Use { did, edit_sp });
        }
    }
}
