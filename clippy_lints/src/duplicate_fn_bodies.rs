use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::{in_automatically_derived, is_from_proc_macro, tokenize_with_text};
use rustc_abi::FieldIdx;
use rustc_data_structures::fx::{FxHashMap, FxHasher};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::intravisit::{self, FnKind, Visitor};
use rustc_hir::{
    AmbigArg, Block, Body, BodyId, Constness, Expr, ExprKind, FnDecl, HirId, IsAsync, Lit, Pat, PatExprKind, PatKind,
    Path, PathSegment, QPath, Stmt, StmtKind, Ty, TyKind,
};
use rustc_lexer::TokenKind;
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_middle::hir::nested_filter;
use rustc_middle::ty;
use rustc_middle::ty::Ty as TyCtxt;
use rustc_middle::ty::adjustment::{
    Adjust, AllowTwoPhase, AutoBorrow, AutoBorrowMutability, DerefAdjustKind, PatAdjust, PointerCoercion,
};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use std::hash::{Hash, Hasher};
use std::mem;

const MIN_BODY_TOKENS: u32 = 12;

const SHAPE_BODY_START: u8 = 0xF0;
const SHAPE_BODY_END: u8 = 0xF1;
const SHAPE_STMT: u8 = 0x01;
const SHAPE_BLOCK: u8 = 0x02;
const SHAPE_PAT: u8 = 0x03;
const SHAPE_EXPR: u8 = 0x04;
const SHAPE_TY: u8 = 0x05;
const SHAPE_PATH: u8 = 0x06;
const SHAPE_LIT: u8 = 0x07;

declare_clippy_lint! {
    /// ### What it does
    /// Detects functions whose bodies contain the same Rust token sequence and whose relevant
    /// name/type resolution is also equivalent.
    ///
    /// Whitespace and comments are ignored. The lint deliberately remains a Type-1 clone
    /// detector: identifier and literal spellings are preserved, so renaming a local or changing
    /// a literal makes two bodies different.
    ///
    /// To avoid suggesting refactorings that may not be possible, free functions are only
    /// compared with other free functions in the same module, and inherent methods are only
    /// compared with methods in the same inherent `impl`. Trait implementations are ignored.
    ///
    /// Function bodies originating from macro expansion, proc macros, automatically derived code,
    /// generic functions, and constructs for which semantic equivalence cannot be proven
    /// conservatively are ignored.
    ///
    /// ### Why is this bad?
    /// Identical function bodies can increase maintenance burden and allow copies to drift apart.
    /// A bug fix may be applied to one copy but not another.
    ///
    /// Identical code is not necessarily a problem, so this lint only emits when it can establish
    /// both exact Type-1 body identity and a compatible semantic/refactoring domain.
    ///
    /// ### Example
    /// ```rust,ignore
    /// fn first(value: i32) -> i32 {
    ///     let adjusted = value + 1;
    ///     adjusted * 2
    /// }
    ///
    /// fn second(value: i32) -> i32 {
    ///     let adjusted = value + 1;
    ///     adjusted * 2
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// fn shared(value: i32) -> i32 {
    ///     let adjusted = value + 1;
    ///     adjusted * 2
    /// }
    ///
    /// fn first(value: i32) -> i32 {
    ///     shared(value)
    /// }
    ///
    /// fn second(value: i32) -> i32 {
    ///     shared(value)
    /// }
    /// ```
    #[clippy::version = "1.99.0"]
    pub DUPLICATE_FN_BODIES,
    nursery,
    "function body is identical to another function body"
}

impl_lint_pass!(DuplicateFnBodies => [DUPLICATE_FN_BODIES]);

#[derive(Default)]
pub struct DuplicateFnBodies {
    pending: Vec<PendingBody>,
    active: Vec<ShapeFrame>,
    bodies: Vec<BodyEntry>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DomainKey {
    kind: u8,
    def_index: u32,
}

impl DomainKey {
    const MODULE: u8 = 0;
    const INHERENT_IMPL: u8 = 1;

