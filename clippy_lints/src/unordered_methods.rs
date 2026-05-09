use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::res::MaybeDef;
use clippy_utils::return_ty;
use rustc_hir::{FnSig, ImplItemKind, Item, ItemKind, OwnerId};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Adt, Ty, Visibility};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::DefId;
use rustc_span::sym;
use std::fmt::Formatter;

declare_clippy_lint! {
    /// ### What it does
    /// Checks that methods within an `impl` block are ordered consistently:
    /// constructors (`pub fn new() -> Self`) first, then public methods,
    /// and then private methods.
    ///
    /// ### Why is this bad?
    /// Following a consistent order for methods within an `impl` block improves readability
    /// and maintainability. Constructors are often the entry point for creating instances
    /// of a struct, so placing them first makes the API clearer. A logical grouping
    /// of methods by visibility further enhances code comprehension.
    ///
    /// ### Example
    /// ```no_run
    /// struct MyStruct;
    ///
    /// impl MyStruct {
    ///     fn do_something_private(&self) {} // Bad: Private method before constructor
    ///
    ///     pub fn new() -> Self {
    ///         MyStruct
    ///     }
    ///
    ///     pub(crate) fn do_something_crate_private(&self) {} // Bad: Crate method before public
    ///
    ///     pub fn do_something_public(&self) {}
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct MyStruct;
    ///
    /// impl MyStruct {
    ///     pub fn new() -> Self {
    ///         MyStruct
    ///     }
    ///
    ///     pub fn do_something_public(&self) {}
    ///
    ///     pub(crate) fn do_something_crate_private(&self) {}
    ///
    ///     fn do_something_private(&self) {}
    /// }
    /// ```
    #[clippy::version = "1.97.0"]
    pub UNORDERED_METHODS,
    pedantic,
    "Linter that checks that the constructor is declared before the struct's methods, \
    and that the public functions are also declared before the crate and private ones"
}

impl_lint_pass!(UnorderedMethods => [UNORDERED_METHODS]);

#[derive(Clone)]
pub struct UnorderedMethods;

/// Defines the desired order of method categories.
/// Higher enum variants mean higher priority (should appear earlier).
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
enum MethodKind {
    Constructor,
    Public,
    Private,
}

struct MethodMetadata {
    kind: MethodKind,
    name: String,
}

impl MethodMetadata {
    fn from<'tcx>(
        fn_sig: &FnSig<'_>,
        cx: &LateContext<'tcx>,
        impl_item_owner_id: OwnerId,
        self_ty: Ty<'tcx>,
        fn_name: &str,
        visibility: Visibility<DefId>,
    ) -> Option<Self> {
        if is_constructor(fn_sig, cx, impl_item_owner_id, self_ty) {
            Some(Self {
                kind: MethodKind::Constructor,
                name: fn_name.to_string(),
            })
        } else if input_has_self(fn_sig) && visibility.is_public() {
            Some(Self {
                kind: MethodKind::Public,
                name: fn_name.to_string(),
            })
        } else if input_has_self(fn_sig) && !visibility.is_public() {
            Some(Self {
                kind: MethodKind::Private,
                name: fn_name.to_string(),
            })
        } else {
            None
        }
    }
}

impl std::fmt::Display for MethodMetadata {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}({})", self.kind, self.name)
    }
}

impl LateLintPass<'_> for UnorderedMethods {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Impl(impl_block) = item.kind {
            // Only check inherent impl blocks (not trait implementations)
            if impl_block.of_trait.is_some() {
                return;
            }

            // Get the `Self` type for the `impl` block
            let self_ty = cx.tcx.type_of(item.owner_id).instantiate_identity().skip_norm_wip();

            let mut last_category: Option<MethodMetadata> = None;

            for &impl_item_id in impl_block.items {
                let impl_item = cx.tcx.hir_impl_item(impl_item_id);

                // Only consider functions for ordering
                if let ImplItemKind::Fn(ref sig, _) = impl_item.kind {
                    let fn_name = impl_item.ident.name.as_str();

                    let visibility = cx.tcx.visibility(impl_item.owner_id.def_id);

                    let Some(current_category) =
                        MethodMetadata::from(sig, cx, impl_item.owner_id, self_ty, fn_name, visibility)
                    else {
                        continue;
                    };

                    if let Some(lc) = last_category
                        // If the current method's category is "less than" (lower priority)
                        // the last encountered category, then it's out of order.
                        && current_category.kind < lc.kind
                    {
                        let msg =
                            format!("methods are not ordered correctly: `{current_category}` declared after `{lc}`");
                        span_lint_and_help(
                            cx,
                            UNORDERED_METHODS,
                            impl_item.span,
                            msg,
                            None,
                            "reorder functions to be constructors, public, private",
                        );
                    }
                    last_category = Some(current_category);
                }
            }
        }
    }
}

/// `is_constructor` check whether the given function is a constructor.
/// A constructor must:
/// - Not have an implicit `self` parameter (&self or &mut self)
/// - Return Self, the struct type, or Option<Self>/Option<struct type>
fn is_constructor<'tcx>(
    fn_sig: &FnSig<'_>,
    cx: &LateContext<'tcx>,
    impl_item_owner_id: OwnerId,
    self_ty: Ty<'tcx>,
) -> bool {
    // Check that the function doesn't take &self or &mut self
    if fn_sig.decl.implicit_self().has_implicit_self() {
        return false;
    }

    // Check if the return type matches the struct type
    let ret_ty = return_ty(cx, impl_item_owner_id);

    // Direct match: returns Self or the struct type
    if ret_ty == self_ty {
        return true;
    }

    // Check for Option<Self> or Option<struct type>
    if ret_ty.is_diag_item(cx, sym::Option)
        && let Adt(_, args) = ret_ty.kind()
        && let Some(inner_ty) = args.first().and_then(|arg| arg.as_type())
        && inner_ty == self_ty
    {
        return true;
    }

    false
}

/// `input_has_self` checks whether the input parameters of a function contain the `self`
/// parameter in any form.
fn input_has_self(fn_sig: &FnSig<'_>) -> bool {
    fn_sig.decl.implicit_self().has_implicit_self()
}
