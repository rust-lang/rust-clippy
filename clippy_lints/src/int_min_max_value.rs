use crate::utils::{meets_msrv, snippet_with_applicability, span_lint_and_help, span_lint_and_sugg};
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Item, ItemKind, QPath, UseKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{lint::in_external_macro, ty};
use rustc_semver::RustcVersion;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::symbol::{sym, Symbol};
use std::borrow::Cow;

declare_clippy_lint! {
    /// **What it does:** Checks for uses of `min_value()` and `max_value()` functions of the
    /// primitive integer types, and the `MIN` and `MAX` constants in their respective modules.
    ///
    /// **Why is this bad?** Both functions and the module constants are soft-deprecated with the
    /// use of the `MIN` and `MAX` constants recommended instead.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// let min = i32::min_value();
    /// let max = i32::max_value();
    /// let min = std::i32::MIN;
    /// let min = std::i32::MAX;
    /// ```
    /// Use instead:
    /// ```rust
    /// let min = i32::MIN;
    /// let max = i32::MAX;
    /// let min = i32::MIN;
    /// let max = i32::MAX;
    /// ```
    pub INT_MIN_MAX_VALUE,
    style,
    "use of `min_value()` and `max_value()` for primitive integer types"
}

impl_lint_pass!(IntMinMaxValue => [INT_MIN_MAX_VALUE]);

const INT_MIN_MAX_VALUE_MSRV: RustcVersion = RustcVersion::new(1, 43, 0);

pub struct IntMinMaxValue {
    msrv: Option<RustcVersion>,
}
impl IntMinMaxValue {
    #[must_use]
    pub fn new(msrv: Option<RustcVersion>) -> Self {
        Self { msrv }
    }
}

impl LateLintPass<'_> for IntMinMaxValue {
    extract_msrv_attr!(LateContext);

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'_>) {
        if_chain! {
            if meets_msrv(self.msrv.as_ref(), &INT_MIN_MAX_VALUE_MSRV);
            if !in_external_macro(cx.sess(), item.span);
            if let ItemKind::Use(path, UseKind::Single) = item.kind;
            if let [crate_name, mod_name] = path.segments;
            then {
                if !(crate_name.ident.as_str() == "std" || crate_name.ident.as_str() == "core") {
                    return;
                }

                let mod_name = mod_name.ident.as_str();
                if mod_name == "i8" || mod_name == "i16"
                    || mod_name == "i32" || mod_name == "i64"
                    || mod_name == "i128" || mod_name == "isize"
                    || mod_name == "u8" || mod_name == "u16"
                    || mod_name == "u32" || mod_name == "u64"
                    || mod_name == "u128" || mod_name == "usize"
                {
                    span_lint_and_help(
                        cx,
                        INT_MIN_MAX_VALUE,
                        item.span,
                        "use of the module constants `MIN` and `MAX` for primitive integer types is soft-deprecated",
                        None,
                        "remove this import",
                    );
                }
            }
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if !meets_msrv(self.msrv.as_ref(), &INT_MIN_MAX_VALUE_MSRV) || in_external_macro(cx.sess(), expr.span) {
            return;
        }

        match expr.kind {
            ExprKind::Call(func, []) => if_chain! {
                if let ExprKind::Path(QPath::TypeRelative(ty, name)) = func.kind;
                let res_ty = cx.typeck_results().node_type(ty.hir_id);
                if let ty::Int(_) | ty::Uint(_) = res_ty.kind();
                then {
                    let (msg, new_name) = if name.ident.as_str() == "max_value" {
                        ("`max_value` is soft-deprecated", "MAX")
                    } else if name.ident.as_str() == "min_value" {
                        ("`min_value` is soft-deprecated", "MIN")
                    } else {
                        return;
                    };


                    let mut app = Applicability::MachineApplicable;
                    let sugg_path = match snippet_with_applicability(cx, ty.span, "_", &mut app) {
                        // the span for the type includes the method name for some reason, strip it off
                        Cow::Owned(x) => {
                            Cow::Owned(x.rsplitn(2, "::").nth(1).unwrap_or("_").into())
                        }
                        Cow::Borrowed(x) => Cow::Borrowed(x),
                    };
                    span_lint_and_sugg(
                        cx,
                        INT_MIN_MAX_VALUE,
                        expr.span,
                        msg,
                        "use constant instead",
                        format!("{}::{}", sugg_path, new_name),
                        app,
                    );
                }
            },
            ExprKind::Path(QPath::Resolved(None, path)) => if_chain! {
                if let [crate_name, _, _] = path.segments;
                if crate_name.ident.name == sym::std || crate_name.ident.name == sym::core;
                if let Some(id) = path.res.opt_def_id();
                if let Some((mod_name, constant_name)) = get_prim_module_constant(&cx.get_def_path(id));
                if path.segments[1].ident.as_str() == mod_name && path.segments[2].ident.as_str() == constant_name;
                then {
                    span_lint_and_sugg(
                        cx,
                        INT_MIN_MAX_VALUE,
                        expr.span,
                        &format!("`{}::{}::{}` is soft-deprecated", crate_name.ident.as_str(), mod_name, constant_name),
                        "use associated constant instead",
                        format!("{}::{}", mod_name, constant_name),
                        Applicability::MachineApplicable,
                    );
                }
            },
            _ => (),
        }
    }
}

fn get_prim_module_constant(path: &[Symbol]) -> Option<(&'static str, &'static str)> {
    if path.len() != 3 {
        return None;
    }
    if !(path[0] == sym::core || path[0] == sym::std) {
        return None;
    }
    let mod_name = match path[1] {
        sym::u8 => "u8",
        sym::u16 => "u16",
        sym::u32 => "u32",
        sym::u64 => "u64",
        sym::u128 => "u128",
        sym::usize => "usize",
        sym::i8 => "i8",
        sym::i16 => "i16",
        sym::i32 => "i32",
        sym::i64 => "i64",
        sym::i128 => "i128",
        sym::isize => "isize",
        _ => return None,
    };
    let constant_name = if path[2].as_str() == "MIN" {
        "MIN"
    } else if path[2].as_str() == "MAX" {
        "MAX"
    } else {
        return None;
    };
    Some((mod_name, constant_name))
}