    fn module(def_id: LocalDefId) -> Self {
        Self {
            kind: Self::MODULE,
            def_index: def_id.local_def_index.as_u32(),
        }
    }

    fn inherent_impl(def_id: LocalDefId) -> Self {
        Self {
            kind: Self::INHERENT_IMPL,
            def_index: def_id.local_def_index.as_u32(),
        }
    }
}

struct PendingBody {
    def_id: LocalDefId,
    hir_id: HirId,
    body_id: BodyId,
    function_span: Span,
    body_span: Span,
    domain: DomainKey,
    is_const: bool,
    is_async: bool,
}

struct ShapeFrame {
    pending: PendingBody,
    hasher: FxHasher,
    node_count: u32,
    unsupported: bool,
}

impl ShapeFrame {
    fn new(pending: PendingBody) -> Self {
        Self {
            pending,
            hasher: FxHasher::default(),
            node_count: 0,
            unsupported: false,
        }
    }

    #[inline]
    fn hash<T: Hash + ?Sized>(&mut self, value: &T) {
        value.hash(&mut self.hasher);
    }

    #[inline]
    fn bump_node(&mut self) {
        self.node_count = self.node_count.saturating_add(1);
    }
}

struct BodyEntry {
    def_id: LocalDefId,
    hir_id: HirId,
    body_id: BodyId,
    function_span: Span,
    body_span: Span,
    domain: DomainKey,
    shape_hash: u64,
    node_count: u32,
    is_const: bool,
    is_async: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TokenKey {
    token_count: u32,
    hash: u64,
}

impl DuplicateFnBodies {
    #[inline]
    fn hash_active<T: Hash + ?Sized>(&mut self, value: &T) {
        for frame in &mut self.active {
            frame.hash(value);
        }
    }

    #[inline]
    fn bump_active(&mut self) {
        for frame in &mut self.active {
            frame.bump_node();
        }
    }

    #[inline]
    fn mark_active_unsupported(&mut self) {
        for frame in &mut self.active {
            frame.unsupported = true;
        }
    }

    #[inline]
    fn hash_node_tag<T>(&mut self, tag: u8, value: &T)
    where
        T: Hash + ?Sized,
    {
        self.hash_active(&tag);
        self.hash_active(value);
        self.bump_active();
    }

    fn begin_body_if_pending(&mut self, body: &Body<'_>) {
        let body_id = body.id();

        let Some(index) = self.pending.iter().rposition(|pending| pending.body_id == body_id) else {
            return;
        };

        let pending = self.pending.remove(index);
        self.active.push(ShapeFrame::new(pending));
    }

    fn finish_body_if_active(&mut self, body: &Body<'_>) {
        let body_id = body.id();

        if !self.active.last().is_some_and(|frame| frame.pending.body_id == body_id) {
            return;
        }

        let frame = self.active.pop().unwrap();

        if frame.unsupported {
            return;
        }

        let ShapeFrame {
            pending,
            hasher,
            node_count,
            unsupported: _,
        } = frame;

        self.bodies.push(BodyEntry {
            def_id: pending.def_id,
            hir_id: pending.hir_id,
            body_id: pending.body_id,
            function_span: pending.function_span,
            body_span: pending.body_span,
            domain: pending.domain,
            shape_hash: hasher.finish(),
            node_count,
            is_const: pending.is_const,
            is_async: pending.is_async,
        });
    }
}

impl<'tcx> LateLintPass<'tcx> for DuplicateFnBodies {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        function_span: Span,
        def_id: LocalDefId,
    ) {
        if !matches!(kind, FnKind::ItemFn(..) | FnKind::Method(..)) {
            return;
        }

        let hir_id = cx.tcx.local_def_id_to_hir_id(def_id);

        if function_span.from_expansion()
            || body.value.span.from_expansion()
            || in_automatically_derived(cx.tcx, hir_id)
        {
            return;
        }

        if is_from_proc_macro(cx, &(&kind, body, hir_id, function_span)) {
            return;
        }

        if !cx.tcx.generics_of(def_id).own_params.is_empty() {
            return;
        }

        let Some(domain) = extraction_domain(cx, kind, def_id) else {
            return;
        };

        self.pending.push(PendingBody {
            def_id,
            hir_id,
            body_id: body.id(),
            function_span,
            body_span: body.value.span,
            domain,
            is_const: matches!(kind.constness(), Constness::Const { .. }),
            is_async: matches!(kind.asyncness(), IsAsync::Async(_)),
        });
    }

