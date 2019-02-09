use crate::utils::span_lint;
use rustc::hir::*;
use rustc::lint::{LateContext, LateLintPass, LintArray, LintPass};
use rustc::{declare_tool_lint, lint_array};
use rustc::util::nodemap::NodeSet;

macro_rules! missing_impl {
    ($trait:ident, $trait_path:path, $lint_constant:ident, $struct_name:ident,
        $trait_check:ident) => (
        declare_clippy_lint! {
            pub $lint_constant,
            correctness,
            concat!("detects missing implementations of ", stringify!($trait_path))
        }

        #[derive(Default)]
        pub struct $struct_name {
            impling_types: Option<NodeSet>,
        }

        impl $struct_name {
            pub fn new() -> $struct_name {
                $struct_name { impling_types: None }
            }
        }

        impl LintPass for $struct_name {
            fn name(&self) -> &'static str {
                stringify!($struct_name)
            }

            fn get_lints(&self) -> LintArray {
                lint_array!($lint_constant)
            }
        }

        impl<'a, 'tcx> LateLintPass<'a, 'tcx> for $struct_name {
            fn check_item(&mut self, cx: &LateContext<'a, 'tcx>, item: &'tcx Item) {
                if !cx.access_levels.is_reachable(item.id) {
                    return;
                }

                match item.node {
                    ItemKind::Struct(..) |
                    ItemKind::Union(..) |
                    ItemKind::Enum(..) => {}
                    _ => return,
                }

                let x = match cx.tcx.lang_items().$trait_check() {
                    Some(x) => x,
                    None => return,
                };

                if self.impling_types.is_none() {
                    let mut impls = NodeSet::default();
                    cx.tcx.for_each_impl(x, |d| {
                        if let Some(ty_def) = cx.tcx.type_of(d).ty_adt_def() {
                            if let Some(node_id) = cx.tcx.hir().as_local_node_id(ty_def.did) {
                                impls.insert(node_id);
                            }
                        }
                    });

                    self.impling_types = Some(impls);
                }

                if !self.impling_types.as_ref().unwrap().contains(&item.id) {
                    span_lint(
                        cx,
                        $lint_constant,
                        item.span,
                        concat!("type does not implement `", stringify!($trait_path), "`; \
                        consider adding #[derive(", stringify!($trait), ")] or a manual \
                        implementation"))
                }
            }
        }
    )
}

/// **What it does:** Checks for `Copy` implementations missing from structs and enums
///
/// **Why is this bad?** `Copy` is a core trait that should be implemented for
/// all types as much as possible.
///
/// For more, see the [Rust API Guidelines](https://rust-lang-nursery.github.io/api-guidelines/interoperability.html#c-common-traits)
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
/// // Bad
/// struct Foo;
///
/// // Good
/// #[derive(Copy)]
/// struct Bar;
/// ```
missing_impl!(
    Copy,
    Copy,
    MISSING_COPY_IMPLEMENTATIONS,
    MissingCopyImplementations,
    copy_trait);

/// **What it does:** Checks for `Debug` implementations missing from structs and enums
///
/// **Why is this bad?** `Debug` is a core trait that should be implemented for
/// all types as much as possible.
///
/// For more, see the [Rust API Guidelines](https://rust-lang-nursery.github.io/api-guidelines/interoperability.html#c-common-traits)
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
/// // Bad
/// struct Foo;
///
/// // Good
/// #[derive(Debug)]
/// struct Bar;
/// ```
missing_impl!(
    Debug,
    Debug,
    MISSING_DEBUG_IMPLEMENTATIONS,
    MissingDebugImplementations,
    debug_trait);
