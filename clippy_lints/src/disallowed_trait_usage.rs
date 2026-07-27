use clippy_config::Conf;
use clippy_config::types::{DisallowedPathWithoutReplacement, create_disallowed_map};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::{FormatArgsStorage, find_format_arg_expr, is_format_macro, root_macro_call_first_node};
use clippy_utils::paths::{PathNS, find_crates, lookup_path};
use clippy_utils::sym;
use clippy_utils::ty::implements_trait;
use rustc_ast::{FormatArgsPiece, FormatTrait};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, DefIdMap};
use rustc_hir::{Expr, ExprKind, PrimTy};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_session::impl_lint_pass;
use rustc_span::{Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Denies using a configured type via a configured trait interface.
    ///
    /// Note: Even though this lint is warn-by-default, it will only trigger if
    /// entries are defined in the clippy.toml file.
    ///
    /// ### Why is this bad?
    /// Some trait implementations on certain types produce undesirable results.
    /// For example, `Debug` formatting of path types includes escaping and quoting
    /// that is usually not wanted in user-facing output.
    ///
    /// ### Example
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// [[disallowed-trait-usage]]
    /// trait = "std::fmt::Debug"
    /// # Forbid `Debug` formatting of specific types:
    /// types = [
    ///     { path = "std::path::PathBuf", reason = "Use path.display() instead" },
    ///     "std::path::Path",
    /// ]
    /// # Forbid `Debug` formatting of every type implementing these traits:
    /// implements = [
    ///     { path = "std::error::Error", reason = "Use Display for errors" },
    /// ]
    ///
    /// # Forbid a trait outright:
    /// [[disallowed-trait-usage]]
    /// trait = "std::fmt::Pointer"
    /// all-types = "Do not print addresses"
    /// ```
    ///
    /// ```rust,ignore
    /// use std::path::PathBuf;
    /// let path = PathBuf::from("/tmp");
    /// println!("{path:?}"); // Triggers the lint
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// use std::path::PathBuf;
    /// let path = PathBuf::from("/tmp");
    /// println!("{}", path.display()); // OK
    /// ```
    #[clippy::version = "1.99.0"]
    pub DISALLOWED_TRAIT_USAGE,
    style,
    "use of a type via a disallowed trait interface"
}

impl_lint_pass!(DisallowedTraitUsage => [DISALLOWED_TRAIT_USAGE]);

/// A configured path, together with the entry it was written as.
#[derive(Clone, Copy)]
struct DisallowedPathEntry {
    /// The path as written in the configuration.
    path: &'static str,

    /// The configuration entry the path came from, carrying its `reason`.
    disallowed_path: &'static DisallowedPathWithoutReplacement,
}

impl From<(&'static str, &'static DisallowedPathWithoutReplacement)> for DisallowedPathEntry {
    fn from((path, disallowed_path): (&'static str, &'static DisallowedPathWithoutReplacement)) -> Self {
        Self { path, disallowed_path }
    }
}

/// The concrete types of a `types` list.
struct DisallowedTypes {
    def_ids: DefIdMap<DisallowedPathEntry>,
    prim_tys: FxHashMap<PrimTy, DisallowedPathEntry>,
}

impl DisallowedTypes {
    fn get(&self, ty: Ty<'_>) -> Option<DisallowedPathEntry> {
        match ty.kind() {
            ty::Adt(adt_def, _) => self.def_ids.get(&adt_def.did()).copied(),
            _ => ty_as_prim(ty).and_then(|prim| self.prim_tys.get(&prim).copied()),
        }
    }
}

