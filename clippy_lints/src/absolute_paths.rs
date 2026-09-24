use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::macros::{MacroCall, macro_backtrace};
use clippy_utils::paths::{PathNS, find_crates, lookup_path};
use clippy_utils::source::{SpanExt as _, first_line_of_span, snippet_indent};
use clippy_utils::{get_enclosing_block, is_lint_allowed};
use rustc_data_structures::fx::{FxHashMap, FxHashSet, FxIndexMap};
use rustc_errors::Applicability;
use rustc_hir::attrs::AttributeKind;
use rustc_hir::def::{DefKind, Namespace, Res};
use rustc_hir::def_id::{CRATE_DEF_INDEX, DefId, LOCAL_CRATE};
use rustc_hir::intravisit::{Visitor, walk_expr, walk_item, walk_pat, walk_path, walk_ty};
use rustc_hir::{
    AmbigArg, Attribute, Block, Expr, ExprKind, HirId, ImplItemId, Item, ItemId, ItemKind, Node, Pat, PatKind, Path,
    QPath, Stmt, StmtKind, TraitItemId, Ty, TyKind, UseKind, find_attr,
};
use rustc_lexer::is_ident;
use rustc_lint::{LateContext, LateLintPass, impl_lint_pass};
use rustc_middle::hir::nested_filter;
use rustc_span::def_id::LocalModId;
use rustc_span::hygiene::MacroKind;
use rustc_span::symbol::kw;
use rustc_span::{BytePos, ExpnId, Pos as _, Span, Symbol};
use std::ops::ControlFlow;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of items through absolute paths, like `std::f64::consts::PI`, including
    /// macro and derive paths, like `core::ptr::addr_of!(..)`. Where it is safe to, it suggests
    /// importing the item and shortening the path.
    ///
    /// ### Why restrict this?
    /// Many codebases have their own style when it comes to importing, but one that is seldom used
    /// is using absolute paths *everywhere*. This is generally considered unidiomatic, and you
    /// should add a `use` statement.
    ///
    /// The default maximum segments (2) is pretty strict, you may want to increase this in
    /// `clippy.toml`.
    ///
    /// Note: One exception to this is code from macro expansion - this does not lint such cases, as
    /// using absolute paths is the proper way of referencing items in one.
    ///
    /// ### Known issues
    ///
    /// Attribute macro paths, e.g. `#[path::to::macro]`, are not caught: importing one can take
    /// over a built-in attribute of the same name, such as `test`.
    ///
    /// The suggested `use` goes at the top of the module, or, for a usage behind a `#[cfg(..)]`
    /// gate, where it inherits the same gate. No suggestion is made when the shortened name is
    /// already taken, or when the import would change what a name from the prelude or a glob
    /// import means elsewhere in the scope. Only compiled code is taken into account, so a name
    /// used under a disabled `#[cfg(..)]` can still clash, which is why the suggestion is not
    /// applied automatically.
    ///
    /// ### Example
    /// ```no_run
    /// let x = std::f64::consts::PI;
    /// ```
    /// Use any of the below instead, or anything else:
    /// ```no_run
    /// use std::f64;
    /// use std::f64::consts;
    /// use std::f64::consts::PI;
    /// let x = f64::consts::PI;
    /// let x = consts::PI;
    /// let x = PI;
    /// use std::f64::consts as f64_consts;
    /// let x = f64_consts::PI;
    /// ```
    #[clippy::version = "1.73.0"]
    pub ABSOLUTE_PATHS,
    restriction,
    "checks for usage of an item without a `use` statement"
}

impl_lint_pass!(AbsolutePaths => [ABSOLUTE_PATHS]);

pub struct AbsolutePaths {
    max_segments: u64,
    allowed_crates: &'static FxHashSet<Symbol>,
    /// Planned once the whole crate is seen, so each scope gets one sorted `use` block.
    candidates: Vec<Candidate>,
    seen_expns: FxHashSet<ExpnId>,
}

impl AbsolutePaths {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            max_segments: conf.absolute_paths_max_segments,
            allowed_crates: &conf.absolute_paths_allowed_crates,
            candidates: Vec::new(),
            seen_expns: FxHashSet::default(),
        }
    }
}

struct Candidate {
    hir_id: HirId,
    path_span: Span,
    /// The leading portion of the path, deleted to shorten it. `None` when that can't be done.
    shorten_span: Option<Span>,
    use_path: String,
    name: Symbol,
}