    fn check_body(&mut self, _: &LateContext<'tcx>, body: &Body<'tcx>) {
        self.begin_body_if_pending(body);

        self.hash_active(&SHAPE_BODY_START);
    }

    fn check_body_post(&mut self, _: &LateContext<'tcx>, body: &Body<'tcx>) {
        self.hash_active(&SHAPE_BODY_END);
        self.finish_body_if_active(body);
    }

    fn check_stmt(&mut self, _: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if self.active.is_empty() {
            return;
        }

        self.hash_node_tag(SHAPE_STMT, &mem::discriminant(&stmt.kind));

        if matches!(stmt.kind, StmtKind::Item(_)) {
            self.mark_active_unsupported();
        }
    }

    fn check_block(&mut self, _: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        if self.active.is_empty() {
            return;
        }

        self.hash_node_tag(SHAPE_BLOCK, &mem::discriminant(&block.rules));
    }

    fn check_pat(&mut self, _: &LateContext<'tcx>, pat: &'tcx Pat<'tcx>) {
        if self.active.is_empty() {
            return;
        }

        self.hash_node_tag(SHAPE_PAT, &mem::discriminant(&pat.kind));

        if let PatKind::Binding(_, _, ident, _) = pat.kind {
            self.hash_active(&ident.name);
        }

        if matches!(pat.kind, PatKind::Missing | PatKind::Err(_)) {
            self.mark_active_unsupported();
        }
    }

    fn check_expr(&mut self, _: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if self.active.is_empty() {
            return;
        }

        self.hash_node_tag(SHAPE_EXPR, &mem::discriminant(&expr.kind));

        match expr.kind {
            ExprKind::Binary(op, _, _) => {
                self.hash_active(&mem::discriminant(&op.node));
            },

            ExprKind::AssignOp(op, _, _) => {
                self.hash_active(&mem::discriminant(&op.node));
            },

            ExprKind::MethodCall(segment, _, _, _) => {
                self.hash_active(&segment.ident.name);
            },

            ExprKind::Field(_, ident) => {
                self.hash_active(&ident.name);
            },

            ExprKind::Break(destination, _) => {
                if let Some(label) = destination.label {
                    self.hash_active(&label.ident.name);
                }
            },

            ExprKind::Continue(destination) => {
                if let Some(label) = destination.label {
                    self.hash_active(&label.ident.name);
                }
            },

            ExprKind::Loop(_, label, source, _) => {
                self.hash_active(&mem::discriminant(&source));

                if let Some(label) = label {
                    self.hash_active(&label.ident.name);
                }
            },

            ExprKind::Match(_, _, source) => {
                self.hash_active(&mem::discriminant(&source));
            },

            ExprKind::ConstBlock(_) | ExprKind::InlineAsm(_) | ExprKind::OffsetOf(..) | ExprKind::Err(_) => {
                self.mark_active_unsupported();
            },

            _ => {},
        }
    }

    fn check_ty(&mut self, _: &LateContext<'tcx>, ty: &'tcx Ty<'tcx, AmbigArg>) {
        if self.active.is_empty() {
            return;
        }

        self.hash_node_tag(SHAPE_TY, &mem::discriminant(&ty.kind));

        if matches!(ty.kind, TyKind::Err(_)) {
            self.mark_active_unsupported();
        }
    }

    fn check_path(&mut self, _: &LateContext<'tcx>, path: &Path<'tcx>, _: HirId) {
        if self.active.is_empty() {
            return;
        }

        self.hash_active(&SHAPE_PATH);
        self.hash_active(&path.segments.len());

        for segment in path.segments {
            self.hash_active(&segment.ident.name);
        }

        self.bump_active();
    }

