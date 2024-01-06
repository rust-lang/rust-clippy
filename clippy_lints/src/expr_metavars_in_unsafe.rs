use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::is_lint_allowed;
use itertools::Itertools;
use rustc_hir::intravisit::{walk_block, walk_expr, walk_stmt, Visitor};
use rustc_hir::{BlockCheckMode, Expr, ExprKind, HirId, Stmt, UnsafeSource};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::def_id::DefId;
use rustc_span::{sym, Span, SyntaxContext};

declare_clippy_lint! {
    /// ### What it does
    /// Looks for macros that expand metavariables in an unsafe block.
    ///
    /// ### Why is this bad?
    /// This is unsound: it allows the user of the macro to write unsafe code outside of an
    /// unsafe block at callsite, potentially invoking undefined behavior in safe code.
    ///
    /// ### Known limitations
    /// Due to how macros are represented in the compiler at the time Clippy runs its lints,
    /// it's not possible to look for metavariables in macro definitions directly.
    ///
    /// Instead, this lint looks at expansions of macros defined in the same crate.
    /// This leads to false negatives when a macro is never actually invoked.
    ///
    /// ### Example
    /// ```no_run
    /// /// Gets the first element of a slice
    /// macro_rules! first {
    ///     ($slice:expr) => {
    ///         unsafe {
    ///             let slice = $slice; // ⚠️ expansion inside of `unsafe {}`
    ///
    ///             assert!(!slice.is_empty());
    ///             // SAFETY: slice is checked to have at least one element
    ///             slice.first().unwrap_unchecked()
    ///         }
    ///     }
    /// }
    ///
    /// assert_eq!(*first!(&[1i32]), 1);
    ///
    /// // This will compile as a consequence (note the lack of `unsafe {}`)
    /// assert_eq!(*first!(std::hint::unreachable_unchecked() as &[i32]), 1);
    /// ```
    /// Use instead:
    /// ```compile_fail
    /// macro_rules! first {
    ///     ($slice:expr) => {{
    ///         let slice = $slice; // ✅ outside of `unsafe {}`
    ///         unsafe {
    ///             assert!(!slice.is_empty());
    ///             // SAFETY: slice is checked to have at least one element
    ///             slice.first().unwrap_unchecked()
    ///         }
    ///     }}
    /// }
    ///
    /// assert_eq!(*first!(&[1]), 1);
    ///
    /// // This won't compile:
    /// assert_eq!(*first!(std::hint::unreachable_unchecked() as &[i32]), 1);
    /// ```
    #[clippy::version = "1.77.0"]
    pub EXPR_METAVARS_IN_UNSAFE,
    nursery,
    "expanding expr metavariables in an unsafe block"
}

#[derive(Clone, Debug)]
enum MetavarState {
    ReferencedInUnsafe { unsafe_blocks: Vec<HirId> },
    ReferencedInSafe,
}

#[derive(Default)]
pub struct ExprMetavarsInUnsafe {
    /// A metavariable can be expanded more than once, potentially across multiple bodies, so it
    /// requires some state kept across HIR nodes to make it possible to delay a warning
    /// and later undo:
    ///
    /// ```ignore
    /// macro_rules! x {
    ///     ($v:expr) => {
    ///         unsafe { $v; } // unsafe context, it might be possible to emit a warning here, so add it to the map
    ///
    ///         $v;            // `$v` expanded another time but in a safe context, set to ReferencedInSafe to suppress
    ///     }
    /// }
    /// ```
    metavar_expns: BTreeMap<Span, MetavarState>,
}
impl_lint_pass!(ExprMetavarsInUnsafe => [EXPR_METAVARS_IN_UNSAFE]);

struct BodyVisitor<'a> {
    /// The top item always represents the last seen unsafe block
    macro_unsafe_blocks: Vec<HirId>,
    /// When this is >0, it means that the node in the visitor currently being visited is "within" a
    /// macro definition. This helps reduce the number of spans we need to insert into the map,
    /// since only spans from macros are relevant.
    expn_depth: u32,
    metavar_map: &'a mut BTreeMap<Span, MetavarState>,
}

impl<'a, 'tcx> Visitor<'tcx> for BodyVisitor<'a> {
    fn visit_stmt(&mut self, s: &'tcx Stmt<'tcx>) {
        let from_expn = s.span.from_expansion();
        if from_expn {
            self.expn_depth += 1;
        }
        walk_stmt(self, s);
        if from_expn {
            self.expn_depth -= 1;
        }
    }