enum Fix {
    /// Every occurrence in a scope carries the same `use` block, so each suggestion works on its
    /// own, while rustfix applies an identical replacement only once.
    ImportAndShorten {
        at: Span,
        uses: String,
        count: usize,
        shorten: Span,
    },
    Shorten(Span),
    None,
}

impl<'tcx> LateLintPass<'tcx> for AbsolutePaths {
    fn check_path(&mut self, cx: &LateContext<'tcx>, path: &Path<'tcx>, hir_id: HirId) {
        // Candidates are kept until the end of the crate, so don't collect any while the lint is off.
        if is_lint_allowed(cx, ABSOLUTE_PATHS, hir_id) {
            return;
        }
        let segments = match path.segments {
            [] | [_] => return,
            // Don't count enum variants and trait items as part of the length.
            [rest @ .., _]
                if let [.., s] = rest
                    && matches!(s.res, Res::Def(DefKind::Enum | DefKind::Trait | DefKind::TraitAlias, _)) =>
            {
                rest
            },
            path => path,
        };

        let [s1, s2, ..] = segments else { return };
        let has_root = s1.ident.name == kw::PathRoot;
        let first = if has_root { s2 } else { s1 };
        let len = segments.len() - usize::from(has_root);
        if len as u64 <= self.max_segments {
            return;
        }

        let crate_name = if let Res::Def(DefKind::Mod, DefId { index, .. }) = first.res
            && index == CRATE_DEF_INDEX
        {
            // `other_crate::foo` or `::other_crate::foo`
            first.ident.name
        } else if first.ident.name == kw::Crate || has_root {
            // `::foo` or `crate::foo`
            kw::Crate
        } else {
            return;
        };

        if path.span.from_expansion() {
            return;
        }
        let node = cx.tcx.hir_node(hir_id);
        if matches!(node, Node::Item(item) if matches!(item.kind, ItemKind::Use(..))) {
            return;
        }
        if self.allowed_crates.contains(&crate_name) {
            return;
        }
        if clippy_utils::is_from_proc_macro(cx, path) {
            return;
        }

        let use_segments: Vec<Symbol> = segments
            .iter()
            .map(|s| s.ident.name)
            .filter(|&n| n != kw::PathRoot)
            .collect();
        let last_seg = &segments[segments.len() - 1];

        // Deleted from the first segment, not `path.span`, which in `<T as a::B>::c` starts at `<`.
        // Generic args on the final segment are kept.
        let first_lo = first.ident.span.lo();
        let lo = if has_root {
            let root = path.span.with_lo(first_lo - BytePos(2)).with_hi(first_lo);
            root.get_text(cx).is_some_and(|t| &*t == "::").then(|| root.lo())
        } else {
            Some(first_lo)
        };
        let shorten_span = lo
            .filter(|_| use_segments.len() > 1)
            .map(|lo| path.span.with_lo(lo).with_hi(last_seg.ident.span.lo()));

        self.candidates.push(Candidate {
            hir_id,
            path_span: path.span,
            shorten_span,
            use_path: use_segments.iter().map(Symbol::as_str).collect::<Vec<_>>().join("::"),
            name: last_seg.ident.name,
        });
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        self.check_macro_calls(cx, expr.span, expr.hir_id);
    }

    fn check_pat(&mut self, cx: &LateContext<'tcx>, pat: &'tcx Pat<'tcx>) {
        self.check_macro_calls(cx, pat.span, pat.hir_id);
    }

    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &'tcx Ty<'tcx, AmbigArg>) {
        self.check_macro_calls(cx, ty.span, ty.hir_id);
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // A derive's output is an item of its own, so lint levels come from the type it is on.
        let hir_id = if let ItemKind::Impl(imp) = item.kind
            && let TyKind::Path(QPath::Resolved(None, path)) = imp.self_ty.kind
            && let Res::Def(_, def_id) = path.res
            && let Some(def_id) = def_id.as_local()
        {
            cx.tcx.local_def_id_to_hir_id(def_id)
        } else {
            item.hir_id()
        };
        self.check_macro_calls(cx, item.span, hir_id);
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        for (candidate, fix) in plan(cx, std::mem::take(&mut self.candidates)) {
            let Candidate {
                hir_id,
                path_span,
                use_path,
                ..
            } = candidate;
            span_lint_hir_and_then(
                cx,
                ABSOLUTE_PATHS,
                hir_id,
                path_span,
                "consider bringing this path into scope with the `use` keyword",
                |diag| match fix {
                    Fix::ImportAndShorten {
                        at,
                        uses,
                        count,
                        shorten,
                    } => {
                        let msg = if count > 1 {
                            "bring these items into scope and shorten the path"
                        } else {
                            "bring the item into scope and shorten the path"
                        };
                        diag.multipart_suggestion(
                            msg,
                            vec![(at, uses), (shorten, String::new())],
                            Applicability::MaybeIncorrect,
                        );
                    },
                    Fix::Shorten(shorten) => {
                        diag.multipart_suggestion(
                            "shorten the path",
                            vec![(shorten, String::new())],
                            Applicability::MaybeIncorrect,
                        );
                    },
                    Fix::None => {
                        diag.help(format!("add `use {use_path};`"));
                    },
                },
            );
        }
    }
}