    fn check_lit(&mut self, _: &LateContext<'tcx>, _: HirId, lit: Lit, is_negated_pat: bool) {
        if self.active.is_empty() {
            return;
        }

        self.hash_active(&SHAPE_LIT);
        self.hash_active(&lit.node);
        self.hash_active(&is_negated_pat);
    }

    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        self.bodies.sort_unstable_by_key(|entry| {
            (
                entry.domain,
                entry.node_count,
                entry.shape_hash,
                entry.function_span.lo(),
                entry.function_span.hi(),
            )
        });

        let mut start = 0;

        while start < self.bodies.len() {
            let first = &self.bodies[start];

            let mut end = start + 1;
            while end < self.bodies.len()
                && self.bodies[end].domain == first.domain
                && self.bodies[end].node_count == first.node_count
                && self.bodies[end].shape_hash == first.shape_hash
            {
                end += 1;
            }

            if end - start >= 2 {
                process_shape_bucket(cx, &self.bodies[start..end]);
            }

            start = end;
        }
    }
}

fn extraction_domain(cx: &LateContext<'_>, kind: FnKind<'_>, def_id: LocalDefId) -> Option<DomainKey> {
    match kind {
        FnKind::ItemFn(..) => {
            let parent = cx.tcx.opt_local_parent(def_id)?;

            if !matches!(cx.tcx.def_kind(parent), DefKind::Mod) {
                return None;
            }

            Some(DomainKey::module(parent))
        },

        FnKind::Method(..) => {
            let impl_id = cx.tcx.inherent_impl_of_assoc(def_id.to_def_id())?.as_local()?;

            Some(DomainKey::inherent_impl(impl_id))
        },

        FnKind::Closure => None,
    }
}

fn process_shape_bucket<'tcx>(cx: &LateContext<'tcx>, entries: &[BodyEntry]) {
    let mut token_candidates = Vec::with_capacity(entries.len());

    for entry in entries {
        let Some(key) = token_key(cx, entry.body_span) else {
            continue;
        };

        if key.token_count < MIN_BODY_TOKENS {
            continue;
        }

        token_candidates.push((key, entry));
    }

    if token_candidates.len() < 2 {
        return;
    }

    token_candidates.sort_unstable_by_key(|(key, entry)| (*key, entry.function_span.lo(), entry.function_span.hi()));

    let mut start = 0;

    while start < token_candidates.len() {
        let key = token_candidates[start].0;

        let mut end = start + 1;
        while end < token_candidates.len() && token_candidates[end].0 == key {
            end += 1;
        }

        if end - start >= 2 {
            let entries = token_candidates[start..end]
                .iter()
                .map(|(_, entry)| *entry)
                .collect::<Vec<_>>();

            process_token_hash_bucket(cx, &entries);
        }

        start = end;
    }
}

fn process_token_hash_bucket<'tcx>(cx: &LateContext<'tcx>, candidates: &[&BodyEntry]) {
    let mut exact_token_classes: Vec<Vec<&BodyEntry>> = Vec::new();

    for &candidate in candidates {
        let mut matching_class = None;

        for (index, class) in exact_token_classes.iter().enumerate() {
            let representative = class[0];

            if token_stream_eq(cx, representative.body_span, candidate.body_span) {
                matching_class = Some(index);
                break;
            }
        }

        if let Some(index) = matching_class {
            exact_token_classes[index].push(candidate);
        } else {
            exact_token_classes.push(vec![candidate]);
        }
    }

    for token_class in exact_token_classes {
        if token_class.len() < 2 {
            continue;
        }

        process_exact_token_class(cx, &token_class);
    }
}

struct SemanticCandidate<'a, 'tcx> {
    entry: &'a BodyEntry,
    tape: SemanticTape<'tcx>,
}

