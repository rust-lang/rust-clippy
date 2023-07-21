use clippy_utils::{
    diagnostics::{span_lint, span_lint_and_sugg, span_lint_hir_and_then},
    get_parent_expr, in_constant, is_from_proc_macro, is_integer_literal, is_lint_allowed, iter_input_pats,
    last_path_segment,
    source::{snippet, snippet_opt, snippet_with_context},
    std_or_core,
    sugg::Sugg,
    SpanlessEq,
};
use hir::OwnerNode;
use if_chain::if_chain;
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{
    self as hir, def, BinOpKind, BindingAnnotation, Body, ByRef, Expr, ExprKind, FnDecl, ItemKind, Lit, Mutability,
    Node, PatKind, Stmt, StmtKind, TyKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::def_id::LocalDefId;
use rustc_span::hygiene::DesugaringKind;
use rustc_span::source_map::{ExpnKind, Span};
use rustc_span::BytePos;

use crate::ref_patterns::REF_PATTERNS;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for function arguments and let bindings denoted as
    /// `ref`.
    ///
    /// ### Why is this bad?
    /// The `ref` declaration makes the function take an owned
    /// value, but turns the argument into a reference (which means that the value
    /// is destroyed when exiting the function). This adds not much value: either
    /// take a reference type, or take an owned value and create references in the
    /// body.
    ///
    /// For let bindings, `let x = &foo;` is preferred over `let ref x = foo`. The
    /// type of `x` is more obvious with the former.
    ///
    /// ### Known problems
    /// If the argument is dereferenced within the function,
    /// removing the `ref` will lead to errors. This can be fixed by removing the
    /// dereferences, e.g., changing `*x` to `x` within the function.
    ///
    /// ### Example
    /// ```rust
    /// fn foo(ref _x: u8) {}
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// fn foo(_x: &u8) {}
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub TOPLEVEL_REF_ARG,
    style,
    "an entire binding declared as `ref`, in a function argument or a `let` statement"
}
declare_clippy_lint! {
    /// ### What it does
    /// Checks for the use of bindings with a single leading
    /// underscore.
    ///
    /// ### Why is this bad?
    /// A single leading underscore is usually used to indicate
    /// that a binding will not be used. Using such a binding breaks this
    /// expectation.
    ///
    /// ### Known problems
    /// The lint does not work properly with desugaring and
    /// macro, it has been allowed in the mean time.
    ///
    /// ### Example
    /// ```rust
    /// let _x = 0;
    /// let y = _x + 1; // Here we are using `_x`, even though it has a leading
    ///                 // underscore. We should rename `_x` to `x`
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub USED_UNDERSCORE_BINDING,
    pedantic,
    "using a binding which is prefixed with an underscore"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the use of short circuit boolean conditions as
    /// a
    /// statement.
    ///
    /// ### Why is this bad?
    /// Using a short circuit boolean condition as a statement
    /// may hide the fact that the second part is executed or not depending on the
    /// outcome of the first part.
    ///
    /// ### Example
    /// ```rust,ignore
    /// f() && g(); // We should write `if f() { g(); }`.
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub SHORT_CIRCUIT_STATEMENT,
    complexity,
    "using a short circuit boolean condition as a statement"
}