    fn visit_expr(&mut self, e: &'tcx Expr<'tcx>) {
        let ctxt = e.span.ctxt();

        if let ExprKind::Block(block, _) = e.kind
            && let BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided) = block.rules
            && !ctxt.is_root()
            && ctxt.outer_expn_data().macro_def_id.is_some_and(DefId::is_local)
        {
            self.macro_unsafe_blocks.push(block.hir_id);
            walk_block(self, block);
            self.macro_unsafe_blocks.pop();
        } else if ctxt.is_root() && self.expn_depth > 0 {
            let unsafe_block = self.macro_unsafe_blocks.last().copied();

            match (self.metavar_map.entry(e.span), unsafe_block) {
                (Entry::Vacant(e), None) => {
                    e.insert(MetavarState::ReferencedInSafe);
                },
                (Entry::Vacant(e), Some(unsafe_block)) => {
                    e.insert(MetavarState::ReferencedInUnsafe {
                        unsafe_blocks: vec![unsafe_block],
                    });
                },
                (Entry::Occupied(mut e), None) => {
                    if let MetavarState::ReferencedInUnsafe { .. } = *e.get() {
                        e.insert(MetavarState::ReferencedInSafe);
                    }
                },
                (Entry::Occupied(mut e), Some(unsafe_block)) => {
                    if let MetavarState::ReferencedInUnsafe { unsafe_blocks } = e.get_mut()
                        && !unsafe_blocks.contains(&unsafe_block)
                    {
                        unsafe_blocks.push(unsafe_block);
                    }
                },
            }

            // NB: No need to visit descendant nodes. They're guaranteed to represent the same
            // metavariable
        } else {
            walk_expr(self, e);
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ExprMetavarsInUnsafe {
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &'tcx rustc_hir::Body<'tcx>) {
        if is_lint_allowed(cx, EXPR_METAVARS_IN_UNSAFE, body.value.hir_id) {
            return;
        }

        // This BodyVisitor is separate and not part of the lint pass because there is no
        // `check_stmt_post` on `(Late)LintPass`, which we'd need to detect when we're leaving a macro span

        let mut vis = BodyVisitor {
            #[expect(clippy::bool_to_int_with_if)] // obfuscates the meaning
            expn_depth: if body.value.span.from_expansion() { 1 } else { 0 },
            macro_unsafe_blocks: Vec::new(),
            metavar_map: &mut self.metavar_expns,
        };
        vis.visit_body(body);
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Aggregate all unsafe blocks from all spans:
        // ```
        // macro_rules! x {
        //   ($w:expr, $x:expr, $y:expr) => { $w; unsafe { $w; $x; }; unsafe { $x; $y; }; }
        // }
        // $w: []  (unsafe#0 is never added because it was referenced in a safe context)
        // $x: [unsafe#0, unsafe#1]
        // $y: [unsafe#1]
        // ```
        // We want to lint unsafe blocks #0 and #1
        let bad_unsafe_blocks = self
            .metavar_expns
            .iter()
            .filter_map(|(_, state)| match state {
                MetavarState::ReferencedInUnsafe { unsafe_blocks } => Some(unsafe_blocks.as_slice()),
                MetavarState::ReferencedInSafe => None,
            })
            .flatten()
            .copied()
            .map(|id| {
                // Remove the syntax context to hide "in this macro invocation" in the diagnostic.
                // The invocation doesn't matter. Also we want to dedupe by the unsafe block and not by anything
                // related to the callsite.
                let span = cx.tcx.hir().span(id);
                let macro_def_id = span.ctxt().outer_expn_data().macro_def_id.and_then(DefId::as_local);
                (
                    id,
                    Span::new(span.lo(), span.hi(), SyntaxContext::root(), None),
                    macro_def_id,
                )
            })
            .dedup_by(|(_, a, _), (_, b, _)| a == b);

        for (id, span, def_id) in bad_unsafe_blocks {
            if let Some(def_id) = def_id
                && (cx.effective_visibilities.is_exported(def_id) || cx.tcx.has_attr(def_id, sym::macro_export))
                && !cx.tcx.is_doc_hidden(def_id)
            {
                span_lint_hir_and_then(
                    cx,
                    EXPR_METAVARS_IN_UNSAFE,
                    id,
                    span,
                    "this unsafe block in a macro expands `expr` metavariables",
                    |diag| {
                        diag.note("this allows the user of the macro to write unsafe code outside of an unsafe block");
                        diag.help(
                            "consider expanding any metavariables outside of this block, e.g. by storing them in a variable",
                        );
                        diag.help(
                            "... or also expand referenced metavariables in a safe context to require an unsafe block at callsite",
                        );
                    },
                );
            }
        }
    }
}