fn process_exact_token_class<'tcx>(cx: &LateContext<'tcx>, token_class: &[&BodyEntry]) {
    let mut candidates = Vec::with_capacity(token_class.len());

    for &entry in token_class {
        let Some(tape) = semantic_tape(cx, entry) else {
            continue;
        };

        candidates.push(SemanticCandidate { entry, tape });
    }

    if candidates.len() < 2 {
        return;
    }

    let mut classes: Vec<Vec<usize>> = Vec::new();

    for candidate_index in 0..candidates.len() {
        let mut matching_class = None;

        for (class_index, class) in classes.iter().enumerate() {
            let representative_index = class[0];

            let left = &candidates[representative_index];
            let right = &candidates[candidate_index];

            if left.tape == right.tape && function_contract_eq(cx, left.entry, right.entry) {
                matching_class = Some(class_index);
                break;
            }
        }

        if let Some(class_index) = matching_class {
            classes[class_index].push(candidate_index);
        } else {
            classes.push(vec![candidate_index]);
        }
    }

    for class in classes {
        if class.len() < 2 {
            continue;
        }

        let mut entries = class
            .into_iter()
            .map(|index| candidates[index].entry)
            .collect::<Vec<_>>();

        entries.sort_unstable_by_key(|entry| (entry.function_span.lo(), entry.function_span.hi()));

        emit_duplicate_class(cx, &entries);
    }
}

fn emit_duplicate_class(cx: &LateContext<'_>, class: &[&BodyEntry]) {
    debug_assert!(class.len() >= 2);

    for entry in class {
        if let Some(expectation) = cx
            .tcx
            .lint_level_spec_at_node(DUPLICATE_FN_BODIES, entry.hir_id)
            .lint_id()
        {
            cx.fulfill_expectation(expectation);
        }
    }

    let primary = class[0];

    for duplicate in &class[1..] {
        span_lint_hir_and_then(
            cx,
            DUPLICATE_FN_BODIES,
            duplicate.hir_id,
            duplicate.function_span,
            "this function has the same body as another function",
            |diag| {
                diag.span_note(primary.function_span, "identical function body is defined here");
                diag.help("consider sharing the implementation if the duplication is unintentional");
            },
        );
    }
}

fn token_key(cx: &LateContext<'_>, span: Span) -> Option<TokenKey> {
    let source = cx.sess().source_map().span_to_snippet(span).ok()?;

    let mut hasher = FxHasher::default();
    let mut token_count = 0_u32;

    for (kind, token, _) in tokenize_with_text(&source) {
        if is_ignored_token(kind) {
            continue;
        }

        mem::discriminant(&kind).hash(&mut hasher);
        token.hash(&mut hasher);

        token_count = token_count.saturating_add(1);
    }

    Some(TokenKey {
        token_count,
        hash: hasher.finish(),
    })
}

fn token_stream_eq(cx: &LateContext<'_>, left: Span, right: Span) -> bool {
    let Some(left_source) = cx.sess().source_map().span_to_snippet(left).ok() else {
        return false;
    };

    let Some(right_source) = cx.sess().source_map().span_to_snippet(right).ok() else {
        return false;
    };

    let left_tokens = tokenize_with_text(&left_source)
        .filter(|(kind, _, _)| !is_ignored_token(*kind))
        .map(|(kind, text, _)| (kind, text));

    let right_tokens = tokenize_with_text(&right_source)
        .filter(|(kind, _, _)| !is_ignored_token(*kind))
        .map(|(kind, text, _)| (kind, text));

    left_tokens.eq(right_tokens)
}

#[inline]
fn is_ignored_token(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Whitespace | TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
    )
}

