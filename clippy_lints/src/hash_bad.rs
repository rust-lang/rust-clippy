use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::sym;
use clippy_utils::ty::implements_trait;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_middle::ty;
use rustc_span::def_id::DefId;

declare_clippy_lint! {
    /// ### What it does
    /// This lint prevents using HashMap/HashSet usage with keys/values respectively
    /// that do not implement `Eq + Hash` traits
    /// and BTreeMap/BTreeSet usage with keys/values respectively
    /// that do not implement `Ord` trait
    /// ### Why is this bad?
    /// Although the compiler allows constructing Map/Set with any type as a key/value respectively
    /// such semantics make no sense, since you are unable to perform basic operations on a Map/Set
    /// like `insert()`, `add()`, `remove()`, `get()`, `contains()`, `first()`, e.t.c, if mentioned conditions are not satisfied
    /// ### Example
    /// ```no_run
    /// let _: HashMap<BadType, ()> = HashMap::new();
    /// // warning, because BadType does not implement `Hash + Eq` traits
    ///
    /// let _: BTreeMap<BadType, ()> = BTreeMap::new();
    /// // warning, because BadType does not implement `Ord` trait
    /// ```
    #[clippy::version = "1.101.0"]
    pub HASH_BAD,
    suspicious,
    "something something"
}

declare_lint_pass!(HashBad => [HASH_BAD]);

impl<'tcx> LateLintPass<'tcx> for HashBad {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, exp: &'tcx Expr<'tcx>) {
        let expr_ty = cx.typeck_results().expr_ty(exp);

        if let ty::Adt(adt_def, args) = expr_ty.kind()
            && let Some((struct_type, container_type)) = extract_adt(cx, adt_def.did())
            && let Some(first_ty) = args.first().and_then(|it| it.as_type())
        {
            match struct_type {
                StructType::Hash => {
                    if let Some(hash_did) = cx.tcx.get_diagnostic_item(sym::Hash)
                        && let Some(eq_did) = cx.tcx.get_diagnostic_item(sym::Eq)
                        && !(implements_trait(cx, first_ty, eq_did, &[])
                            && implements_trait(cx, first_ty, hash_did, &[]))
                    {
                    } else {
                        return;
                    }
                },
                StructType::Tree => {
                    if let Some(ord_did) = cx.tcx.get_diagnostic_item(sym::Ord)
                        && !implements_trait(cx, first_ty, ord_did, &[])
                    {
                    } else {
                        return;
                    }
                },
            }

            let type_name = format_args!("{}{}", struct_type.type_prefix(), container_type.type_name());
            let msg = format!(
                r#"A {} of type `{type_name}` must implement `{}`"#,
                container_type.target_name(),
                struct_type.requiered_trait()
            );
            let help = format!(
                r#"Although it is possible to use any type as a {} for {},
it is recommended for it to implement {}, otherwise most of methods for the type will be unavailable"#,
                container_type.target_name(),
                type_name,
                struct_type.requiered_trait(),
            );
            span_lint_and_help(cx, HASH_BAD, exp.span, msg, None, help);
        }
    }
}

fn extract_adt(cx: &LateContext<'_>, def_id: DefId) -> Option<(StructType, ContainerType)> {
    if cx.tcx.is_diagnostic_item(sym::HashMap, def_id) {
        Some((StructType::Hash, ContainerType::Map))
    } else if cx.tcx.is_diagnostic_item(sym::HashSet, def_id) {
        Some((StructType::Hash, ContainerType::Set))
    } else if cx.tcx.is_diagnostic_item(sym::BTreeMap, def_id) {
        Some((StructType::Tree, ContainerType::Map))
    } else if cx.tcx.is_diagnostic_item(sym::BTreeSet, def_id) {
        Some((StructType::Tree, ContainerType::Set))
    } else {
        None
    }
}

enum StructType {
    Tree,
    Hash,
}
impl StructType {
    fn requiered_trait(&self) -> &str {
        match self {
            StructType::Tree => "Ord",
            StructType::Hash => "Eq + Hash",
        }
    }
    fn type_prefix(&self) -> &str {
        match self {
            StructType::Tree => "BTree",
            StructType::Hash => "Hash",
        }
    }
}

enum ContainerType {
    Map,
    Set,
}
impl ContainerType {
    fn target_name(&self) -> &str {
        match self {
            ContainerType::Map => "key",
            ContainerType::Set => "value",
        }
    }
    fn type_name(&self) -> &str {
        match self {
            ContainerType::Map => "Map",
            ContainerType::Set => "Set",
        }
    }
}