impl AbsolutePaths {
    /// Looks for absolute macro paths, like `core::ptr::addr_of!(..)` or
    /// `#[derive(core::clone::Clone)]`, behind a node produced by macro expansion.
    fn check_macro_calls(&mut self, cx: &LateContext<'_>, span: Span, hir_id: HirId) {
        if !span.from_expansion() || is_lint_allowed(cx, ABSOLUTE_PATHS, hir_id) {
            return;
        }
        for call in macro_backtrace(span) {
            // A path written by a macro itself is not the user's to change, so only calls written
            // out in the source are considered.
            if !call.span.from_expansion()
                && self.seen_expns.insert(call.expn)
                && let Some(candidate) = self.macro_candidate(cx, &call, hir_id)
            {
                self.candidates.push(candidate);
            }
        }
    }

    fn macro_candidate(&self, cx: &LateContext<'_>, call: &MacroCall, hir_id: HirId) -> Option<Candidate> {
        let text = call_path_text(cx, call)?;
        let (has_root, rest) = text
            .strip_prefix("::")
            .map_or((false, text.as_str()), |rest| (true, rest));
        let segments: Vec<&str> = rest.split("::").collect();
        // Only a plain `a::b::c` is rewritten, not one with whitespace, comments or raw identifiers.
        if !segments.iter().all(|s| is_ident(s)) || segments.len() as u64 <= self.max_segments {
            return None;
        }
        let (&first, &last) = (segments.first()?, segments.last()?);
        // The written name must be the macro's own, so that the text read back from the source
        // really is the path that was resolved.
        if cx.tcx.item_name(call.def_id).as_str() != last {
            return None;
        }

        let first_sym = Symbol::intern(first);
        let crate_name = if first != "crate" && !find_crates(cx.tcx, first_sym).is_empty() {
            first_sym
        } else if first == "crate" || has_root {
            kw::Crate
        } else {
            return None;
        };
        if self.allowed_crates.contains(&crate_name) {
            return None;
        }

        let lo = call.span.lo();
        Some(Candidate {
            hir_id,
            path_span: call.span.with_hi(lo + BytePos::from_usize(text.len())),
            shorten_span: Some(call.span.with_hi(lo + BytePos::from_usize(text.rfind("::")? + 2))),
            use_path: segments.join("::"),
            name: Symbol::intern(last),
        })
    }
}

/// Source text of the path a macro was called by: `core::ptr::addr_of` for
/// `core::ptr::addr_of!(x)`, or the path inside `#[derive(..)]`.
///
/// Attribute macros are skipped, since `use tokio::test;` would turn every plain `#[test]` in the
/// module into a tokio test, and those are not compiled outside test builds for the check to see.
fn call_path_text(cx: &LateContext<'_>, call: &MacroCall) -> Option<String> {
    let text = call.span.get_text(cx)?;
    match call.kind {
        MacroKind::Bang => Some(text.split_once('!')?.0.to_owned()),
        MacroKind::Derive => Some(text.to_string()),
        MacroKind::Attr => None,
    }
}

struct Scope {
    indent: String,
    /// The character `at` covers, restored after the block.
    displaced: String,
    /// Imported path and its `#[cfg(..)]` lines per bound name, so a second path wanting a taken
    /// name is turned down.
    imports: FxIndexMap<Symbol, (String, String)>,
}

enum Planned {
    Import(Span),
    Shorten,
    None,
}