fn function_contract_eq(cx: &LateContext<'_>, left: &BodyEntry, right: &BodyEntry) -> bool {
    if left.is_const != right.is_const || left.is_async != right.is_async {
        return false;
    }

    let left_typeck = cx.tcx.typeck_body(left.body_id);
    let right_typeck = cx.tcx.typeck_body(right.body_id);

    let left_liberated = left_typeck.liberated_fn_sigs().get(left.hir_id).copied();

    let right_liberated = right_typeck.liberated_fn_sigs().get(right.hir_id).copied();

    if let (Some(left_sig), Some(right_sig)) = (left_liberated, right_liberated) {
        return cx.tcx.erase_and_anonymize_regions(left_sig) == cx.tcx.erase_and_anonymize_regions(right_sig);
    }

    // Conservative fallback.
    let left_sig = cx
        .tcx
        .erase_and_anonymize_regions(cx.tcx.fn_sig(left.def_id).instantiate_identity());

    let right_sig = cx
        .tcx
        .erase_and_anonymize_regions(cx.tcx.fn_sig(right.def_id).instantiate_identity());

    left_sig == right_sig
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SemanticTape<'tcx> {
    atoms: Vec<SemanticAtom<'tcx>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SemanticAtom<'tcx> {
    Resolution(ResolutionKey),
    TypeDependent { kind: DefKind, def_id: DefId },
    GenericArgs(ty::GenericArgsRef<'tcx>),
    FieldIndex(FieldIdx),
    FieldBaseType(TyCtxt<'tcx>),
    Adjustment(AdjustmentKey<'tcx>),
    PatAdjustment(PatAdjustmentKey<'tcx>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ResolutionKey {
    Local(u32),
    Other(Res),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AdjustmentKey<'tcx> {
    kind: AdjustmentKindKey,
    target: TyCtxt<'tcx>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AdjustmentKindKey {
    NeverToAny,

    DerefBuiltin,

    DerefOverloaded {
        mutability: rustc_hir::Mutability,
        method: DefId,
    },

    DerefPin,

    BorrowRef {
        mutability: rustc_hir::Mutability,
        two_phase: bool,
    },

    BorrowRaw(rustc_hir::Mutability),

    BorrowPin(rustc_hir::Mutability),

    Pointer(PointerCoercion),

    GenericReborrow(rustc_hir::Mutability),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PatAdjustmentKey<'tcx> {
    kind: PatAdjustmentKindKey,
    source: TyCtxt<'tcx>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum PatAdjustmentKindKey {
    BuiltinDeref,
    OverloadedDeref,
    PinDeref,
}

fn semantic_tape<'tcx>(cx: &LateContext<'tcx>, entry: &BodyEntry) -> Option<SemanticTape<'tcx>> {
    let typeck = cx.tcx.typeck_body(entry.body_id);

    if typeck.tainted_by_errors.is_some() {
        return None;
    }

    let body = cx.tcx.hir_body(entry.body_id);

    let mut visitor = SemanticVisitor {
        cx,
        typeck,
        root_def_id: entry.def_id,
        atoms: Vec::new(),
        locals: FxHashMap::default(),
        next_local: 0,
        unknown: false,
    };

    visitor.visit_body(body);

    if visitor.unknown {
        None
    } else {
        Some(SemanticTape { atoms: visitor.atoms })
    }
}

struct SemanticVisitor<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    typeck: &'tcx ty::TypeckResults<'tcx>,
    root_def_id: LocalDefId,

    atoms: Vec<SemanticAtom<'tcx>>,

    locals: FxHashMap<HirId, u32>,
    next_local: u32,

    unknown: bool,
}

impl<'tcx> SemanticVisitor<'_, 'tcx> {
    #[inline]
    fn owns(&mut self, hir_id: HirId) -> bool {
        if hir_id.owner == self.typeck.hir_owner {
            true
        } else {
            self.unknown = true;
            false
        }
    }

    fn register_local(&mut self, hir_id: HirId) {
        if self.locals.contains_key(&hir_id) {
            return;
        }

        let ordinal = self.next_local;
        self.next_local = self.next_local.saturating_add(1);
        self.locals.insert(hir_id, ordinal);
    }

    fn push_resolution(&mut self, resolution: Res) {
        if self.unknown {
            return;
        }

        let key = match resolution {
            Res::Err => {
                self.unknown = true;
                return;
            },

            Res::Local(hir_id) => {
                let Some(&ordinal) = self.locals.get(&hir_id) else {
                    self.unknown = true;
                    return;
                };

                ResolutionKey::Local(ordinal)
            },

            other => ResolutionKey::Other(other),
        };

        self.atoms.push(SemanticAtom::Resolution(key));
    }

    fn push_qpath_resolution(&mut self, qpath: &QPath<'tcx>, hir_id: HirId) {
        if !self.owns(hir_id) {
            return;
        }

        if matches!(qpath, QPath::TypeRelative(..)) {
            let resolution = self.typeck.qpath_res(qpath, hir_id);
            self.push_resolution(resolution);
        }
    }

    fn record_node(&mut self, hir_id: HirId) {
        if self.unknown || !self.owns(hir_id) {
            return;
        }

        if let Some((kind, def_id)) = self.typeck.type_dependent_def(hir_id) {
            self.atoms.push(SemanticAtom::TypeDependent { kind, def_id });
        }

        if let Some(args) = self.typeck.node_args_opt(hir_id) {
            let args = self.cx.tcx.erase_and_anonymize_regions(args);
            self.atoms.push(SemanticAtom::GenericArgs(args));
        }

        if let Some(field_index) = self.typeck.opt_field_index(hir_id) {
            self.atoms.push(SemanticAtom::FieldIndex(field_index));
        }

        if self.typeck.splatted_def(hir_id).is_some() {
            self.unknown = true;
        }
    }

    fn record_expr_adjustments(&mut self, expr: &Expr<'tcx>) {
        if self.unknown || !self.owns(expr.hir_id) {
            return;
        }

        for adjustment in self.typeck.expr_adjustments(expr) {
            let target = self.cx.tcx.erase_and_anonymize_regions(adjustment.target);

            let kind = match &adjustment.kind {
                Adjust::NeverToAny => AdjustmentKindKey::NeverToAny,

                Adjust::Deref(DerefAdjustKind::Builtin) => AdjustmentKindKey::DerefBuiltin,

                Adjust::Deref(DerefAdjustKind::Overloaded(overloaded)) => AdjustmentKindKey::DerefOverloaded {
                    mutability: overloaded.mutbl,
                    method: overloaded.method_call(self.cx.tcx),
                },

                Adjust::Deref(DerefAdjustKind::Pin) => AdjustmentKindKey::DerefPin,

                Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) => AdjustmentKindKey::BorrowRef {
                    mutability: rustc_hir::Mutability::Not,
                    two_phase: false,
                },

                Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Mut { allow_two_phase_borrow })) => {
                    AdjustmentKindKey::BorrowRef {
                        mutability: rustc_hir::Mutability::Mut,
                        two_phase: matches!(allow_two_phase_borrow, AllowTwoPhase::Yes),
                    }
                },

                Adjust::Borrow(AutoBorrow::RawPtr(mutability)) => AdjustmentKindKey::BorrowRaw(*mutability),

                Adjust::Borrow(AutoBorrow::Pin(mutability)) => AdjustmentKindKey::BorrowPin(*mutability),

                Adjust::Pointer(pointer) => AdjustmentKindKey::Pointer(*pointer),

                Adjust::GenericReborrow(mutability) => AdjustmentKindKey::GenericReborrow(*mutability),
            };

            self.atoms
                .push(SemanticAtom::Adjustment(AdjustmentKey { kind, target }));
        }
    }

    fn record_pat_adjustments(&mut self, pat: &Pat<'tcx>) {
        if self.unknown || !self.owns(pat.hir_id) {
            return;
        }

        let Some(adjustments) = self.typeck.pat_adjustments().get(pat.hir_id) else {
            return;
        };

        for adjustment in adjustments {
            let kind = match adjustment.kind {
                PatAdjust::BuiltinDeref => PatAdjustmentKindKey::BuiltinDeref,
                PatAdjust::OverloadedDeref => PatAdjustmentKindKey::OverloadedDeref,
                PatAdjust::PinDeref => PatAdjustmentKindKey::PinDeref,
            };

            let source = self.cx.tcx.erase_and_anonymize_regions(adjustment.source);

            self.atoms
                .push(SemanticAtom::PatAdjustment(PatAdjustmentKey { kind, source }));
        }
    }
}

