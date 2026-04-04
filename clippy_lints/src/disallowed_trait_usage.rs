use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::{FormatArgsStorage, find_format_arg_expr, is_format_macro, root_macro_call_first_node};
use clippy_utils::paths::{PathNS, find_crates, lookup_path};
use clippy_utils::sym;
use rustc_ast::{FormatArgsPiece, FormatTrait};
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_session::impl_lint_pass;
use rustc_span::Symbol;

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
    /// disallowed-trait-usage = [
    ///     { type = "std::path::PathBuf", trait = "std::fmt::Debug", reason = "Use path.display() instead" },
    /// ]
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
    #[clippy::version = "1.96.0"]
    pub DISALLOWED_TRAIT_USAGE,
    style,
    "use of a type via a disallowed trait interface"
}

impl_lint_pass!(DisallowedTraitUsage => [DISALLOWED_TRAIT_USAGE]);

/// Identifies a type that may be either an ADT or a primitive.
#[derive(Clone, Copy)]
enum TypeId {
    Def(DefId),
    Bool,
    Char,
    Str,
    Int(ty::IntTy),
    Uint(ty::UintTy),
    Float(ty::FloatTy),
}

/// A resolved disallowed (type, trait) pair.
struct ResolvedEntry {
    type_id: TypeId,
    trait_def_id: DefId,
    type_path: &'static str,
    trait_path: &'static str,
    reason: Option<&'static str>,
}

pub struct DisallowedTraitUsage {
    format_args: FormatArgsStorage,
    entries: Vec<ResolvedEntry>,
}

/// Returns true if path's root crate is loaded (or the path is a single segment).
fn is_crate_loaded(tcx: TyCtxt<'_>, sym_path: &[Symbol]) -> bool {
    sym_path.len() < 2 || !find_crates(tcx, sym_path[0]).is_empty()
}

fn emit_invalid_path_warning(tcx: TyCtxt<'_>, sym_path: &[Symbol], path: &str, expected: &str) {
    if !is_crate_loaded(tcx, sym_path) {
        return;
    }

    // Re-lookup in arbitrary namespace to produce a good "expected X, found Y" message
    let found = lookup_path(tcx, PathNS::Arbitrary, sym_path);
    let message = if let Some(&def_id) = found.first() {
        let (article, description) = tcx.article_and_description(def_id);
        format!("expected a {expected}, found {article} {description}: `{path}` (in `disallowed-trait-usage`)")
    } else {
        format!("`{path}` does not refer to a reachable {expected} (in `disallowed-trait-usage`)")
    };

    tcx.sess.dcx().warn(message);
}

fn resolve_type_id(tcx: TyCtxt<'_>, path: &str) -> Option<TypeId> {
    let sym_name = Symbol::intern(path);
    if let Some(prim) = rustc_hir::PrimTy::from_name(sym_name) {
        return Some(match prim {
            rustc_hir::PrimTy::Int(i) => TypeId::Int(i),
            rustc_hir::PrimTy::Uint(u) => TypeId::Uint(u),
            rustc_hir::PrimTy::Float(f) => TypeId::Float(f),
            rustc_hir::PrimTy::Str => TypeId::Str,
            rustc_hir::PrimTy::Bool => TypeId::Bool,
            rustc_hir::PrimTy::Char => TypeId::Char,
        });
    }

    let sym_path: Vec<Symbol> = path.split("::").map(Symbol::intern).collect();
    let def_ids = lookup_path(tcx, PathNS::Type, &sym_path);
    let result = def_ids.iter().find(|&&did| {
        matches!(
            tcx.def_kind(did),
            DefKind::Struct | DefKind::Union | DefKind::Enum | DefKind::TyAlias | DefKind::ForeignTy
        )
    });

    if let Some(&def_id) = result {
        Some(TypeId::Def(def_id))
    } else {
        emit_invalid_path_warning(tcx, &sym_path, path, "type");
        None
    }
}

fn resolve_trait_def_id(tcx: TyCtxt<'_>, path: &str) -> Option<DefId> {
    let sym_path: Vec<Symbol> = path.split("::").map(Symbol::intern).collect();
    let def_ids = lookup_path(tcx, PathNS::Type, &sym_path);
    let result = def_ids.iter().find(|&&did| matches!(tcx.def_kind(did), DefKind::Trait));

    if let Some(&def_id) = result {
        Some(def_id)
    } else {
        emit_invalid_path_warning(tcx, &sym_path, path, "trait");
        None
    }
}

impl DisallowedTraitUsage {
    pub fn new(tcx: TyCtxt<'_>, conf: &'static Conf, format_args: FormatArgsStorage) -> Self {
        let entries = conf
            .disallowed_trait_usage
            .iter()
            .filter_map(|entry| {
                let type_id = resolve_type_id(tcx, &entry.type_path);
                let trait_def_id = resolve_trait_def_id(tcx, &entry.trait_path);

                Some(ResolvedEntry {
                    type_id: type_id?,
                    trait_def_id: trait_def_id?,
                    type_path: &entry.type_path,
                    trait_path: &entry.trait_path,
                    reason: entry.reason.as_deref(),
                })
            })
            .collect();

        Self { format_args, entries }
    }

    fn check_type_trait<'tcx>(
        &self,
        cx: &LateContext<'tcx>,
        ty: Ty<'tcx>,
        trait_def_id: DefId,
        report_span: rustc_span::Span,
    ) {
        let ty = ty.peel_refs();

        for entry in &self.entries {
            if entry.trait_def_id != trait_def_id {
                continue;
            }

            let matches = match entry.type_id {
                TypeId::Def(def_id) => match ty.kind() {
                    ty::Adt(adt_def, _) => adt_def.did() == def_id,
                    _ => false,
                },
                TypeId::Bool => ty.is_bool(),
                TypeId::Char => ty.is_char(),
                TypeId::Str => ty.is_str(),
                TypeId::Int(int_ty) => matches!(ty.kind(), ty::Int(i) if *i == int_ty),
                TypeId::Uint(uint_ty) => matches!(ty.kind(), ty::Uint(u) if *u == uint_ty),
                TypeId::Float(float_ty) => matches!(ty.kind(), ty::Float(f) if *f == float_ty),
            };

            if matches {
                span_lint_and_then(
                    cx,
                    DISALLOWED_TRAIT_USAGE,
                    report_span,
                    format!(
                        "use of `{}` via trait `{}` is disallowed",
                        entry.type_path, entry.trait_path,
                    ),
                    |diag| {
                        if let Some(reason) = entry.reason {
                            diag.note(reason.to_owned());
                        }
                    },
                );
            }
        }
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