fn plan(cx: &LateContext<'_>, candidates: Vec<Candidate>) -> Vec<(Candidate, Fix)> {
    let mut scopes: FxIndexMap<Span, Scope> = FxIndexMap::default();
    let mut bare_uses: FxHashMap<Span, FxHashMap<Symbol, Vec<Res>>> = FxHashMap::default();
    let mut imported: FxHashMap<String, Vec<(Namespace, DefId)>> = FxHashMap::default();
    let planned: Vec<Planned> = candidates
        .iter()
        .map(|c| {
            // An `#[expect]`ed occurrence's suggestion is never applied, so its path must not be
            // pulled into a scope's `use` block.
            if c.shorten_span.is_none() || cx.tcx.lint_level_spec_at_node(ABSOLUTE_PATHS, c.hir_id).is_expect() {
                return Planned::None;
            }
            let imported = imported
                .entry(c.use_path.clone())
                .or_insert_with(|| imported_items(cx, &c.use_path));
            if imported.is_empty() {
                return Planned::None;
            }
            match existing_binding(cx, c.hir_id, c.name, imported) {
                Binding::ToSame => return Planned::Shorten,
                // A taken name would collide (`E0252`, `E0255`) or resolve the shortened path wrongly.
                Binding::ToOther => return Planned::None,
                Binding::Free => {},
            }
            let Some(Anchor {
                at,
                prefix,
                indent,
                scope,
            }) = insertion_point(cx, c.hir_id, c.path_span)
            else {
                return Planned::None;
            };
            // Nor may the import change what a prelude or glob-imported name means in the scope:
            // bringing in a different `Vec` breaks every plain `Vec` around it.
            let uses = bare_uses.entry(at).or_insert_with(|| BareUses::collect(cx, scope));
            if uses.get(&c.name).is_some_and(|uses| shadows(imported, uses)) {
                return Planned::None;
            }
            let Some(displaced) = at.get_text(cx) else {
                return Planned::None;
            };
            let scope = scopes.entry(at).or_insert_with(|| Scope {
                indent,
                displaced: displaced.to_string(),
                imports: FxIndexMap::default(),
            });
            match scope.imports.get(&c.name) {
                Some((path, _)) if *path != c.use_path => Planned::None,
                Some(_) => Planned::Import(at),
                None => {
                    scope.imports.insert(c.name, (c.use_path.clone(), prefix));
                    Planned::Import(at)
                },
            }
        })
        .collect();

    let blocks: FxHashMap<Span, String> = scopes
        .iter()
        .map(|(&at, scope)| {
            let mut imports: Vec<&(String, String)> = scope.imports.values().collect();
            imports.sort_unstable();
            let mut uses = imports.iter().fold(String::new(), |mut uses, (path, prefix)| {
                uses.push_str(prefix);
                uses.push_str("use ");
                uses.push_str(path);
                uses.push_str(";\n");
                uses.push_str(&scope.indent);
                uses
            });
            uses.push_str(&scope.displaced);
            (at, uses)
        })
        .collect();

    candidates
        .into_iter()
        .zip(planned)
        .map(|(c, planned)| {
            let fix = match (planned, c.shorten_span) {
                (Planned::Import(at), Some(shorten)) => Fix::ImportAndShorten {
                    at,
                    uses: blocks[&at].clone(),
                    count: scopes[&at].imports.len(),
                    shorten,
                },
                (Planned::Shorten, Some(shorten)) => Fix::Shorten(shorten),
                _ => Fix::None,
            };
            (c, fix)
        })
        .collect()
}

/// Where the `use` for an occurrence belongs.
struct Anchor<'tcx> {
    at: Span,
    /// `#[cfg(..)]` lines to replicate ahead of the `use`.
    prefix: String,
    /// Indentation restoring the line the insertion displaces.
    indent: String,
    /// Where the `use` is visible once inserted.
    scope: UseScope<'tcx>,
}

#[derive(Clone, Copy)]
enum UseScope<'tcx> {
    Module(LocalModId),
    Block(&'tcx Block<'tcx>),
}

fn insertion_point<'tcx>(cx: &LateContext<'tcx>, hir_id: HirId, path_span: Span) -> Option<Anchor<'tcx>> {
    match nearest_cfg_gate(cx, hir_id) {
        Some(GatedBy::Block(block)) if block.span.contains(path_span) => block_insertion_anchor(cx, block),
        // Also a gated block not containing the path, i.e. a path in a gated `fn`'s signature.
        Some(_) => item_insertion_anchor(cx, hir_id),
        None => module_insertion_anchor(cx, hir_id),
    }
}

