use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::{match_def_path, paths};
use rustc_hir::def_id::DefId;
use rustc_hir::{AsyncGeneratorKind, Body, BodyId, GeneratorKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::GeneratorInteriorTypeCause;
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to await while holding a
    /// non-async-aware MutexGuard.
    ///
    /// ### Why is this bad?
    /// The Mutex types found in std::sync and parking_lot
    /// are not designed to operate in an async context across await points.
    ///
    /// There are two potential solutions. One is to use an async-aware Mutex
    /// type. Many asynchronous foundation crates provide such a Mutex type. The
    /// other solution is to ensure the mutex is unlocked before calling await,
    /// either by introducing a scope or an explicit call to Drop::drop.
    ///
    /// ### Known problems
    /// Will report false positive for explicitly dropped guards ([#6446](https://github.com/rust-lang/rust-clippy/issues/6446)).
    ///
    /// ### Example
    /// ```rust,ignore
    /// use std::sync::Mutex;
    ///
    /// async fn foo(x: &Mutex<u32>) {
    ///   let guard = x.lock().unwrap();
    ///   *guard += 1;
    ///   bar.await;
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// use std::sync::Mutex;
    ///
    /// async fn foo(x: &Mutex<u32>) {
    ///   {
    ///     let guard = x.lock().unwrap();
    ///     *guard += 1;
    ///   }
    ///   bar.await;
    /// }
    /// ```
    #[clippy::version = "1.45.0"]
    pub AWAIT_HOLDING_LOCK,
    pedantic,
    "Inside an async function, holding a MutexGuard while calling await"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to await while holding a
    /// `RefCell` `Ref` or `RefMut`.
    ///
    /// ### Why is this bad?
    /// `RefCell` refs only check for exclusive mutable access
    /// at runtime. Holding onto a `RefCell` ref across an `await` suspension point
    /// risks panics from a mutable ref shared while other refs are outstanding.
    ///
    /// ### Known problems
    /// Will report false positive for explicitly dropped refs ([#6353](https://github.com/rust-lang/rust-clippy/issues/6353)).
    ///
    /// ### Example
    /// ```rust,ignore
    /// use std::cell::RefCell;
    ///
    /// async fn foo(x: &RefCell<u32>) {
    ///   let mut y = x.borrow_mut();
    ///   *y += 1;
    ///   bar.await;
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// use std::cell::RefCell;
    ///
    /// async fn foo(x: &RefCell<u32>) {
    ///   {
    ///      let mut y = x.borrow_mut();
    ///      *y += 1;
    ///   }
    ///   bar.await;
    /// }
    /// ```
    #[clippy::version = "1.49.0"]
    pub AWAIT_HOLDING_REFCELL_REF,
    pedantic,
    "Inside an async function, holding a RefCell ref while calling await"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to await while holding a
    /// `tracing` span's `Entered` or `EnteredSpan` guards.
    ///
    /// ### Why is this bad?
    /// The guards created by `tracing::Span::enter()` or `tracing::Span::entered()`
    /// across `.await` points will result in incorrect traces. This occurs when
    /// an async function or async block yields at an .await point, the current
    /// scope is exited, but values in that scope are not dropped (because
    /// the async block will eventually resume execution from that await point).
    /// This means that another task will begin executing while remaining in the entered span.
    ///
    /// ### Known problems
    /// Will report false positive for explicitly dropped refs ([#6353](https://github.com/rust-lang/rust-clippy/issues/6353)).
    ///
    /// ### Example
    /// ```rust,ignore
    /// use tracing::{span, Level};
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///
    ///     THIS WILL RESULT IN INCORRECT TRACES
    ///     let _enter = span.enter();
    ///     bar().await;
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// use tracing::{span, Level}
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///
    ///     let some_value = span.in_scope(|| {
    ///         // run some synchronous code inside the span...
    ///     });
    ///
    ///     // This is okay! The span has already been exited before we reach
    ///     // the await point.
    ///     bar(some_value).await;
    /// }
    /// ```
    ///
    /// Or use:
    ///
    /// ```rust,ignore
    /// use tracing::{span, Level, Instrument};
    ///
    /// async fn foo() {
    ///     let span = span!(Level::INFO, "foo");
    ///     async move {
    ///         // This is correct! If we yield here, the span will be exited,
    ///         // and re-entered when we resume.
    ///         bar().await;
    ///     }.instrument(span) // instrument the async block with the span...
    ///     .await // ...and await it.
    /// }
    /// ```
    #[clippy::version = "1.60.0"]
    pub AWAIT_HOLDING_SPAN_GUARD,
    pedantic,
    "Inside an async function, holding a Span guard while calling await"
}