declare_clippy_lint! {
    /// ### What it does
    /// Catch casts from `0` to some pointer type
    ///
    /// ### Why is this bad?
    /// This generally means `null` and is better expressed as
    /// {`std`, `core`}`::ptr::`{`null`, `null_mut`}.
    ///
    /// ### Example
    /// ```rust
    /// let a = 0 as *const u32;
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// let a = std::ptr::null::<u32>();
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub ZERO_PTR,
    style,
    "using `0 as *{const, mut} T`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for numeric literals (e.g., `1.0_f64`) without a suffix (The `u32` in `1u32`).
    ///
    /// ### Why is this bad?
    /// It's not, but some projects may wish to make the type of every literal explicit. In many
    /// cases this can prevent default numeric fallback as well, see
    /// [RFC0212](https://github.com/rust-lang/rfcs/blob/master/text/0212-restore-int-fallback.md)
    /// for more info and `default_numeric_fallback` for an alternative that only tackles the
    /// latter if explicitness is not desired.
    ///
    /// Note that when type annotations are provided, the type is already explicit; the lint will
    /// not lint those cases unless the `allow_missing_suffix_with_type_annotations` configuration
    /// option is disabled.
    ///
    /// ### Example
    /// ```rust
    /// let x = 1;
    /// ```
    /// Use instead:
    /// ```rust
    /// let x = 1i32;
    /// ```
    #[clippy::version = "1.72.0"]
    pub NUMERIC_LITERAL_MISSING_SUFFIX,
    restriction,
    "numeric literals missing explicit suffixes"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for numeric literals (e.g., 1.0_f64) without a separator (`_`) before the suffix
    /// (`f64` in `1.0_f64`).
    ///
    /// ### Why is this bad?
    /// It's not, but enforcing a consistent style is important. In this case the codebase prefers
    /// a separator.
    ///
    /// Also see the `suffix_without_separator` lint for an alternative.
    ///
    /// ### Example
    /// ```rust
    /// let x = 1i32;
    /// ```
    /// Use instead:
    /// ```rust
    /// let x = 1_i32;
    /// ```
    #[clippy::version = "1.72.0"]
    pub SUFFIX_WITH_SEPARATOR,
    restriction,
    "prefer numeric literals with a separator (`_`) before the suffix"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for numeric literals (e.g., 1.0_f64) with a separator (`_`) before the suffix (`f64`
    /// in `1.0_f64`).
    ///
    /// ### Why is this bad?
    /// It's not, but enforcing a consistent style is important. In this case the codebase prefers
    /// no separator.
    ///
    /// Also see the `suffix_with_separator` lint for an alternative.
    ///
    /// ### Example
    /// ```rust
    /// let x = 1_i32;
    /// ```
    /// Use instead:
    /// ```rust
    /// let x = 1i32;
    /// ```
    #[clippy::version = "1.72.0"]
    pub SUFFIX_WITHOUT_SEPARATOR,
    restriction,
    "prefer numeric literals without a separator (`_`) before the suffix"
}

pub struct LintPass {
    pub allow_missing_suffix_with_type_annotations: bool,
}
impl_lint_pass!(LintPass => [
    TOPLEVEL_REF_ARG,
    USED_UNDERSCORE_BINDING,
    SHORT_CIRCUIT_STATEMENT,
    ZERO_PTR,
    NUMERIC_LITERAL_MISSING_SUFFIX,
    SUFFIX_WITH_SEPARATOR,
    SUFFIX_WITHOUT_SEPARATOR,
]);

impl<'tcx> LateLintPass<'tcx> for LintPass {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        k: FnKind<'tcx>,
        decl: &'tcx FnDecl<'_>,
        body: &'tcx Body<'_>,
        span: Span,
        _: LocalDefId,
    ) {
        if let FnKind::Closure = k {
            // Does not apply to closures
            return;
        }
        if in_external_macro(cx.tcx.sess, span) {
            return;
        }
        for arg in iter_input_pats(decl, body) {
            // Do not emit if clippy::ref_patterns is not allowed to avoid having two lints for the same issue.
            if !is_lint_allowed(cx, REF_PATTERNS, arg.pat.hir_id) {
                return;
            }
            if let PatKind::Binding(BindingAnnotation(ByRef::Yes, _), ..) = arg.pat.kind {
                span_lint(
                    cx,
                    TOPLEVEL_REF_ARG,
                    arg.pat.span,
                    "`ref` directly on a function argument is ignored. \
                    Consider using a reference type instead",
                );
            }
        }
    }

    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'_>) {
        if_chain! {
            if !in_external_macro(cx.tcx.sess, stmt.span);
            if let StmtKind::Local(local) = stmt.kind;
            if let PatKind::Binding(BindingAnnotation(ByRef::Yes, mutabl), .., name, None) = local.pat.kind;
            if let Some(init) = local.init;
            // Do not emit if clippy::ref_patterns is not allowed to avoid having two lints for the same issue.
            if is_lint_allowed(cx, REF_PATTERNS, local.pat.hir_id);
            then {
                let ctxt = local.span.ctxt();
                let mut app = Applicability::MachineApplicable;
                let sugg_init = Sugg::hir_with_context(cx, init, ctxt, "..", &mut app);
                let (mutopt, initref) = if mutabl == Mutability::Mut {
                    ("mut ", sugg_init.mut_addr())
                } else {
                    ("", sugg_init.addr())
                };
                let tyopt = if let Some(ty) = local.ty {
                    let ty_snip = snippet_with_context(cx, ty.span, ctxt, "_", &mut app).0;
                    format!(": &{mutopt}{ty_snip}")
                } else {
                    String::new()
                };
                span_lint_hir_and_then(
                    cx,
                    TOPLEVEL_REF_ARG,
                    init.hir_id,
                    local.pat.span,
                    "`ref` on an entire `let` pattern is discouraged, take a reference with `&` instead",
                    |diag| {
                        diag.span_suggestion(
                            stmt.span,
                            "try",
                            format!(
                                "let {name}{tyopt} = {initref};",
                                name=snippet(cx, name.span, ".."),
                            ),
                            app,
                        );
                    }
                );
            }
        };
        if_chain! {
            if let StmtKind::Semi(expr) = stmt.kind;
            if let ExprKind::Binary(ref binop, a, b) = expr.kind;
            if binop.node == BinOpKind::And || binop.node == BinOpKind::Or;
            if let Some(sugg) = Sugg::hir_opt(cx, a);
            then {
                span_lint_hir_and_then(
                    cx,
                    SHORT_CIRCUIT_STATEMENT,
                    expr.hir_id,
                    stmt.span,
                    "boolean short circuit operator in statement may be clearer using an explicit test",
                    |diag| {
                        let sugg = if binop.node == BinOpKind::Or { !sugg } else { sugg };
                        diag.span_suggestion(
                            stmt.span,
                            "replace it with",
                            format!(
                                "if {sugg} {{ {}; }}",
                                &snippet(cx, b.span, ".."),
                            ),
                            Applicability::MachineApplicable, // snippet
                        );
                    });
            }
        };
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Lit(lit) = expr.kind {
            self.check_lit(cx, lit, expr);
        }
        if let ExprKind::Cast(e, ty) = expr.kind {
            self.check_cast(cx, expr.span, e, ty);
            return;
        }
        if in_attributes_expansion(expr) || expr.span.is_desugaring(DesugaringKind::Await) {
            // Don't lint things expanded by #[derive(...)], etc or `await` desugaring
            return;
        }
        let sym;
        let binding = match expr.kind {
            ExprKind::Path(ref qpath) if !matches!(qpath, hir::QPath::LangItem(..)) => {
                let binding = last_path_segment(qpath).ident.as_str();
                if binding.starts_with('_') &&
                    !binding.starts_with("__") &&
                    binding != "_result" && // FIXME: #944
                    is_used(cx, expr) &&
                    // don't lint if the declaration is in a macro
                    non_macro_local(cx, cx.qpath_res(qpath, expr.hir_id))
                {
                    Some(binding)
                } else {
                    None
                }
            },
            ExprKind::Field(_, ident) => {
                sym = ident.name;
                let name = sym.as_str();
                if name.starts_with('_') && !name.starts_with("__") {
                    Some(name)
                } else {
                    None
                }
            },
            _ => None,
        };
        if let Some(binding) = binding {
            span_lint(
                cx,
                USED_UNDERSCORE_BINDING,
                expr.span,
                &format!(
                    "used binding `{binding}` which is prefixed with an underscore. A leading \
                     underscore signals that a binding will not be used"
                ),
            );
        }
    }
}