impl<'tcx> Visitor<'tcx> for SemanticVisitor<'_, 'tcx> {
    type NestedFilter = nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_stmt(&mut self, stmt: &'tcx Stmt<'tcx>) {
        if self.unknown {
            return;
        }

        if matches!(stmt.kind, StmtKind::Item(_)) {
            self.unknown = true;
            return;
        }

        intravisit::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if self.unknown {
            return;
        }

        if !self.owns(expr.hir_id) {
            return;
        }

        self.record_node(expr.hir_id);

        match &expr.kind {
            ExprKind::Path(qpath) => {
                self.push_qpath_resolution(qpath, expr.hir_id);
            },

            ExprKind::Field(base, _) => {
                if !self.owns(base.hir_id) {
                    return;
                }

                let base_ty = self
                    .cx
                    .tcx
                    .erase_and_anonymize_regions(self.typeck.expr_ty_adjusted(base));

                self.atoms.push(SemanticAtom::FieldBaseType(base_ty));
            },

            ExprKind::ConstBlock(_) | ExprKind::InlineAsm(_) | ExprKind::OffsetOf(..) | ExprKind::Err(_) => {
                self.unknown = true;
                return;
            },

            _ => {},
        }

        self.record_expr_adjustments(expr);

        if self.unknown {
            return;
        }

        intravisit::walk_expr(self, expr);
    }