enum GatedBy<'tcx> {
    Item,
    Block(&'tcx Block<'tcx>),
}

/// Finds the nearest ancestor carrying a `#[cfg(..)]`/`#[cfg_attr(..)]` attribute and classifies it
/// as gating via an inner block or via the enclosing item.
fn nearest_cfg_gate<'tcx>(cx: &LateContext<'tcx>, hir_id: HirId) -> Option<GatedBy<'tcx>> {
    let tcx = cx.tcx;
    for id in std::iter::once(hir_id).chain(tcx.hir_parent_id_iter(hir_id)) {
        if find_attr!(tcx, id, CfgTrace(..) | CfgAttrTrace(..)) {
            let node = tcx.hir_node(id);
            if let Node::Item(_) = node {
                return Some(GatedBy::Item);
            }
            // A gated `{ .. }` statement or expression gates its own block; any other gated node
            // is gated along with the block that contains it.
            let block = match node {
                Node::Block(block) => Some(block),
                Node::Stmt(&Stmt {
                    kind: StmtKind::Expr(e) | StmtKind::Semi(e),
                    ..
                })
                | Node::Expr(e) => match e.kind {
                    ExprKind::Block(block, _) => Some(block),
                    _ => None,
                },
                _ => None,
            };
            return Some(
                block
                    .or_else(|| get_enclosing_block(cx, id))
                    .map_or(GatedBy::Item, GatedBy::Block),
            );
        }
    }
    None
}

enum Binding {
    Free,
    /// Already imported, so the path only needs shortening.
    ToSame,
    ToOther,
}

/// What `name` is already bound to by an item, import, generic parameter or local binding that
/// would collide with, or take precedence over, a `use` for the usage at `hir_id`.
fn existing_binding(cx: &LateContext<'_>, hir_id: HirId, name: Symbol, imported: &[(Namespace, DefId)]) -> Binding {
    let tcx = cx.tcx;
    let item_binding = |id: ItemId| {
        let item = tcx.hir_item(id);
        match item.kind {
            _ if item.kind.ident().is_none_or(|i| i.name != name) => Binding::Free,
            ItemKind::Use(path, UseKind::Single(_))
                if path
                    .res
                    .present_items()
                    .all(|res| imported.iter().any(|&(_, id)| res.opt_def_id() == Some(id))) =>
            {
                Binding::ToSame
            },
            _ => Binding::ToOther,
        }
    };

    let mut found = Binding::Free;
    let mut record = |binding| match binding {
        Binding::ToOther => found = Binding::ToOther,
        Binding::ToSame if matches!(found, Binding::Free) => found = Binding::ToSame,
        _ => {},
    };
    for id in tcx.hir_parent_id_iter(hir_id) {
        let node = tcx.hir_node(id);
        if node
            .generics()
            .is_some_and(|g| g.params.iter().any(|p| p.name.ident().name == name))
        {
            record(Binding::ToOther);
        }
        if let Node::Block(block) = node {
            for stmt in block.stmts {
                if let StmtKind::Item(id) = stmt.kind {
                    record(item_binding(id));
                }
            }
        }
    }
    let (module, ..) = tcx.hir_get_module(tcx.parent_module(hir_id));
    for &id in module.item_ids {
        record(item_binding(id));
    }
    // Any binding of the name in the body counts, wherever it is. Working out which of them are
    // actually in scope at the usage is not worth it for the rare case where one is not.
    if tcx
        .hir_maybe_body_owned_by(hir_id.owner.def_id)
        .is_some_and(|body| BindingFinder { cx, name }.visit_body(body).is_break())
    {
        record(Binding::ToOther);
    }
    found
}

struct BindingFinder<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    name: Symbol,
}

impl<'tcx> Visitor<'tcx> for BindingFinder<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;
    type Result = ControlFlow<()>;

    fn visit_pat(&mut self, pat: &'tcx Pat<'tcx>) -> Self::Result {
        if let PatKind::Binding(_, _, ident, _) = pat.kind
            && ident.name == self.name
        {
            return ControlFlow::Break(());
        }
        walk_pat(self, pat)
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}