/// Which types a trait is forbidden for.
enum Forbidden {
    /// Every type, from `all-types`.
    All { reason: &'static str },

    /// Only the configured types, from `types` and `implements`.
    Specific {
        /// Concrete types, from `types`.
        types: DisallowedTypes,

        /// Traits whose implementors are disallowed, from `implements`.
        implements: Vec<(DefId, DisallowedPathEntry)>,
    },
}

/// One resolved `disallowed-trait-usage` entry: a trait plus the types it may not be used on.
struct ResolvedEntry {
    trait_def_id: DefId,
    trait_path: &'static str,
    forbidden: Forbidden,
}

pub struct DisallowedTraitUsage {
    format_args: FormatArgsStorage,
    entries: Vec<ResolvedEntry>,
}

/// Returns true if path's root crate is loaded (or the path is a single segment).
fn is_crate_loaded(tcx: TyCtxt<'_>, sym_path: &[Symbol]) -> bool {
    sym_path.len() < 2 || !find_crates(tcx, sym_path[0]).is_empty()
}

fn emit_invalid_path_warning(tcx: TyCtxt<'_>, sym_path: &[Symbol], path: &str, expected: &str, span: Span) {
    if !is_crate_loaded(tcx, sym_path) {
        return;
    }

    // Re-lookup in arbitrary namespace to produce a good "expected X, found Y" message
    let found = lookup_path(tcx, PathNS::Arbitrary, sym_path);
    let message = if let Some(&def_id) = found.first() {
        let (article, description) = tcx.article_and_description(def_id);
        format!("expected a {expected}, found {article} {description}")
    } else {
        format!("`{path}` does not refer to a reachable {expected}")
    };

    tcx.sess.dcx().span_warn(span, message);
}

fn type_def_kind_predicate(def_kind: DefKind) -> bool {
    matches!(
        def_kind,
        DefKind::Struct | DefKind::Union | DefKind::Enum | DefKind::TyAlias | DefKind::ForeignTy
    )
}

fn trait_def_kind_predicate(def_kind: DefKind) -> bool {
    matches!(def_kind, DefKind::Trait)
}

/// Resolves the `types` of a `disallowed-trait-usage` entry.
#[expect(rustc::potential_query_instability, reason = "collected into another unordered map")]
fn resolve_types(tcx: TyCtxt<'_>, types: &'static [DisallowedPathWithoutReplacement]) -> DisallowedTypes {
    let (def_ids, prim_tys) = create_disallowed_map(tcx, types, PathNS::Type, type_def_kind_predicate, "type", true);

    DisallowedTypes {
        def_ids: def_ids
            .into_items()
            .map(|(def_id, entry)| (def_id, entry.into()))
            .into(),
        prim_tys: prim_tys
            .into_iter()
            .map(|(prim_ty, entry)| (prim_ty, entry.into()))
            .collect(),
    }
}

/// Resolves the `implements` of a `disallowed-trait-usage` entry.
fn resolve_implements(
    tcx: TyCtxt<'_>,
    implements: &'static [DisallowedPathWithoutReplacement],
) -> Vec<(DefId, DisallowedPathEntry)> {
    let (def_ids, _) = create_disallowed_map(tcx, implements, PathNS::Type, trait_def_kind_predicate, "trait", false);

    // Sorted so that diagnostics are emitted in a deterministic order.
    def_ids
        .into_items()
        .into_sorted_stable_ord_by_key(|(_, (path, _))| path)
        .into_iter()
        .map(|(def_id, entry)| (def_id, entry.into()))
        .collect()
}

fn resolve_trait_def_id(tcx: TyCtxt<'_>, path: &str, span: Span) -> Option<DefId> {
    let sym_path: Vec<Symbol> = path.split("::").map(Symbol::intern).collect();
    let def_ids = lookup_path(tcx, PathNS::Type, &sym_path);
    let result = def_ids.iter().find(|&&did| matches!(tcx.def_kind(did), DefKind::Trait));

    if let Some(&def_id) = result {
        Some(def_id)
    } else {
        emit_invalid_path_warning(tcx, &sym_path, path, "trait", span);
        None
    }
}

impl DisallowedTraitUsage {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf, format_args: FormatArgsStorage) -> Self {
        let entries = conf
            .disallowed_trait_usage
            .iter()
            .filter_map(|entry| {
                let trait_def_id = resolve_trait_def_id(tcx, &entry.trait_path, entry.span);

                let forbidden = if let Some(reason) = entry.all_types.as_deref() {
                    if !entry.types.is_empty() || !entry.implements.is_empty() {
                        tcx.sess.dcx().span_warn(
                            entry.span,
                            "`all-types` already covers `types` and `implements`, which are ignored",
                        );
                    }
                    Forbidden::All { reason }
                } else if entry.types.is_empty() && entry.implements.is_empty() {
                    tcx.sess.dcx().span_warn(
                        entry.span,
                        "at least one of `types`, `implements` or `all-types` must be specified",
                    );
                    return None;
                } else {
                    Forbidden::Specific {
                        types: resolve_types(tcx, &entry.types),
                        implements: resolve_implements(tcx, &entry.implements),
                    }
                };

                Some(ResolvedEntry {
                    trait_def_id: trait_def_id?,
                    trait_path: &entry.trait_path,
                    forbidden,
                })
            })
            .collect();

        Self { format_args, entries }
    }

