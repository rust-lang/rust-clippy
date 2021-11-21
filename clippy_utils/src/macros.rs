use rustc_hir::def_id::DefId;
use rustc_hir::{self as hir, HirId, Node};
use rustc_lint::LateContext;
use rustc_span::hygiene::{MacroKind, SyntaxContext};
use rustc_span::{ExpnData, ExpnId, ExpnKind, Span};

/// A macro call, like `vec![1, 2, 3]`.
///
/// Use `tcx.item_name(macro_call.def_id)` to get the macro name.
/// Even better is to check if it is a diagnostic item.
///
/// This structure is similar to `ExpnData` but it precludes desugaring expansions.
#[derive(Debug)]
pub struct MacroCall {
    /// Macro `DefId`
    pub def_id: DefId,
    /// Kind of macro
    pub kind: MacroKind,
    /// The expansion produced by the macro call
    pub expn: ExpnId,
    /// Span of the macro call site
    pub span: Span,
}

impl MacroCall {
    pub fn is_local(&self) -> bool {
        span_is_local(self.span)
    }
}

/// Returns an iterator of expansions that created the given span
pub fn expn_backtrace(mut span: Span) -> impl Iterator<Item = (ExpnId, ExpnData)> {
    std::iter::from_fn(move || {
        let ctxt = span.ctxt();
        if ctxt == SyntaxContext::root() {
            return None;
        }
        let expn = ctxt.outer_expn();
        let data = expn.expn_data();
        span = data.call_site;
        Some((expn, data))
    })
}

/// Checks whether the span is from the root expansion or a locally defined macro
pub fn span_is_local(span: Span) -> bool {
    !span.from_expansion() || expn_is_local(span.ctxt().outer_expn())
}

/// Checks whether the expansion is the root expansion or a locally defined macro
pub fn expn_is_local(expn: ExpnId) -> bool {
    if expn == ExpnId::root() {
        return true;
    }
    let data = expn.expn_data();
    let backtrace = expn_backtrace(data.call_site);
    std::iter::once((expn, data))
        .chain(backtrace)
        .find_map(|(_, data)| data.macro_def_id)
        .map_or(true, DefId::is_local)
}

/// Returns an iterator of macro expansions that created the given span.
/// Note that desugaring expansions are skipped.
pub fn macro_backtrace(span: Span) -> impl Iterator<Item = MacroCall> {
    expn_backtrace(span).filter_map(|(expn, data)| match data {
        ExpnData {
            kind: ExpnKind::Macro(kind, _),
            macro_def_id: Some(def_id),
            call_site: span,
            ..
        } => Some(MacroCall {
            def_id,
            kind,
            expn,
            span,
        }),
        _ => None,
    })
}

/// If the macro backtrace of `span` has a macro call at the root expansion
/// (i.e. not a nested macro call), returns `Some` with the `MacroCall`
pub fn root_macro_call(span: Span) -> Option<MacroCall> {
    macro_backtrace(span).last()
}

/// Like [`root_macro_call`], but only returns `Some` if `node` is the "first node"
/// produced by the macro call, as in [`first_node_in_macro`].
pub fn root_macro_call_first_node(cx: &LateContext<'_>, node: &impl HirNode) -> Option<MacroCall> {
    if first_node_in_macro(cx, node) != Some(ExpnId::root()) {
        return None;
    }
    root_macro_call(node.span())
}

/// Like [`macro_backtrace`], but only returns macro calls where `node` is the "first node" of the
/// macro call, as in [`first_node_in_macro`].
pub fn first_node_macro_backtrace(cx: &LateContext<'_>, node: &impl HirNode) -> impl Iterator<Item = MacroCall> {
    let span = node.span();
    first_node_in_macro(cx, node)
        .into_iter()
        .flat_map(move |expn| macro_backtrace(span).take_while(move |macro_call| macro_call.expn != expn))
}

/// If `node` is the "first node" in a macro expansion, returns `Some` with the `ExpnId` of the
/// macro call site (i.e. the parent of the macro expansion). This generally means that `node`
/// is the outermost node of an entire macro expansion, but there are some caveats noted below.
/// This is useful for finding macro calls while visiting the HIR without processing the macro call
/// at every node within its expansion.
///
/// If you already have immediate access to the parent node, it is simpler to
/// just check the context of that span directly (e.g. `parent.span.from_expansion()`).
///
/// If a macro call is in statement position, it expands to one or more statements.
/// In that case, each statement *and* their immediate descendants will all yield `Some`
/// with the `ExpnId` of the containing block.
///
/// A node may be the "first node" of multiple macro calls in a macro backtrace.
/// The expansion of the outermost macro call site is returned in such cases.
pub fn first_node_in_macro(cx: &LateContext<'_>, node: &impl HirNode) -> Option<ExpnId> {
    // get the macro expansion or return `None` if not found
    // `macro_backtrace` importantly ignores desugaring expansions
    let expn = macro_backtrace(node.span()).next()?.expn;

    // get the parent node, possibly skipping over a statement
    // if the parent is not found, it is sensible to return `Some(root)`
    let hir = cx.tcx.hir();
    let mut parent_iter = hir.parent_iter(node.hir_id());
    let (parent_id, _) = match parent_iter.next() {
        None => return Some(ExpnId::root()),
        Some((_, Node::Stmt(_))) => match parent_iter.next() {
            None => return Some(ExpnId::root()),
            Some(next) => next,
        },
        Some(next) => next,
    };

    // get the macro expansion of the parent node
    let parent_span = hir.span(parent_id);
    let Some(parent_macro_call) = macro_backtrace(parent_span).next() else {
        // the parent node is not in a macro
        return Some(ExpnId::root());
    };

    if parent_macro_call.expn.is_descendant_of(expn) {
        // `node` is input to a macro call
        return None;
    }

    Some(parent_macro_call.expn)
}

pub trait HirNode {
    fn hir_id(&self) -> HirId;
    fn span(&self) -> Span;
}

macro_rules! impl_hir_node {
    ($($t:ident),*) => {
        $(impl HirNode for hir::$t<'_> {
            fn hir_id(&self) -> HirId {
                self.hir_id
            }
            fn span(&self) -> Span {
                self.span
            }
        })*
    };
}

impl_hir_node!(Expr, Pat);

impl HirNode for hir::Item<'_> {
    fn hir_id(&self) -> HirId {
        self.hir_id()
    }

    fn span(&self) -> Span {
        self.span
    }
}
