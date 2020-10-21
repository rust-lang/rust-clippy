use crate::utils::span_lint_and_then;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_hir::def::Res;
use rustc_hir::def_id::DefId;
use rustc_hir::intravisit::{self, NestedVisitorMap, Visitor};
use rustc_hir::CRATE_HIR_ID;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, TyCtxt};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{symbol::Ident, Span, Symbol};

declare_clippy_lint! {
    /// **What it does:**
    ///
    /// Checks if a `pub(crate)` visibility modifier was unnecessary given the
    /// way the item was actually used.
    ///
    /// **Why is this bad?**
    ///
    /// You may be decieved into thinking an item is used far away, when it is not!
    ///
    /// **Known problems:** Does not check positional fields.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// mod outer {
    ///     mod inner {
    ///         pub(crate) fn foo() { } // this function is never used in a `pub(crate)` fashion, does it really need to be `pub(crate)`?
    ///         pub(crate) fn bar() { foo() }
    ///     }
    ///     pub fn main() {
    ///         inner::bar();
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// mod outer {
    ///     mod inner {
    ///         fn foo() { } // this function is never used in a `pub(crate)` fashion, does it really need to be `pub(crate)`?
    ///         pub(crate) fn bar() { foo() }
    ///     }
    ///     pub fn main() {
    ///         inner::bar();
    ///     }
    /// }
    /// ```
    pub UNNEEDED_PUB_CRATE,
    pedantic,
    "Using `pub(crate)` visibility on items that are only accessed from within the module that contains the item."
}

#[derive(Default, Debug)]
pub struct UnneededPubCrate {
    watched_item_map: FxHashMap<hir::HirId, WatchedItem>,
    current_module_path: Vec<hir::HirId>,
}

impl_lint_pass!(UnneededPubCrate => [UNNEEDED_PUB_CRATE]);

/// An item with `pub(crate)` visibiliy that we're watching to see if it's
/// referenced in ways where the visibility was useful.
#[derive(Debug, Clone)]
struct WatchedItem {
    enclosing_module: hir::HirId,
    status: WatchStatus,
    /// The span of the visibility modifier
    span: Span,
    // the fates of these watched items are intertwined; if any of them become
    // `CrateReference`, they all are considered `CrateReference`.
    linked_fate: Vec<hir::HirId>,
}

/// The status of an item we're watching, accumulated while we check the HIR.
#[derive(Copy, Clone, Debug)]
enum WatchStatus {
    /// An item starts off unreferenced. If it nevers leaves this state, that
    /// means the item is very dead.
    Unreferenced,
    /// We've only seen local references to this item. If an item ends in this
    /// state, we can demote the `pub(crate)` to `priv`.
    LocalReference,
    /// We've seen at least one reference from somewhere in the crate that didn't qualify for
    /// `LocalReference`.
    CrateReference,
}
use WatchStatus::*;

impl WatchStatus {
    fn observe(&mut self, observation: WatchStatus) {
        match (*self, observation) {
            (Unreferenced, _) => *self = observation,
            (LocalReference, CrateReference) => *self = CrateReference,
            _ => {},
        }
    }
}

struct UseScanner<'tcx> {
    tcx: TyCtxt<'tcx>,
    maybe_typeck_results: Option<&'tcx ty::TypeckResults<'tcx>>,
    watched_item_map: FxHashMap<hir::HirId, WatchedItem>,
    current_module_path: Vec<hir::HirId>,
}