    fn check_type_trait<'tcx>(&self, cx: &LateContext<'tcx>, ty: Ty<'tcx>, trait_def_id: DefId, span: Span) {
        let ty = ty.peel_refs();

        for entry in &self.entries {
            let ResolvedEntry {
                trait_def_id: entry_trait_def_id,
                trait_path,
                forbidden,
            } = entry;

            if *entry_trait_def_id != trait_def_id {
                continue;
            }

            match forbidden {
                Forbidden::All { reason } => span_lint_and_then(
                    cx,
                    DISALLOWED_TRAIT_USAGE,
                    span,
                    format!("use of trait `{trait_path}` is disallowed"),
                    |diag| {
                        if !reason.is_empty() {
                            diag.note((*reason).to_owned());
                        }
                    },
                ),
                Forbidden::Specific { types, implements } => {
                    if let Some(DisallowedPathEntry { path, disallowed_path }) = types.get(ty) {
                        span_lint_and_then(
                            cx,
                            DISALLOWED_TRAIT_USAGE,
                            span,
                            format!("use of `{path}` via trait `{trait_path}` is disallowed"),
                            disallowed_path.diag_amendment(span),
                        );
                    }

                    for &(impl_trait_id, DisallowedPathEntry { path, disallowed_path }) in implements {
                        if implements_trait(cx, ty, impl_trait_id, &[]) {
                            span_lint_and_then(
                                cx,
                                DISALLOWED_TRAIT_USAGE,
                                span,
                                format!("use of implementor of `{path}` via trait `{trait_path}` is disallowed"),
                                disallowed_path.diag_amendment(span),
                            );
                        }
                    }
                },
            }
        }
    }
}

fn ty_as_prim(ty: Ty<'_>) -> Option<PrimTy> {
    match *ty.kind() {
        ty::Bool => Some(PrimTy::Bool),
        ty::Char => Some(PrimTy::Char),
        ty::Str => Some(PrimTy::Str),
        ty::Int(int_ty) => Some(PrimTy::Int(int_ty)),
        ty::Uint(uint_ty) => Some(PrimTy::Uint(uint_ty)),
        ty::Float(float_ty) => Some(PrimTy::Float(float_ty)),
        _ => None,
    }
}

fn format_trait_to_diagnostic_sym(format_trait: FormatTrait) -> Symbol {
    match format_trait {
        FormatTrait::Display => rustc_span::sym::Display,
        FormatTrait::Debug => rustc_span::sym::Debug,
        FormatTrait::LowerExp => sym::LowerExp,
        FormatTrait::UpperExp => sym::UpperExp,
        FormatTrait::Octal => sym::Octal,
        FormatTrait::Pointer => rustc_span::sym::Pointer,
        FormatTrait::Binary => sym::Binary,
        FormatTrait::LowerHex => sym::LowerHex,
        FormatTrait::UpperHex => sym::UpperHex,
    }
}

impl<'tcx> LateLintPass<'tcx> for DisallowedTraitUsage {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if self.entries.is_empty() {
            return;
        }

        // Check format macro arguments
        if let Some(macro_call) = root_macro_call_first_node(cx, expr)
            && is_format_macro(cx, macro_call.def_id)
            && let Some(format_args) = self.format_args.get(cx, expr, macro_call.expn)
        {
            for piece in &format_args.template {
                if let FormatArgsPiece::Placeholder(placeholder) = piece
                    && let Ok(index) = placeholder.argument.index
                    && let Some(arg) = format_args.arguments.all_args().get(index)
                    && let Some(arg_expr) = find_format_arg_expr(expr, arg)
                {
                    let diag_sym = format_trait_to_diagnostic_sym(placeholder.format_trait);
                    if let Some(trait_def_id) = cx.tcx.get_diagnostic_item(diag_sym) {
                        let ty = cx.typeck_results().expr_ty(arg_expr);
                        let report_span = placeholder.span.unwrap_or(arg_expr.span);
                        self.check_type_trait(cx, ty, trait_def_id, report_span);
                    }
                }
            }
            return;
        }

        // Check method calls where the method comes from a trait
        if let ExprKind::MethodCall(name, receiver, _, _) = &expr.kind
            && let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        {
            let method_parent = cx.tcx.parent(method_def_id);
            if matches!(cx.tcx.def_kind(method_parent), DefKind::Trait) {
                let ty = cx.typeck_results().expr_ty(receiver);
                self.check_type_trait(cx, ty, method_parent, name.ident.span);
            }
        }
    }
}