    fn visit_pat(&mut self, pat: &'tcx Pat<'tcx>) {
        if self.unknown {
            return;
        }

        if !self.owns(pat.hir_id) {
            return;
        }

        if let PatKind::Binding(_, binding_id, _, _) = pat.kind {
            self.register_local(binding_id);
        }

        self.record_node(pat.hir_id);

        match &pat.kind {
            PatKind::Struct(qpath, ..) | PatKind::TupleStruct(qpath, ..) => {
                self.push_qpath_resolution(qpath, pat.hir_id);
            },

            PatKind::Expr(pat_expr) => {
                if let PatExprKind::Path(qpath) = &pat_expr.kind {
                    self.push_qpath_resolution(qpath, pat.hir_id);
                }
            },

            PatKind::Missing | PatKind::Err(_) => {
                self.unknown = true;
                return;
            },

            _ => {},
        }

        self.record_pat_adjustments(pat);

        if self.unknown {
            return;
        }

        intravisit::walk_pat(self, pat);
    }

    fn visit_ty(&mut self, ty: &'tcx Ty<'tcx, AmbigArg>) {
        if self.unknown {
            return;
        }

        if !self.owns(ty.hir_id) {
            return;
        }

        self.record_node(ty.hir_id);

        match &ty.kind {
            TyKind::Path(qpath) => {
                self.push_qpath_resolution(qpath, ty.hir_id);
            },

            TyKind::Err(_) => {
                self.unknown = true;
                return;
            },

            _ => {},
        }

        if self.unknown {
            return;
        }

        intravisit::walk_ty(self, ty);
    }

    fn visit_path(&mut self, path: &Path<'tcx>, _hir_id: HirId) {
        if self.unknown {
            return;
        }

        self.push_resolution(path.res);

        if self.unknown {
            return;
        }

        intravisit::walk_path(self, path);
    }

    fn visit_path_segment(&mut self, segment: &'tcx PathSegment<'tcx>) {
        if self.unknown {
            return;
        }

        self.record_node(segment.hir_id);

        if self.unknown {
            return;
        }

        intravisit::walk_path_segment(self, segment);
    }

    fn visit_fn(&mut self, kind: FnKind<'tcx>, _: &'tcx FnDecl<'tcx>, body_id: BodyId, _: Span, _: LocalDefId) {
        if self.unknown {
            return;
        }

        if !matches!(kind, FnKind::Closure) {
            self.unknown = true;
            return;
        }

        self.visit_nested_body(body_id);
    }

    fn visit_nested_body(&mut self, body_id: BodyId) {
        if self.unknown {
            return;
        }

        let owner = self.cx.tcx.hir_body_owner_def_id(body_id);

        let typeck_root = self.cx.tcx.typeck_root_def_id(owner.to_def_id()).expect_local();

        if typeck_root != self.root_def_id {
            self.unknown = true;
            return;
        }

        let body = self.cx.tcx.hir_body(body_id);
        self.visit_body(body);
    }
}
