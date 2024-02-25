use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::AttrArgs;
use rustc_ast::token::{Ident, TokenKind};
use rustc_ast::tokenstream::TokenTree::Token;
use rustc_ast::AttrKind::Normal;
use rustc_ast::UintTy::Usize;
use rustc_ast::{Attribute, LitKind};
use rustc_hir::PrimTy::Uint;
use rustc_hir::{ExprKind, HirId, QPath, Stmt, StmtKind, TyKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_session::declare_lint_pass;
use rustc_span::symbol::sym::target_pointer_width;
use rustc_span::symbol::Symbol;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for assigning a value larges than u32::MAX to an usize variable.
    ///
    /// ### Why is this bad?
    ///
    /// This makes the code non-portable to platforms that have 32-bit pointer width.
    ///
    /// ### Example
    /// ```no_run
    /// let foo : usize = ;
    /// ```
    /// Use instead:
    /// ```no_run
    /// let foo : u64 = 1<<32;
    /// ```
    #[clippy::version = "1.78.0"]
    pub USIZE_UNPORTABLE_32_BIT_LITERAL,
    correctness,
    "assignment of a too large literal to an usize variable"
}

declare_lint_pass!(UsizeUnportable32BitLiteral => [USIZE_UNPORTABLE_32_BIT_LITERAL]);

/// Checks if the type is usize
fn is_usize<'tcx>(stmt: &'tcx Stmt<'tcx>) -> bool {
    if let StmtKind::Local(l) = stmt.kind
        && let Some(ty) = l.ty
    {
        match ty.kind {
            TyKind::Path(QPath::Resolved(_, path)) => matches!(path.res, rustc_hir::def::Res::PrimTy(Uint(Usize))),
            _ => false,
        }
    } else {
        false
    }
}

/// checks if the statement contains a literal that is larger than
/// `u32::MAX`
fn is_larger_than_32bit<'tcx>(stmt: &'tcx Stmt<'tcx>) -> bool {
    if let StmtKind::Local(l) = stmt.kind
        && let Some(init) = l.init
    {
        match init.kind {
            ExprKind::Lit(lit) => match lit.node {
                LitKind::Int(v, _) => v.0 > u32::MAX.into(),
                _ => false,
            },
            _ => false,
        }
    } else {
        false
    }
}

/// Checks if the attribute contains `target_pointer_width` = "64"
fn has_pointer_width_64_attr(attrs: &[Attribute]) -> bool {
    let ident = Ident(target_pointer_width, false);
    let str64 = TokenKind::Literal(rustc_ast::token::Lit {
        kind: rustc_ast::token::LitKind::Str,
        symbol: Symbol::intern("64"),
        suffix: None,
    });
    let mut state = 0;
    for attr in attrs {
        if let Normal(normal_attr) = &attr.kind
            && let AttrArgs::Delimited(args) = &normal_attr.item.args
        {
            for t in args.tokens.trees() {
                if let Token(token, _) = t {
                    if state == 0 && token.kind == ident {
                        state = 1;
                    } else if state == 1 && token.kind == TokenKind::Eq {
                        state = 2;
                    } else if state == 2 && token.kind == str64 {
                        return true;
                    } else {
                        state = 0;
                    }
                }
            }
        }
    }
    false
}

/// Checks if we are in the context of a `target_pointer_width` = "64" config
fn any_parent_has_64_pointer_width(tcx: TyCtxt<'_>, node: HirId) -> bool {
    let map = &tcx.hir();
    let mut prev_enclosing_node = None;
    let mut enclosing_node = node;
    while Some(enclosing_node) != prev_enclosing_node {
        if has_pointer_width_64_attr(map.attrs(enclosing_node)) {
            return true;
        }
        prev_enclosing_node = Some(enclosing_node);
        enclosing_node = map.get_parent_item(enclosing_node).into();
    }

    false
}

impl<'tcx> LateLintPass<'tcx> for UsizeUnportable32BitLiteral {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if any_parent_has_64_pointer_width(cx.tcx, stmt.hir_id) {
            return;
        }
        if is_usize(stmt) && is_larger_than_32bit(stmt) {
            span_lint_and_help(
                cx,
                USIZE_UNPORTABLE_32_BIT_LITERAL,
                stmt.span,
                "assignement to usize not portable to 32bit architectures",
                None,
                "use u64 as the data type if you need values larger than 32 bit",
            );
        }
    }
}