impl<'tcx> Visitor<'tcx> for UseScanner<'tcx> {
    type Map = rustc_middle::hir::map::Map<'tcx>;

    fn nested_visit_map(&mut self) -> NestedVisitorMap<Self::Map> {
        NestedVisitorMap::All(self.tcx.hir())
    }

    fn visit_nested_body(&mut self, body: hir::BodyId) {
        let old_maybe_typeck_results = self.maybe_typeck_results.replace(self.tcx.typeck_body(body));
        let body = self.tcx.hir().body(body);
        self.visit_body(body);
        self.maybe_typeck_results = old_maybe_typeck_results;
    }

    fn visit_mod(&mut self, mod_: &'tcx hir::Mod<'tcx>, _span: Span, hir_id: hir::HirId) {
        self.current_module_path.push(hir_id);
        intravisit::walk_mod(self, mod_, hir_id);
        self.current_module_path
            .pop()
            .expect("mismatched push/pop in UnneededPubCrate::check_mod_post");
    }

    fn visit_variant_data(
        &mut self,
        vd: &'tcx hir::VariantData<'tcx>,
        _name: Symbol,
        _generics: &'tcx hir::Generics<'tcx>,
        parent_id: hir::HirId,
        _span: Span,
    ) {
        if let Some(hir_id) = vd.ctor_hir_id() {
            self.examine_use(self.tcx.hir().local_def_id(hir_id).to_def_id(), parent_id)
        }
    }

    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        match expr.kind {
            hir::ExprKind::MethodCall(..) => {
                self.maybe_typeck_results
                    .and_then(|typeck_results| typeck_results.type_dependent_def(expr.hir_id))
                    .map(|(_kind, def_id)| self.examine_use(def_id, expr.hir_id));
            },
            hir::ExprKind::Struct(_qpath, fields, _base) => {
                if let Some(ty::Adt(adt_def, _substs)) = self
                    .maybe_typeck_results
                    .map(|typeck_results| typeck_results.expr_ty(expr).kind())
                {
                    self.examine_use(adt_def.did, expr.hir_id);
                    let ty_fields = adt_def
                        .all_fields()
                        .map(|f| (f.ident, f))
                        .collect::<FxHashMap<Ident, _>>();
                    for field in fields.iter() {
                        if let Some(ty_field) = ty_fields.get(&field.ident) {
                            self.examine_use(ty_field.did, expr.hir_id);
                        }
                    }
                }
            },
            hir::ExprKind::Field(base, field) => {
                if let Some(ty::Adt(adt_def, _substs)) = self
                    .maybe_typeck_results
                    .map(|typeck_results| typeck_results.expr_ty(base).kind())
                {
                    self.examine_use(adt_def.did, expr.hir_id);
                    if let Some(our_field) = adt_def.all_fields().filter(|f| f.ident == field).next() {
                        self.examine_use(our_field.did, expr.hir_id);
                    }
                }
            },
            _ => {},
        };
        intravisit::walk_expr(self, expr);
    }

    fn visit_trait_ref(&mut self, trait_ref: &'tcx hir::TraitRef<'tcx>) {
        if let Some(trait_ref) = trait_ref.trait_def_id() {
            self.examine_use(trait_ref, CRATE_HIR_ID);
        }
        intravisit::walk_trait_ref(self, trait_ref);
    }

    fn visit_qpath(&mut self, qpath: &'tcx hir::QPath<'tcx>, hir_id: hir::HirId, span: Span) {
        let def = match qpath {
            hir::QPath::Resolved(_, path) => match path.res {
                Res::Def(kind, def_id) => Some((kind, def_id)),
                _ => None,
            },
            hir::QPath::TypeRelative(..) | hir::QPath::LangItem(..) => self
                .maybe_typeck_results
                .and_then(|typeck_results| typeck_results.type_dependent_def(hir_id)),
        };
        if let Some((_kind, def_id)) = def {
            self.examine_use(def_id, hir_id);
        }
        intravisit::walk_qpath(self, qpath, hir_id, span);
    }

    /*fn visit_path(&mut self, path: &'tcx hir::Path<'tcx>, _hir_id: hir::HirId) {
        match path.res {
            Res::Def(_kind, def_id) => {
                self.examine_use(def_id);
            },
            _ => {},
        }
        intravisit::walk_path(self, path);
    }*/
}

impl<'tcx> UseScanner<'tcx> {
    fn observe(&mut self, what: hir::HirId, how: WatchStatus) {
        let mut worklist = vec![what];
        let mut seen = FxHashSet::default();
        while let Some(work) = worklist.pop() {
            seen.insert(work);
            match self.watched_item_map.get_mut(&work) {
                Some(watch_item) => {
                    watch_item.status.observe(how);
                    worklist.extend(watch_item.linked_fate.iter().cloned().filter(|e| !seen.contains(&e)))
                },
                None => panic!("uh, why couldn't i find this? their fates are supposed to be linked!"),
            }
        }
    }

    fn examine_use(&mut self, def_id: DefId, _used_by: hir::HirId) {
        // that _used_by is super useful when debugging :)
        if let Some(node) = self.tcx.hir().get_if_local(def_id) {
            match node.hir_id() {
                Some(hir_id) => {
                    if let Some(watch_item) = self.watched_item_map.get(&hir_id).map(WatchedItem::clone) {
                        if self.current_module_path.contains(&watch_item.enclosing_module) {
                            // if the current path contains the
                            // enclosing module of the watched item
                            // somewhere, we would have been able to
                            // reference it even if it weren't marked
                            // `pub(crate)`.
                            self.observe(hir_id, LocalReference);
                        } else {
                            self.observe(hir_id, CrateReference);
                        }
                    } else {
                        // not a tracked item
                    }
                },
                None => { /* ignore it if no HIR id */ },
            }
        } else {
            // ignore it if not a local item
        }
    }
}

