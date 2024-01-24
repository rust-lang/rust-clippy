use clippy_config::types::DisallowedPath;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{fn_def_id, get_parent_expr, path_def_id};
use itertools::Itertools as _;
use rustc_data_structures::unord::UnordMap;
use rustc_hir::def::Res;
use rustc_hir::def_id::{DefId, DefIdMap};
use rustc_hir::{Expr, ExprKind, PrimTy};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, AdtKind};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Denies the configured methods and functions in clippy.toml
    ///
    /// Note: Even though this lint is warn-by-default, it will only trigger if
    /// methods are defined in the clippy.toml file.
    ///
    /// ### Why is this bad?
    /// Some methods are undesirable in certain contexts, and it's beneficial to
    /// lint for them as needed.
    ///
    /// ### Example
    /// An example clippy.toml configuration:
    /// ```toml
    /// # clippy.toml
    /// disallowed-methods = [
    ///     # Can use a string as the path of the disallowed method.
    ///     "std::boxed::Box::new",
    ///     # Can also use an inline table with a `path` key.
    ///     { path = "std::time::Instant::now" },
    ///     # When using an inline table, can add a `reason` for why the method
    ///     # is disallowed.
    ///     { path = "std::vec::Vec::leak", reason = "no leaking memory" },
    /// ]
    /// ```
    ///
    /// ```rust,ignore
    /// // Example code where clippy issues a warning
    /// let xs = vec![1, 2, 3, 4];
    /// xs.leak(); // Vec::leak is disallowed in the config.
    /// // The diagnostic contains the message "no leaking memory".
    ///
    /// let _now = Instant::now(); // Instant::now is disallowed in the config.
    ///
    /// let _box = Box::new(3); // Box::new is disallowed in the config.
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// // Example code which does not raise clippy warning
    /// let mut xs = Vec::new(); // Vec::new is _not_ disallowed in the config.
    /// xs.push(123); // Vec::push is _not_ disallowed in the config.
    /// ```
    #[clippy::version = "1.49.0"]
    pub DISALLOWED_METHODS,
    style,
    "use of a disallowed method call"
}

#[derive(Clone, Debug)]
pub struct DisallowedMethods {
    conf_disallowed: Vec<DisallowedPath>,
    disallowed: DefIdMap<usize>,
    // (Self, TraitMethod)
    disallowed_qualified_trait: UnordMap<(Res, DefId), usize>,
}

impl DisallowedMethods {
    pub fn new(conf_disallowed: Vec<DisallowedPath>) -> Self {
        Self {
            conf_disallowed,
            disallowed: DefIdMap::default(),
            disallowed_qualified_trait: UnordMap::default(),
        }
    }
}

impl_lint_pass!(DisallowedMethods => [DISALLOWED_METHODS]);

impl<'tcx> LateLintPass<'tcx> for DisallowedMethods {
    fn check_crate(&mut self, cx: &LateContext<'_>) {
        for (index, conf) in self.conf_disallowed.iter().enumerate() {
            let path = conf.path();
            if let Some(path) = path.strip_prefix('<') {
                // a qualified associated item
                let Some((tr, method)) = path.split_once(">::") else {
                    continue;
                };
                let Some((self_ty, _as, trait_path)) = tr.split_whitespace().next_tuple() else {
                    continue;
                };
                let self_segs: Vec<_> = self_ty.split("::").collect();
                let self_ress: Vec<_> = clippy_utils::def_path_res(cx, &self_segs);
                let mut method_segs: Vec<_> = trait_path.split("::").collect();
                method_segs.push(method);
                let method_id: Vec<_> = clippy_utils::def_path_def_ids(cx, &method_segs).collect();
                for self_res in &self_ress {
                    for method_id in &method_id {
                        self.disallowed_qualified_trait.insert((*self_res, *method_id), index);
                    }
                }
            } else {
                // simple path
                let segs: Vec<_> = path.split("::").collect();
                for id in clippy_utils::def_path_def_ids(cx, &segs) {
                    self.disallowed.insert(id, index);
                }
            }
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let uncalled_path = if let Some(parent) = get_parent_expr(cx, expr)
            && let ExprKind::Call(receiver, _) = parent.kind
            && receiver.hir_id == expr.hir_id
        {
            None
        } else {
            path_def_id(cx, expr)
        };
        let Some(def_id) = uncalled_path.or_else(|| fn_def_id(cx, expr)) else {
            return;
        };
        let conf = match self.disallowed.get(&def_id) {
            Some(&index) => &self.conf_disallowed[index],
            None => match expr.kind {
                ExprKind::MethodCall(_, self_arg, _, _) if !self.disallowed_qualified_trait.is_empty() => {
                    let typeck = cx.typeck_results();
                    let trait_method_def_id = typeck.type_dependent_def_id(expr.hir_id).unwrap();
                    let self_ty = typeck.expr_ty(self_arg);
                    let self_res: Res<rustc_hir::HirId> = match self_ty.kind() {
                        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => {
                            Res::PrimTy(PrimTy::from_name(self_ty.primitive_symbol().unwrap()).unwrap())
                        },
                        ty::Str => Res::PrimTy(PrimTy::Str),
                        ty::Adt(adt, _) => Res::Def(
                            match adt.adt_kind() {
                                AdtKind::Struct => rustc_hir::def::DefKind::Struct,
                                AdtKind::Union => rustc_hir::def::DefKind::Union,
                                AdtKind::Enum => rustc_hir::def::DefKind::Enum,
                            },
                            adt.did(),
                        ),
                        // FIXME: these other kinds are not currently supported by disallowed_methods due to how
                        // def_path_ref is implemented
                        _ => return,
                    };
                    match self.disallowed_qualified_trait.get(&(self_res, trait_method_def_id)) {
                        Some(&index) => &self.conf_disallowed[index],
                        None => return,
                    }
                },
                _ => return,
            },
        };
        let msg = format!("use of a disallowed method `{}`", conf.path());
        span_lint_and_then(cx, DISALLOWED_METHODS, expr.span, &msg, |diag| {
            if let Some(reason) = conf.reason() {
                diag.note(reason);
            }
        });
    }
}