/// Insertion point at the top of the enclosing module, on rustc's own `use`-injection line, which
/// sits after any inner attributes.
fn module_insertion_anchor<'tcx>(cx: &LateContext<'tcx>, hir_id: HirId) -> Option<Anchor<'tcx>> {
    let module_id = cx.tcx.parent_module(hir_id);
    let (module, ..) = cx.tcx.hir_get_module(module_id);
    let (at, indent) = line_start_anchor(cx, module.spans.inject_use_span)?;
    Some(Anchor {
        at,
        prefix: String::new(),
        indent,
        scope: UseScope::Module(module_id),
    })
}

/// Insertion point before the enclosing item, replicating its `#[cfg(..)]` attributes so the `use`
/// is gated identically and also in scope for the item's signature.
fn item_insertion_anchor<'tcx>(cx: &LateContext<'tcx>, hir_id: HirId) -> Option<Anchor<'tcx>> {
    let tcx = cx.tcx;

    let (item_id, item_span) =
        std::iter::once(hir_id)
            .chain(tcx.hir_parent_id_iter(hir_id))
            .find_map(|id| match tcx.hir_node(id) {
                Node::Item(item) => Some((id, item.span)),
                _ => None,
            })?;

    // Recover the `#[cfg(..)]` attributes from source, bailing out if their text is unavailable.
    let mut cfgs = String::new();
    for attr in tcx.hir_attrs(item_id) {
        if let Attribute::Parsed(AttributeKind::CfgTrace(entries)) = attr {
            for (_, cfg_span) in entries {
                cfgs.push_str(&cfg_span.get_text(cx)?);
                cfgs.push('\n');
            }
        }
    }
    if cfgs.is_empty() {
        return None;
    }

    let (at, indent) = line_start_anchor(cx, span_with_attrs(cx, item_id, item_span))?;
    let mut prefix = String::new();
    for line in cfgs.lines() {
        prefix.push_str(line);
        prefix.push('\n');
        prefix.push_str(&indent);
    }

    Some(Anchor {
        at,
        prefix,
        indent,
        scope: UseScope::Module(tcx.parent_module(item_id)),
    })
}

fn block_insertion_anchor<'tcx>(cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) -> Option<Anchor<'tcx>> {
    let first_span = match (block.stmts.first(), block.expr) {
        (Some(stmt), _) => span_with_attrs(cx, stmt.hir_id, stmt.span),
        (None, Some(expr)) => span_with_attrs(cx, expr.hir_id, expr.span),
        (None, None) => return None,
    };
    let (at, indent) = line_start_anchor(cx, first_span)?;
    Some(Anchor {
        at,
        prefix: String::new(),
        indent,
        scope: UseScope::Block(block),
    })
}

/// Extends `span` up over the attributes attached to `id`, so that inserting at its start cannot
/// land between an attribute and the code it applies to, which would re-target the attribute.
fn span_with_attrs(cx: &LateContext<'_>, id: HirId, span: Span) -> Span {
    let lo = cx
        .tcx
        .hir_attrs(id)
        .iter()
        .map(Attribute::span)
        .filter(|s| !s.from_expansion())
        .fold(span.lo(), |lo, s| lo.min(s.lo()));
    span.with_lo(lo)
}

/// The first character of `span`'s line, which the `use` block goes in front of, plus the line's
/// indentation. `None` unless `span` starts its line, as otherwise the `use` could land outside its
/// scope, as in a one-line `mod m { .. }`.
fn line_start_anchor(cx: &LateContext<'_>, span: Span) -> Option<(Span, String)> {
    if span.from_expansion() {
        return None;
    }
    let lo = first_line_of_span(cx, span).lo();
    // Read from the line, as `span` can be empty, like a module's `use` injection point.
    let line = cx.tcx.sess.source_map().span_extend_to_line(span.shrink_to_lo());
    let first = line.get_text(cx)?.get((lo - line.lo()).to_usize()..)?.chars().next()?;
    (lo == span.lo()).then(|| {
        (
            span.with_hi(lo + BytePos::from_usize(first.len_utf8())),
            snippet_indent(cx, span).unwrap_or_default(),
        )
    })
}

