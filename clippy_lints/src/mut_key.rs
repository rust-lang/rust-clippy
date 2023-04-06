use clippy_utils::diagnostics::span_lint;
use clippy_utils::ty::is_interior_mut_ty;
use clippy_utils::{def_path_def_ids, trait_ref_of_method};
use rustc_data_structures::fx::FxHashSet;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::query::Key;
use rustc_middle::ty::{Adt, Ty};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::def_id::LocalDefId;
use rustc_span::source_map::Span;
use rustc_span::symbol::sym;
use std::iter;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for sets/maps with mutable key types.
    ///
    /// ### Why is this bad?
    /// All of `HashMap`, `HashSet`, `BTreeMap` and
    /// `BtreeSet` rely on either the hash or the order of keys be unchanging,
    /// so having types with interior mutability is a bad idea.
    ///
    /// ### Known problems
    ///
    /// #### False Positives
    /// It's correct to use a struct that contains interior mutability as a key, when its
    /// implementation of `Hash` or `Ord` doesn't access any of the interior mutable types.
    /// However, this lint is unable to recognize this, so it will often cause false positives in
    /// theses cases.  The `bytes` crate is a great example of this.
    ///
    /// #### False Negatives
    /// For custom `struct`s/`enum`s, this lint is unable to check for interior mutability behind
    /// indirection.  For example, `struct BadKey<'a>(&'a Cell<usize>)` will be seen as immutable
    /// and cause a false negative if its implementation of `Hash`/`Ord` accesses the `Cell`.
    ///
    /// This lint does check a few cases for indirection.  Firstly, using some standard library
    /// types (`Option`, `Result`, `Box`, `Rc`, `Arc`, `Vec`, `VecDeque`, `BTreeMap` and
    /// `BTreeSet`) directly as keys (e.g. in `HashMap<Box<Cell<usize>>, ()>`) **will** trigger the
    /// lint, because the impls of `Hash`/`Ord` for these types directly call `Hash`/`Ord` on their
    /// contained type.
    ///
    /// Secondly, the implementations of `Hash` and `Ord` for raw pointers (`*const T` or `*mut T`)
    /// apply only to the **address** of the contained value.  Therefore, interior mutability
    /// behind raw pointers (e.g. in `HashSet<*mut Cell<usize>>`) can't impact the value of `Hash`
    /// or `Ord`, and therefore will not trigger this link.  For more info, see issue
    /// [#6745](https://github.com/rust-lang/rust-clippy/issues/6745).
    ///
    /// ### Example
    /// ```rust
    /// use std::cmp::{PartialEq, Eq};
    /// use std::collections::HashSet;
    /// use std::hash::{Hash, Hasher};
    /// use std::sync::atomic::AtomicUsize;
    ///# #[allow(unused)]
    ///
    /// struct Bad(AtomicUsize);
    /// impl PartialEq for Bad {
    ///     fn eq(&self, rhs: &Self) -> bool {
    ///          ..
    /// ; unimplemented!();
    ///     }
    /// }
    ///
    /// impl Eq for Bad {}
    ///
    /// impl Hash for Bad {
    ///     fn hash<H: Hasher>(&self, h: &mut H) {
    ///         ..
    /// ; unimplemented!();
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let _: HashSet<Bad> = HashSet::new();
    /// }
    /// ```
    #[clippy::version = "1.42.0"]
    pub MUTABLE_KEY_TYPE,
    suspicious,
    "Check for mutable `Map`/`Set` key type"
}

#[derive(Clone)]
pub struct MutableKeyType {
    ignore_interior_mutability: Vec<String>,
    ignore_mut_def_ids: FxHashSet<hir::def_id::DefId>,
}

impl_lint_pass!(MutableKeyType => [ MUTABLE_KEY_TYPE ]);

impl<'tcx> LateLintPass<'tcx> for MutableKeyType {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.ignore_mut_def_ids.clear();
        let mut path = Vec::new();
        for ty in &self.ignore_interior_mutability {
            path.extend(ty.split("::"));
            for id in def_path_def_ids(cx, &path[..]) {
                self.ignore_mut_def_ids.insert(id);
            }
            path.clear();
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'tcx>) {
        if let hir::ItemKind::Fn(ref sig, ..) = item.kind {
            self.check_sig(cx, item.owner_id.def_id, sig.decl);
        }
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::ImplItem<'tcx>) {
        if let hir::ImplItemKind::Fn(ref sig, ..) = item.kind {
            if trait_ref_of_method(cx, item.owner_id.def_id).is_none() {
                self.check_sig(cx, item.owner_id.def_id, sig.decl);
            }
        }
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::TraitItem<'tcx>) {
        if let hir::TraitItemKind::Fn(ref sig, ..) = item.kind {
            self.check_sig(cx, item.owner_id.def_id, sig.decl);
        }
    }

    fn check_local(&mut self, cx: &LateContext<'_>, local: &hir::Local<'_>) {
        if let hir::PatKind::Wild = local.pat.kind {
            return;
        }
        self.check_ty_(cx, local.span, cx.typeck_results().pat_ty(local.pat));
    }
}

impl MutableKeyType {
    pub fn new(ignore_interior_mutability: Vec<String>) -> Self {
        Self {
            ignore_interior_mutability,
            ignore_mut_def_ids: FxHashSet::default(),
        }
    }

    fn check_sig(&self, cx: &LateContext<'_>, fn_def_id: LocalDefId, decl: &hir::FnDecl<'_>) {
        let fn_sig = cx.tcx.fn_sig(fn_def_id).subst_identity();
        for (hir_ty, ty) in iter::zip(decl.inputs, fn_sig.inputs().skip_binder()) {
            self.check_ty_(cx, hir_ty.span, *ty);
        }
        self.check_ty_(cx, decl.output.span(), cx.tcx.erase_late_bound_regions(fn_sig.output()));
    }

    // We want to lint 1. sets or maps with 2. not immutable key types and 3. no unerased
    // generics (because the compiler cannot ensure immutability for unknown types).
    fn check_ty_<'tcx>(&self, cx: &LateContext<'tcx>, span: Span, ty: Ty<'tcx>) {
        let ty = ty.peel_refs();
        if let Adt(def, substs) = ty.kind() {
            let is_keyed_type = [sym::HashMap, sym::BTreeMap, sym::HashSet, sym::BTreeSet]
                .iter()
                .any(|diag_item| cx.tcx.is_diagnostic_item(*diag_item, def.did()));
            if !is_keyed_type {
                return;
            }

            let subst_ty = substs.type_at(0);
            // Determines if a type contains interior mutability which would affect its implementation of
            // [`Hash`] or [`Ord`].
            if is_interior_mut_ty(cx, subst_ty)
                && !matches!(subst_ty.ty_adt_id(), Some(adt_id) if self.ignore_mut_def_ids.contains(&adt_id))
            {
                span_lint(cx, MUTABLE_KEY_TYPE, span, "mutable key type");
            }
        }
    }
}