/// Heuristic to see if an expression is used. Should be compatible with
/// `unused_variables`'s idea
/// of what it means for an expression to be "used".
fn is_used(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    get_parent_expr(cx, expr).map_or(true, |parent| match parent.kind {
        ExprKind::Assign(_, rhs, _) | ExprKind::AssignOp(_, _, rhs) => SpanlessEq::new(cx).eq_expr(rhs, expr),
        _ => is_used(cx, parent),
    })
}

/// Tests whether an expression is in a macro expansion (e.g., something
/// generated by `#[derive(...)]` or the like).
fn in_attributes_expansion(expr: &Expr<'_>) -> bool {
    use rustc_span::hygiene::MacroKind;
    if expr.span.from_expansion() {
        let data = expr.span.ctxt().outer_expn_data();
        matches!(data.kind, ExpnKind::Macro(MacroKind::Attr | MacroKind::Derive, _))
    } else {
        false
    }
}

/// Tests whether `res` is a variable defined outside a macro.
fn non_macro_local(cx: &LateContext<'_>, res: def::Res) -> bool {
    if let def::Res::Local(id) = res {
        !cx.tcx.hir().span(id).from_expansion()
    } else {
        false
    }
}

impl<'tcx> LintPass {
    #[expect(clippy::unused_self)]
    fn check_cast(&self, cx: &LateContext<'_>, span: Span, e: &Expr<'_>, ty: &hir::Ty<'_>) {
        if_chain! {
            if let TyKind::Ptr(ref mut_ty) = ty.kind;
            if is_integer_literal(e, 0);
            if !in_constant(cx, e.hir_id);
            then {
                let (msg, sugg_fn) = match mut_ty.mutbl {
                    Mutability::Mut => ("`0 as *mut _` detected", "ptr::null_mut"),
                    Mutability::Not => ("`0 as *const _` detected", "ptr::null"),
                };
                let std_or_core = std_or_core(cx).unwrap_or("...");

                let (sugg, appl) = if let TyKind::Infer = mut_ty.ty.kind {
                    (format!("{std_or_core}::{sugg_fn}()"), Applicability::MachineApplicable)
                } else if let Some(mut_ty_snip) = snippet_opt(cx, mut_ty.ty.span) {
                    (format!("{std_or_core}::{sugg_fn}::<{mut_ty_snip}>()"), Applicability::MachineApplicable)
                } else {
                    // `MaybeIncorrect` as type inference may not work with the suggested code
                    (format!("{std_or_core}::{sugg_fn}()"), Applicability::MaybeIncorrect)
                };
                span_lint_and_sugg(cx, ZERO_PTR, span, msg, "try", sugg, appl);
            }
        }
    }

    fn check_lit(&self, cx: &LateContext<'tcx>, lit: &Lit, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        if !matches!(lit.node, LitKind::Int(_, _) | LitKind::Float(_, _)) {
            return;
        }

        let sm = cx.sess().source_map();
        let ty = cx.typeck_results().expr_ty(expr).to_string();

        if lit.node.is_unsuffixed()
            && !is_from_proc_macro(cx, expr)
            && (!self.allow_missing_suffix_with_type_annotations
                || !(matches!(
                    cx.tcx.hir().owner(cx.tcx.hir().get_parent_item(expr.hir_id)),
                    OwnerNode::Item(item) if matches!(
                        item.kind,
                        ItemKind::Static(_, _, _) | ItemKind::Const(_, _),
                    ),
                ) || cx
                    .tcx
                    .hir()
                    .parent_iter(expr.hir_id)
                    .any(|(_, p)| matches!(p, Node::Local(local) if local.ty.is_some()))))
        {
            span_lint_and_sugg(
                cx,
                NUMERIC_LITERAL_MISSING_SUFFIX,
                lit.span.shrink_to_hi(),
                "this literal is missing an explicit suffix",
                "add it",
                ty.clone(),
                Applicability::MachineApplicable,
            );
        }

        if lit.node.is_suffixed()
            && let Some(ty_first_char) = ty.chars().next()
            && let separator_span = sm.span_extend_to_prev_char(lit.span.shrink_to_hi(), ty_first_char, false)
            // TODO: There's probably a better way to do this. We want to turn `64` from `_f64` into `_`
            && let separator_span = separator_span
                .with_lo(separator_span.lo() - BytePos(2))
                .with_hi(separator_span.lo() - BytePos(1))
            && let Some(separator_snip) = snippet_opt(cx, separator_span)
        {
            if separator_snip == "_" && !is_from_proc_macro(cx, expr) {
                span_lint_and_sugg(
                    cx,
                    SUFFIX_WITHOUT_SEPARATOR,
                    separator_span,
                    "this literal has a separator before its suffix",
                    "remove it",
                    String::new(),
                    Applicability::MachineApplicable,
                );
            } else if !is_from_proc_macro(cx, expr) {
                span_lint_and_sugg(
                    cx,
                    SUFFIX_WITH_SEPARATOR,
                    // Since this one has no separator, we must be careful with our suggestion. We
                    // cannot just use the original `separator_span` as that'll overwrite the last
                    // digit of the literal. So make it empty and point to after that digit.
                    separator_span.with_lo(separator_span.lo() + BytePos(1)).shrink_to_lo(),
                    "this literal is missing a separator before its suffix",
                    "add it",
                    "_".to_owned(),
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}