/// Everything a `use` of `use_path` brings into scope, in each namespace. Empty when the path cannot
/// be resolved, in which case nothing can be said about what the import would shadow.
fn imported_items(cx: &LateContext<'_>, use_path: &str) -> Vec<(Namespace, DefId)> {
    let path: Vec<Symbol> = use_path
        .split("::")
        .map(|s| {
            if s == "crate" {
                cx.tcx.crate_name(LOCAL_CRATE)
            } else {
                Symbol::intern(s)
            }
        })
        .collect();
    [
        (PathNS::Type, Namespace::TypeNS),
        (PathNS::Value, Namespace::ValueNS),
        (PathNS::Macro, Namespace::MacroNS),
    ]
    .into_iter()
    .flat_map(|(path_ns, ns)| lookup_path(cx.tcx, path_ns, &path).into_iter().map(move |id| (ns, id)))
    .collect()
}

/// Whether importing `imported` would change what any of `uses`, the existing bare uses of the
/// same name, resolves to.
fn shadows(imported: &[(Namespace, DefId)], uses: &[Res]) -> bool {
    uses.iter().any(|res| {
        let Some(ns) = res.ns() else { return false };
        let mut in_ns = imported.iter().filter(|&&(n, _)| n == ns).peekable();
        in_ns.peek().is_some() && !in_ns.any(|&(_, id)| res.opt_def_id() == Some(id))
    })
}

/// The names used without a path in a scope, and what each resolves to: the first segment of
/// every relative path, and every macro called by a bare name.
struct BareUses<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    uses: FxHashMap<Symbol, Vec<Res>>,
    seen_expns: FxHashSet<ExpnId>,
}

impl<'a, 'tcx> BareUses<'a, 'tcx> {
    fn collect(cx: &'a LateContext<'tcx>, scope: UseScope<'tcx>) -> FxHashMap<Symbol, Vec<Res>> {
        let mut this = Self {
            cx,
            uses: FxHashMap::default(),
            seen_expns: FxHashSet::default(),
        };
        match scope {
            UseScope::Module(module) => {
                for &id in cx.tcx.hir_get_module(module).0.item_ids {
                    this.visit_nested_item(id);
                }
            },
            UseScope::Block(block) => this.visit_block(block),
        }
        this.uses
    }

    fn macro_calls(&mut self, span: Span) {
        if !span.from_expansion() {
            return;
        }
        for call in macro_backtrace(span) {
            if !self.seen_expns.insert(call.expn) {
                continue;
            }
            let name = if call.span.from_expansion() {
                // Inside another macro's body the path can't be read back, but a bare name there
                // resolves where that macro is used, so count it under the macro's own name.
                self.cx.tcx.item_name(call.def_id)
            } else if let Some(path) = call_path_text(self.cx, &call)
                && is_ident(&path)
            {
                Symbol::intern(&path)
            } else {
                continue;
            };
            let res = Res::Def(self.cx.tcx.def_kind(call.def_id), call.def_id);
            self.uses.entry(name).or_default().push(res);
        }
    }
}

impl<'tcx> Visitor<'tcx> for BareUses<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_nested_item(&mut self, id: ItemId) {
        let item = self.cx.tcx.hir_item(id);
        // A child module is a scope of its own, which the `use` does not reach.
        if !matches!(item.kind, ItemKind::Mod(..)) {
            self.visit_item(item);
        }
    }

    fn visit_nested_impl_item(&mut self, id: ImplItemId) {
        self.visit_impl_item(self.cx.tcx.hir_impl_item(id));
    }

    fn visit_nested_trait_item(&mut self, id: TraitItemId) {
        self.visit_trait_item(self.cx.tcx.hir_trait_item(id));
    }

    fn visit_path(&mut self, path: &Path<'tcx>, _: HirId) {
        if let [first, ..] = path.segments
            && !first.ident.is_path_segment_keyword()
        {
            self.uses.entry(first.ident.name).or_default().push(first.res);
        }
        walk_path(self, path);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        self.macro_calls(expr.span);
        walk_expr(self, expr);
    }

    fn visit_pat(&mut self, pat: &'tcx Pat<'tcx>) {
        self.macro_calls(pat.span);
        walk_pat(self, pat);
    }

    fn visit_ty(&mut self, ty: &'tcx Ty<'tcx, AmbigArg>) {
        self.macro_calls(ty.span);
        walk_ty(self, ty);
    }

    fn visit_item(&mut self, item: &'tcx Item<'tcx>) {
        self.macro_calls(item.span);
        walk_item(self, item);
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}
