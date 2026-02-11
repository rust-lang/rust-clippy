use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_ast::NodeId;
use rustc_ast::ast::{Block, Expr, ExprKind, Fn, FnRetTy, FnSig, MethodCall, Stmt, StmtKind, Ty, Visibility};
use rustc_ast::visit::{AssocCtxt, FnCtxt, FnKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::symbol::kw;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for the `get_` prefix on public getters.
    ///
    /// ### Why is this bad?
    /// The Rust API Guidelines section on naming
    /// [specifies](https://rust-lang-nursery.github.io/api-guidelines/naming.html#getter-names-follow-rust-convention-c-getter)
    /// that the `get_` prefix is not used for getters in Rust code unless
    /// there is a single and obvious thing that could reasonably be gotten by
    /// a getter.
    ///
    /// The exceptions to this naming convention are as follows:
    /// - `get` (such as in
    ///   [`std::cell::Cell::get`](https://doc.rust-lang.org/std/cell/struct.Cell.html#method.get))
    /// - `get_mut`
    /// - `get_unchecked`
    /// - `get_unchecked_mut`
    /// - `get_ref`
    ///
    /// ### Example
    /// ```no_run
    /// struct B {
    ///     id: usize
    /// }
    ///
    /// impl B {
    ///     // Bad
    ///     pub fn get_id(&self) -> usize {
    ///         self.id
    ///     }
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct G {
    ///     id: usize
    /// }
    ///
    /// impl G {
    ///     // Good
    ///     pub fn id(&self) -> usize {
    ///         self.id
    ///     }
    ///
    ///     // Also allowed
    ///     pub fn get(&self) -> usize {
    ///         self.id
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.95.0"]
    pub GETTER_PREFIXES,
    style,
    "prefixing a getter with `get_`, which does not follow convention"
}

const EXCLUDED_SUFFIXES: [&str; 5] = ["", "mut", "unchecked", "unchecked_mut", "ref"];

declare_lint_pass!(GetterPrefixes => [GETTER_PREFIXES]);

impl EarlyLintPass for GetterPrefixes {
    fn check_fn(&mut self, cx: &EarlyContext<'_>, fn_kind: FnKind<'_>, _: Span, _: NodeId) {
        if let FnKind::Fn(
            FnCtxt::Assoc(AssocCtxt::Impl { of_trait: false }),
            Visibility {
                kind: visibility_kind, ..
            },
            &Fn {
                ref ident,
                sig: FnSig { ref decl, .. },
                body: Some(ref block),
                ..
            },
        ) = fn_kind
            && visibility_kind.is_pub()
            && decl.has_self()
            && let FnRetTy::Ty(box Ty { ref kind, .. }) = decl.output
            && !kind.is_unit()
            && let Some(ref suffix) = ident.name.as_str().strip_prefix("get_")
            && !EXCLUDED_SUFFIXES.contains(suffix)
            && let Some(expr) = body_return_expr(block)
            && has_trivial_getter_logic(expr)
        {
            span_lint_and_sugg(
                cx,
                GETTER_PREFIXES,
                ident.span,
                "prefixing a getter with `get_` does not follow naming conventions",
                "replace it with",
                suffix.to_string(),
                Applicability::Unspecified,
            );
        }
    }
}

fn body_return_expr(block: &Block) -> Option<&Expr> {
    match block.stmts.last() {
        Some(&Stmt {
            kind:
                StmtKind::Expr(ref expr)
                | StmtKind::Semi(box Expr {
                    kind: ExprKind::Ret(Some(ref expr)),
                    ..
                }),
            ..
        }) => Some(expr),
        _ => None,
    }
}

fn has_trivial_getter_logic(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Lit(lit) => lit.symbol == kw::SelfLower,
        ExprKind::Path(None, path) if path.segments.first().is_some_and(|seg| seg.ident.name == kw::SelfLower) => true,

        ExprKind::Array(expr_vec) | ExprKind::Tup(expr_vec) => {
            expr_vec.iter().any(|expr| has_trivial_getter_logic(expr))
        },

        ExprKind::Paren(box inner)
        | ExprKind::Cast(box inner, _)
        | ExprKind::Field(box inner, _)
        | ExprKind::Index(box inner, _, _)
        | ExprKind::AddrOf(_, _, box inner)
        | ExprKind::Unary(_, box inner) => has_trivial_getter_logic(inner),

        ExprKind::Binary(_, box lhs_expr, box rhs_expr) => {
            has_trivial_getter_logic(lhs_expr) || has_trivial_getter_logic(rhs_expr)
        },

        ExprKind::MethodCall(box MethodCall { box receiver, .. }) => has_trivial_getter_logic(receiver),

        _ => false,
    }
}