declare_lint_pass!(AwaitHolding => [AWAIT_HOLDING_LOCK, AWAIT_HOLDING_REFCELL_REF, AWAIT_HOLDING_SPAN_GUARD]);

impl LateLintPass<'_> for AwaitHolding {
    fn check_body(&mut self, cx: &LateContext<'_>, body: &'_ Body<'_>) {
        use AsyncGeneratorKind::{Block, Closure, Fn};
        if let Some(GeneratorKind::Async(Block | Closure | Fn)) = body.generator_kind {
            let body_id = BodyId {
                hir_id: body.value.hir_id,
            };
            let typeck_results = cx.tcx.typeck_body(body_id);
            check_interior_types(
                cx,
                typeck_results.generator_interior_types.as_ref().skip_binder(),
                body.value.span,
            );
        }
    }
}

fn check_interior_types(cx: &LateContext<'_>, ty_causes: &[GeneratorInteriorTypeCause<'_>], span: Span) {
    for ty_cause in ty_causes {
        if let rustc_middle::ty::Adt(adt, _) = ty_cause.ty.kind() {
            if is_mutex_guard(cx, adt.did) {
                span_lint_and_note(
                    cx,
                    AWAIT_HOLDING_LOCK,
                    ty_cause.span,
                    "this MutexGuard is held across an 'await' point. Consider using an async-aware Mutex type or ensuring the MutexGuard is dropped before calling await",
                    ty_cause.scope_span.or(Some(span)),
                    "these are all the await points this lock is held through",
                );
            }
            if is_refcell_ref(cx, adt.did) {
                span_lint_and_note(
                    cx,
                    AWAIT_HOLDING_REFCELL_REF,
                    ty_cause.span,
                    "this RefCell Ref is held across an 'await' point. Consider ensuring the Ref is dropped before calling await",
                    ty_cause.scope_span.or(Some(span)),
                    "these are all the await points this ref is held through",
                );
            }
            if is_tracing_span_guard(cx, adt.did) {
                span_lint_and_note(
                    cx,
                    AWAIT_HOLDING_SPAN_GUARD,
                    ty_cause.span,
                    "this Span guard is held across an 'await' point. Consider using the `.instrument()` combinator or the `.in_scope()` method instead",
                    ty_cause.scope_span.or(Some(span)),
                    "these are all the await points this ref is held through",
                );
            }
        }
    }
}

fn is_mutex_guard(cx: &LateContext<'_>, def_id: DefId) -> bool {
    match_def_path(cx, def_id, &paths::MUTEX_GUARD)
        || match_def_path(cx, def_id, &paths::RWLOCK_READ_GUARD)
        || match_def_path(cx, def_id, &paths::RWLOCK_WRITE_GUARD)
        || match_def_path(cx, def_id, &paths::PARKING_LOT_MUTEX_GUARD)
        || match_def_path(cx, def_id, &paths::PARKING_LOT_RWLOCK_READ_GUARD)
        || match_def_path(cx, def_id, &paths::PARKING_LOT_RWLOCK_WRITE_GUARD)
}

fn is_refcell_ref(cx: &LateContext<'_>, def_id: DefId) -> bool {
    match_def_path(cx, def_id, &paths::REFCELL_REF) || match_def_path(cx, def_id, &paths::REFCELL_REFMUT)
}

fn is_tracing_span_guard(cx: &LateContext<'_>, def_id: DefId) -> bool {
    match_def_path(cx, def_id, &paths::TRACING_SPAN_ENTER_GUARD)
        || match_def_path(cx, def_id, &paths::TRACING_SPAN_ENTERED_GUARD)
}
