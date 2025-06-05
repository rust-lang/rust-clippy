use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::sym;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks that `use` statements come before all other items (functions, structs, constants, etc.)
    /// at the module level, exceptions beinng `mod` and `extern crate` statements before `use` (confgiurable).
    /// Ignores all use statements in cfg blocks as it's a common pattern.
    ///
    /// ### Why is this bad?
    /// Having `use` statements scattered throughout a module makes it harder to see all imports
    /// at a glance. Keeping imports grouped near the top (with `mod` and `extern crate`)
    /// improves code organization and readability.
    ///
    /// ### Example
    /// ```no_run
    /// mod my_module;
    /// extern crate some_crate;
    ///
    /// fn foo() {}
    /// use std::collections::HashMap;
    ///
    /// const VALUE: i32 = 32;
    /// use std::vec::Vec;
    /// ```
    /// Use instead:
    /// ```no_run
    /// mod my_module;
    /// extern crate some_crate;
    /// use std::collections::HashMap;
    /// use std::vec::Vec;
    ///
    /// fn foo() {}
    /// const VALUE: i32 = 32;
    /// ```
    #[clippy::version = "1.89.0"]
    pub ITEMS_BEFORE_USE,
    style,
    "checks if module level `use` statements precede all other items"
}

// check function to ignore cfg blocks (allowed in both stylistic and pedantic levels)
fn is_cfg(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    let mut def = item.owner_id.def_id;

    loop {
        let attrs = cx.tcx.hir_attrs(item.hir_id());
        if attrs.iter().any(|attr| {
            attr.has_any_name(&[
                sym::cfg,
                sym::cfg_attr,
                sym::cfg_eval,
                sym::cfg_hide,
                sym::cfg_panic,
                sym::cfg_trace,
                sym::cfg_doctest,
                sym::cfg_version,
                sym::cfg_sanitize,
                sym::cfg_fmt_debug,
                sym::cfg_ub_checks,
                sym::cfg_accessible,
                sym::cfg_attr_multi,
                sym::cfg_attr_trace,
                sym::cfg_target_abi,
                sym::cfg_sanitizer_cfi,
                sym::cfg_target_vendor,
                sym::cfg_target_compact,
                sym::cfg_target_feature,
                sym::cfg_contract_checks,
                sym::cfg_overflow_checks,
                sym::cfg_boolean_literals,
                sym::cfg_relocation_model,
                sym::cfg_target_has_atomic,
                sym::cfg_emscripten_wasm_eh,
                sym::cfg_target_thread_local,
                sym::cfg_target_has_reliable_f16_f128,
                sym::cfg_target_has_atomic_equal_alignment,
                sym::doc_cfg,
                sym::doc_cfg_hide,
                sym::doc_auto_cfg,
                sym::link_cfg,
            ])
        }) {
            return true;
        }
        match cx.tcx.opt_parent(def.to_def_id()) {
            Some(parent) => def = parent.expect_local(),
            None => break false,
        }
    }
}

pub struct ItemsBeforeUse {
    pub strict_order_of_use: bool,
}

impl ItemsBeforeUse {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            strict_order_of_use: conf.strict_order_of_use,
        }
    }
}

impl_lint_pass!(ItemsBeforeUse => [ITEMS_BEFORE_USE]);

impl<'tcx> LateLintPass<'tcx> for ItemsBeforeUse {
    fn check_mod(&mut self, cx: &LateContext<'tcx>, module: &'tcx Mod<'tcx>, _hir_id: HirId) {
        let mut saw_non_use = false;
        let mut saw_mod_or_extern = false;
        for item_ids in module.item_ids {
            let item = cx.tcx.hir_item(*item_ids);

            if is_cfg(cx, item) {
                continue;
            }

            match item.kind {
                ItemKind::Use(..) => {
                    // strict mode (pedantic) will lint for mod and extern crare too
                    if (saw_mod_or_extern || saw_non_use) && self.strict_order_of_use {
                        span_lint_and_note(
                            cx,
                            ITEMS_BEFORE_USE,
                            item.span,
                            "strict_order_of_use enabled: use statements should precede all other items including mod and extern crate statements",
                            None,
                            "Move this statement to the top of the module",
                        );
                    } else if saw_non_use {
                        // stylistic (on by default) will only lint for non-mod/extern items
                        span_lint_and_note(
                            cx,
                            ITEMS_BEFORE_USE,
                            item.span,
                            "module level use statements should precede all other items",
                            None,
                            "consider moving this statement to the top of the module",
                        );
                    }
                },
                ItemKind::Mod(..) | ItemKind::ExternCrate(..) => saw_mod_or_extern = true,
                _ => saw_non_use = true,
            }
        }
    }
}