impl UnneededPubCrate {
    /// link the fate of the constructors of data to the type definining them.
    fn notice_variant_data<'tcx>(&mut self, vd: &hir::VariantData<'tcx>, span: Span, parent_id: hir::HirId) {
        if let Some(ctor_hir_id) = vd.ctor_hir_id() {
            self.watched_item_map.insert(
                ctor_hir_id,
                WatchedItem {
                    enclosing_module: *self.current_module_path.last().unwrap(),
                    status: Unreferenced,
                    span: span,
                    linked_fate: vec![parent_id],
                },
            );
            self.watched_item_map
                .get_mut(&parent_id)
                .map(|item| item.linked_fate.push(ctor_hir_id));
        }
        for field in vd.fields() {
            if matches!(field.vis.node, hir::VisibilityKind::Crate{..}) {
                self.watched_item_map.insert(
                    field.hir_id,
                    WatchedItem {
                        enclosing_module: *self.current_module_path.last().unwrap(),
                        status: Unreferenced,
                        span: field.vis.span,
                        linked_fate: vec![], // should the fields be linked as well?
                    },
                );
            }
        }
    }
}
impl<'tcx> LateLintPass<'tcx> for UnneededPubCrate {
    fn check_mod(&mut self, _cx: &LateContext<'tcx>, _mod: &'tcx hir::Mod<'tcx>, _span: Span, hir_id: hir::HirId) {
        self.current_module_path.push(hir_id);
    }

    fn check_mod_post(&mut self, _cx: &LateContext<'tcx>, _mod: &hir::Mod<'tcx>, _span: Span, hir_id: hir::HirId) {
        assert_eq!(
            self.current_module_path
                .pop()
                .expect("mismatched push/pop in UnneededPubCrate::check_mod_post"),
            hir_id
        );
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &hir::Item<'tcx>) {
        if matches!(item.vis.node, hir::VisibilityKind::Crate { .. }) && !cx.access_levels.is_exported(item.hir_id) {
            self.watched_item_map.insert(
                item.hir_id,
                WatchedItem {
                    enclosing_module: *self.current_module_path.last().unwrap(),
                    status: Unreferenced,
                    span: item.vis.span,
                    linked_fate: vec![],
                },
            );
            match &item.kind {
                hir::ItemKind::Union(vd, _generics) | hir::ItemKind::Struct(vd, _generics) => {
                    self.notice_variant_data(vd, item.vis.span, item.hir_id);
                },
                hir::ItemKind::Enum(enum_def, _generics) => {
                    for variant in enum_def.variants {
                        self.notice_variant_data(&variant.data, item.vis.span, item.hir_id);
                    }
                },
                _ => {},
            }
            return;
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &hir::ImplItem<'tcx>) {
        if matches!(item.vis.node, hir::VisibilityKind::Crate { .. }) && !cx.access_levels.is_exported(item.hir_id) {
            self.watched_item_map.insert(
                item.hir_id,
                WatchedItem {
                    enclosing_module: *self.current_module_path.last().unwrap(),
                    status: Unreferenced,
                    span: item.vis.span,
                    linked_fate: vec![],
                },
            );
        }
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>, crate_: &'tcx hir::Crate<'tcx>) {
        // ok, now that we have scanned the entire crate for things with
        // visibility and filled the watched item map, let's scan it again for
        // any uses of those items.
        let watched_item_map = std::mem::replace(&mut self.watched_item_map, FxHashMap::default());
        let mut use_scanner = UseScanner {
            tcx: cx.tcx,
            maybe_typeck_results: cx.maybe_typeck_results(),
            watched_item_map,
            current_module_path: vec![CRATE_HIR_ID],
        };

        intravisit::walk_crate(&mut use_scanner, crate_);

        for (_watched_id, watched_item) in use_scanner.watched_item_map {
            if let LocalReference = watched_item.status {
                span_lint_and_then(
                    cx,
                    UNNEEDED_PUB_CRATE,
                    watched_item.span,
                    "pub(crate) item is never used outside of its defining module",
                    |diag| {
                        diag.span_suggestion(
                            watched_item.span,
                            "consider removing pub(crate)",
                            String::new(),
                            Applicability::MachineApplicable,
                        );
                    },
                );
            }
        }
    }
}
