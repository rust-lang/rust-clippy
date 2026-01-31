//! Utilities for interacting with the source text and manipulating spans.
//!
//! The main entry points for working with the source text are on the [`SpanExt`] trait. This trait
//! is implemented on a few types and exists as a bridge between a [`Span`] and the source text
//! backing it. The following are the main functions:
//!
//! * [`SpanExt::mk_edit_cx`]: This is the most general method interacting with the source text and
//!   the other methods should be preferred when possible. This is useful for handling multiple
//!   sub-spans (e.g. multiple items in a list), or when checking a predicate on the text
//!   surrounding a span.
//! * [`SpanExt::get_text`]: Gets a `SourceText` representing the text the span refers to. It works
//!   very similarly to an `Arc<str>`. This is normally used when building a suggestion.
//! * [`SpanExt::get_text_within_other`]: Like `get_text`, but first adjusts the context to match
//!   another span. This is needed when the node this span is from might be created by a macro.
//! * [`SpanExt::check_text`]: A slightly simpler way to check a predicate on the text than using
//!   `get_text`. Prefer this if possible.
//! * [`SpanExt::map_range`]: The main way to adjust the range portion of a span or to split a
//!   single span. See [`SpanEditCx`] for range adjustment utilities.
//! * [`SpanExt::map_split_range`]: Similar to the previous, but for when you need to create
//!   multiple spans from a single span.
//! * [`SpanExt::get_external_text`] and [`SpanExt::check_external_text`]: Like their non-external
//!   counterparts, but will also access the source text of external crates. Avoid these if at all
//!   possible.
//!
//! # Working With Macros
//!
//! All parts of the linting infrastructure work after macro expansion meaning that there are no
//! nodes in the AST or HIR representing a macro call (AST nodes have a variant for these, but it's
//! only used before expansion). Because of this there's no direct way of knowing where a macro call
//! occurred. Instead the `Span`, or more specifically the [`SyntaxContext`], contains which
//! expansion created the node.
//!
//! Each `SyntaxContext` is associated with a particular macro call or AST desugaring (e.g. a `for`
//! loop). These expansions form a tree starting from a singular root context with each macro call
//! and desugaring creating a new child node. As an example:
//!
//! ```rust
//! macro_rules! m1 {
//!   ($e1:expr) => { $e1 + 1 };
//! }
//! macro_rules! m2 {
//!   ($e2:expr) => {{ m1!($e2) + m1!($e2) }};
//! }
//! let x = m2!(1u32);
//! let y = m1!(1u32);
//! ```
//!
//! This would create the following expansions:
//!
//! ```none
//!     root
//!    /    \
//!   m2    m1
//!  /  \
//! m1  m1
//! ```
//!
//! When accessing the text for the initializer of `x`, just naively using the span for the
//! expression will get the contents of the `m2` macro (`m1!($e2) + m1!($e2)`). Similarly, the span
//! for the left-hand side expression of the addition in `m1` (`$e1 + 1`) will be the span of the
//! argument at the call site. In both cases [`SpanExt::get_text_within_other`] can be used to get
//! the span of the macro call (`m2!(1u32)`) or the metavar expansion (`$e1`) respectively.
//!
//! # Warnings
//!
//! You _cannot_ assume anything about the `Span` or source text of any item. The parser will apply
//! token substitution in some cases (e.g. replacing `（`, with `(`), macros can rearrange tokens,
//! and proc-macros in particular can freely set the `Span` of any token to a different one. These
//! can only be detected by checking the source text. With this the source text of all AST/HIR item
//! can be almost anything. In short, validate all range adjustments against the source text.

use core::fmt;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::{Deref, Index, Range, RangeFrom, RangeFull, RangeTo};
use core::slice::SliceIndex;
use core::str::pattern::{Pattern, ReverseSearcher};
use rustc_ast::{LitKind, StrStyle};
use rustc_errors::Applicability;
use rustc_hir::def_id::LocalDefId;
use rustc_hir::{BlockCheckMode, Expr, ExprKind, UnsafeSource};
use rustc_lexer::{FrontmatterAllowed, LiteralKind, TokenKind, tokenize};
use rustc_lint::{EarlyContext, LateContext, LintContext};
use rustc_middle::ty::TyCtxt;
use rustc_session::Session;
use rustc_span::source_map::{SourceMap, original_sp};
use rustc_span::{
    BytePos, DUMMY_SP, DesugaringKind, ExpnKind, Pos, RelativeBytePos, SourceFile, SourceFileAndLine, Span, SpanData,
    SyntaxContext, hygiene, with_metavar_spans,
};
use std::borrow::{Borrow, Cow};

/// A type which contains a `SourceMap`.
pub trait HasSourceMap<'sm>: Copy {
    #[must_use]
    fn source_map(self) -> &'sm SourceMap;
}
impl<'sm> HasSourceMap<'sm> for &'sm SourceMap {
    #[inline]
    fn source_map(self) -> &'sm SourceMap {
        self
    }
}
impl<'sm> HasSourceMap<'sm> for &'sm Session {
    #[inline]
    fn source_map(self) -> &'sm SourceMap {
        self.source_map()
    }
}
impl<'sm> HasSourceMap<'sm> for TyCtxt<'sm> {
    #[inline]
    fn source_map(self) -> &'sm SourceMap {
        self.sess.source_map()
    }
}
impl<'sm> HasSourceMap<'sm> for &'sm EarlyContext<'_> {
    #[inline]
    fn source_map(self) -> &'sm SourceMap {
        self.sess().source_map()
    }
}
impl<'sm> HasSourceMap<'sm> for &LateContext<'sm> {
    #[inline]
    fn source_map(self) -> &'sm SourceMap {
        self.tcx.sess.source_map()
    }
}

/// A position in the `SourceMap` and the `SyntaxContext` it came from.
#[derive(Clone, Copy)]
pub struct PosWithCtxt {
    pub pos: BytePos,
    pub ctxt: SyntaxContext,
}

// Workaround for `array::map` not working with `track_caller`. This will fail to drop values if a
// panic occurs, but that doesn't matter for our use.
const fn mk_uninit<T, U, const N: usize>(_: &[T; N]) -> [MaybeUninit<U>; N] {
    [const { MaybeUninit::uninit() }; N]
}
macro_rules! map_array_inline {
    ($a:expr, $p:pat => $e:expr $(,)?) => {{
        let src = ManuallyDrop::new($a);
        let mut dst = mk_uninit(&src);
        for i in 0..dst.len() {
            // SAFETY: `src` was fully initialized at the start of the loop.
            let $p = unsafe { (&raw const src[i]).read() };
            dst[i].write($e);
        }
        // SAFETY: The loop has written every element in the array.
        unsafe { MaybeUninit::array_assume_init(dst) }
    }};
}

pub trait HasSpanData: Copy {
    #[must_use]
    fn data(self) -> SpanData;
}
impl HasSpanData for SpanData {
    #[inline]
    fn data(self) -> SpanData {
        self
    }
}
impl HasSpanData for &SpanData {
    #[inline]
    fn data(self) -> SpanData {
        *self
    }
}
impl HasSpanData for Span {
    #[inline]
    fn data(self) -> SpanData {
        self.data()
    }
}

pub trait CompressedSpan: Copy {
    #[must_use]
    fn span(self) -> Span;
}
impl CompressedSpan for Span {
    #[inline]
    fn span(self) -> Span {
        self
    }
}

pub trait SpanExt: Sized {
    #[must_use]
    fn span_range(self) -> Range<BytePos>;

    /// Gets the `lo` position and the `SyntaxContext`
    #[inline]
    #[must_use]
    fn lo_ctxt(self) -> PosWithCtxt
    where
        Self: HasSpanData,
    {
        let data = self.data();
        PosWithCtxt {
            pos: data.lo,
            ctxt: data.ctxt,
        }
    }

    /// Gets the `hi` position and the `SyntaxContext`
    #[inline]
    #[must_use]
    fn hi_ctxt(self) -> PosWithCtxt
    where
        Self: HasSpanData,
    {
        let data = self.data();
        PosWithCtxt {
            pos: data.hi,
            ctxt: data.ctxt,
        }
    }

    /// Attempts to get a handle to the source text of a crate-local file. Returns `None` if the
    /// range is within a non-local file or cannot index the file's text.
    ///
    /// With debug assertions this will assert that the range:
    /// * Is within a crate-local file.
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_text<'sm>(self, sm: impl HasSourceMap<'sm>) -> Option<SourceText> {
        // TODO(@Jarcho): Actually check all use sites so we can use `for_local`
        // like the documentation says.
        self.get_external_text(sm)
    }

    /// Attempts to get a handle to the source text. Returns `None` if an external file could not be
    /// loaded or the range cannot index the file's text.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_external_text<'sm>(self, sm: impl HasSourceMap<'sm>) -> Option<SourceText> {
        SourceText::for_external_range(sm.source_map(), self.span_range())
    }

    /// Checks if the source text of a crate-local file satisfies the given predicate. Returns
    /// `false` if the range is within a non-local file or cannot index the file's text.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn check_text<'sm>(self, sm: impl HasSourceMap<'sm>, pred: impl FnOnce(&str) -> bool) -> bool {
        // TODO(@Jarcho): Actually check all use sites so we can use `get_text` like the
        // documentation says.
        self.get_external_text(sm).as_deref().is_some_and(pred)
    }

    /// Checks if the source text satisfies the given predicate. Returns `false` if the range cannot
    /// index the file's text.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn check_external_text<'sm>(self, sm: impl HasSourceMap<'sm>, pred: impl FnOnce(&str) -> bool) -> bool {
        self.get_external_text(sm).as_deref().is_some_and(pred)
    }

    /// Walks this span into the context of another; either up to the call site or down to the
    /// metavar expansion site. Returns `None` if either the target context could be reached or the
    /// adjusted span does not lie within the target span.
    ///
    /// # Examples
    ///
    /// Given the following code:
    ///
    /// ```rust,ignore
    /// macro_rules! m1 { ($e1:expr) => { f1($e1) }; }
    /// macro_rules! m2 { ($e2:expr) => { f2(m1!($e2)) }; }
    /// f3(m2!(0))
    /// ```
    ///
    /// This expands to `f3(f2(f1(0)))` with the following `SyntaxContext`s:
    ///
    /// |Context     |Contents    |
    /// |------------|------------|
    /// |Root context|`f3(_)`, `0`|
    /// |`m2!`       |`f2(_)`     |
    /// |`m1!`       |`f1(_)`     |
    ///
    /// The following table lists the results of various possible argument combinations:
    ///
    /// |Span   |Context     |Result  |
    /// |-------|------------|--------|
    /// |`f3(_)`|Root        |`f3(_)` |
    /// |`f3(_)`|`m1!`, `m2!`|None    |
    /// |`f2(_)`|Root        |`m2!(0)`|
    /// |`f2(_)`|`m2!`       |`f2(_)` |
    /// |`f2(_)`|`m1!`       |None    |
    /// |`f1(_)`|Root        |`m2!(0)`|
    /// |`f1(_)`|`m2!`       |`m1!(0)`|
    /// |`f1(_)`|`m1!`       |`f1(_)` |
    /// |`0`    |Root        |`0`     |
    /// |`0`    |`m2!`       |`$e2`   |
    /// |`0`    |`m1!`       |`$e1`   |
    #[inline]
    #[must_use]
    fn walk_into_other(self, target: &SpanData) -> Option<SpanData>
    where
        Self: CompressedSpan,
    {
        fn f(sp: Span, target: &SpanData) -> Option<SpanData> {
            let mut sp_data = sp.data();
            if sp_data.ctxt != target.ctxt {
                let expn = sp_data.ctxt.outer_expn_data();
                let call_site = expn.call_site.data();
                if call_site.ctxt != target.ctxt {
                    sp_data = hygiene::walk_chain(expn.call_site, target.ctxt).data();
                    if sp_data.ctxt != target.ctxt {
                        sp_data = with_metavar_spans(|mspans| mspans.get(sp))?.data();
                        if sp_data.ctxt != target.ctxt {
                            return None;
                        }
                    }
                } else if matches!(expn.kind, ExpnKind::Desugaring(DesugaringKind::RangeExpr)) {
                    // The call site of a range desugaring does not include any surrounding parenthesis.
                    // e.g. `(1..2)` will have `1..2` as the call site, but `(1..2)` as the desugaring
                    // span.
                    //
                    // Currently some uses are assuming that we will keep the surrounding parenthesis so
                    // we can't use the call site.
                    sp_data.ctxt = target.ctxt;
                } else {
                    sp_data = expn.call_site.data();
                }
            }
            (target.lo <= sp_data.lo && sp_data.hi <= target.hi).then_some(sp_data)
        }
        f(self.span(), target)
    }

    /// Walks this span up the expansion call chain into the target context. Returns `None` if the
    /// target context cannot be found this way.
    ///
    /// This is similar to [`SpanExt::walk_into_other`] except it will not walk to a metavar
    /// expansion site, nor will it check the adjusted span's range.
    #[inline]
    #[must_use]
    fn walk_to_parent(self, target: SyntaxContext) -> Option<Span>
    where
        Self: CompressedSpan,
    {
        #[cold]
        #[inline(never)]
        fn slow(sp: Span, sp_ctxt: SyntaxContext, target: SyntaxContext) -> Option<Span> {
            let expn = sp_ctxt.outer_expn_data();
            if expn.call_site.from_expansion() {
                let sp = hygiene::walk_chain(expn.call_site, SyntaxContext::root());
                (sp.ctxt() == target).then_some(sp)
            } else if matches!(expn.kind, ExpnKind::Desugaring(DesugaringKind::RangeExpr)) {
                // The call site of a range desugaring does not include any surrounding parenthesis.
                // e.g. `(1..2)` will have `1..2` as the call site, but `(1..2)` as the desugaring
                // span.
                //
                // Currently some uses are assuming that we will keep the surrounding parenthesis so
                // we can't use the call site.
                Some(sp.with_ctxt(target))
            } else {
                Some(expn.call_site)
            }
        }

        let sp = self.span();
        let sp_ctxt = sp.ctxt();
        if sp_ctxt == target {
            Some(sp)
        } else {
            slow(sp, sp_ctxt, target)
        }
    }

    /// Walks this span up the expansion call chain to the root context.
    #[inline]
    #[must_use]
    fn walk_to_root(self) -> Span
    where
        Self: CompressedSpan,
    {
        #[cold]
        #[inline(never)]
        fn slow(sp: Span) -> Span {
            let expn = sp.ctxt().outer_expn_data();
            if expn.call_site.from_expansion() {
                hygiene::walk_chain(expn.call_site, SyntaxContext::root())
            } else if matches!(expn.kind, ExpnKind::Desugaring(DesugaringKind::RangeExpr)) {
                // The call site of a range desugaring does not include any surrounding parenthesis.
                // e.g. `(1..2)` will have `1..2` as the call site, but `(1..2)` as the desugaring
                // span.
                //
                // Currently some uses are assuming that we will keep the surrounding parenthesis so
                // we can't use the call site.
                sp.with_ctxt(SyntaxContext::root())
            } else {
                expn.call_site
            }
        }

        let sp = self.span();
        if sp.from_expansion() { slow(sp) } else { sp }
    }

    /// Attempts to get a handle to the source text of a crate-local file after adjusting this span
    /// to be in the same context as another. This will return `None` if this span could not be
    /// adjusted to the target's context; or the adjusted span does not lie within the target or
    /// cannot index the text of a single file.
    ///
    /// See `[SpanExt::walk_to_other]` for details about how context adjustment works.
    ///
    /// With debug assertions this will assert that the adjusted range:
    /// * Lies within a single crate-local file.
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_text_within_other<'sm>(self, sm: impl HasSourceMap<'sm>, other: &SpanData) -> Option<SourceText>
    where
        Self: CompressedSpan,
    {
        self.walk_into_other(other).and_then(|sp| sp.get_text(sm))
    }

    /// Attempts to create a new edit context for a source range within a crate-local file. Returns
    /// both the context and the adjusted range, or `None` if the range is within a non-local file.
    ///
    /// With debug assertions this will assert that the range:
    /// * Is within a crate-local file.
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn mk_edit_cx<'sm>(self, sm: impl HasSourceMap<'sm>) -> Option<(SpanEditCx<'sm>, FileRange)>
    where
        Self: HasSpanData,
    {
        SpanEditCx::for_local(sm.source_map(), self.data())
    }

    /// Maps the range of the current span within a crate-local file. Returns `None` if the given
    /// function returns `None`, or the span is within a non-local file.
    ///
    /// With debug assertions this will assert that both the initial and mapped ranges:
    /// * Do not start after their respective ends.
    /// * Do not exceed the bounds of a single source file.
    /// * Lie on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn map_range<'sm>(
        self,
        sm: impl HasSourceMap<'sm>,
        f: impl FnOnce(&SpanEditCx<'sm>, FileRange) -> Option<FileRange>,
    ) -> Option<Span>
    where
        Self: HasSpanData,
    {
        if let Some((scx, range)) = self.mk_edit_cx(sm)
            && let Some(mapped) = f(&scx, range.clone())
        {
            Some(scx.mk_span(mapped, Some(range)))
        } else {
            None
        }
    }

    /// Maps and splits the range of the current span within a crate-local file. Returns `None` if
    /// the given function returns `None`, or the span is within a non-local file.
    ///
    /// With debug assertions this will assert that both the initial and mapped ranges:
    /// * Do not start after their respective ends.
    /// * Do not exceed the bounds of a single source file.
    /// * Lie on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn map_split_range<'sm, const N: usize>(
        self,
        sm: impl HasSourceMap<'sm>,
        f: impl FnOnce(&SpanEditCx<'sm>, FileRange) -> Option<[FileRange; N]>,
    ) -> Option<[Span; N]>
    where
        Self: HasSpanData,
    {
        if let Some((scx, range)) = self.mk_edit_cx(sm)
            && let Some(mapped) = f(&scx, range.clone())
        {
            Some(map_array_inline!(mapped, r => scx.mk_span(r, Some(range.clone()))))
        } else {
            None
        }
    }
}
impl SpanExt for Span {
    #[inline]
    fn span_range(self) -> Range<BytePos> {
        self.data().span_range()
    }
}
impl SpanExt for SpanData {
    #[inline]
    fn span_range(self) -> Range<BytePos> {
        self.lo..self.hi
    }
}
impl SpanExt for Range<BytePos> {
    #[inline]
    fn span_range(self) -> Range<BytePos> {
        self
    }
}

mod source_text {
    use rustc_span::SourceFile;
    use rustc_span::source_map::SourceMap;
    use std::sync::Arc;

    /// Handle to a substring of text in a source file.
    #[derive(Clone)]
    pub struct SourceText {
        file: Arc<SourceFile>,
        // This is a pointer into the text owned by the source file. If the source is external
        // then the `FreezeLock` on the text must be frozen.
        text: *const str,
    }
    impl SourceText {
        /// Gets the text of the given crate-local file. Returns `None` if the file is non-local.
        ///
        /// With debug assertions this will assert that the file is local.
        #[inline]
        #[must_use]
        #[cfg_attr(debug_assertions, track_caller)]
        pub fn for_local_file(file: Arc<SourceFile>) -> Option<Self> {
            let text: *const str = if let Some(text) = &file.src {
                &raw const ***text
            } else {
                debug_assert!(
                    false,
                    "attempted to access the non-local file `{}` as local.",
                    file.name.prefer_local_unconditionally()
                );
                return None;
            };
            Some(Self { file, text })
        }

        /// Gets the text of the given file. Returns `None` if the file's text could not be loaded.
        #[must_use]
        pub fn for_external_file(sm: &SourceMap, file: Arc<SourceFile>) -> Option<Self> {
            let text: *const str = if let Some(text) = &file.src {
                &raw const ***text
            } else if !sm.ensure_source_file_source_present(&file) {
                return None;
            }
            // `get` or `freeze` must be used to ensure the contents of the lock cannot change.
            // Since `ensure_source_file_source_present` calls `freeze` when loading the source
            // we use `get` to avoid the extra load.
            else if let Some(src) = file.external_src.get()
                && let Some(text) = src.get_source()
            {
                text
            } else {
                return None;
            };
            Some(Self { file, text })
        }

        /// Gets the source text.
        #[inline]
        #[must_use]
        pub fn as_str(&self) -> &str {
            // SAFETY: `text` is owned by `sf` and comes from either an `Option<Arc<String>>`, or a
            // frozen `FeezeLock<ExternalSrc>` (which ultimately contains an `Arc<String>`). Neither
            // of these can change so long as we own `sf`.
            unsafe { &*self.text }
        }

        /// Gets the source file containing the text.
        #[inline]
        #[must_use]
        pub fn file(&self) -> &Arc<SourceFile> {
            &self.file
        }

        /// Takes ownership of the source file handle.
        #[inline]
        #[must_use]
        pub fn into_file(self) -> Arc<SourceFile> {
            self.file
        }

        /// Applies the mapping function to the contained string.
        #[inline]
        #[must_use]
        pub fn map_text(mut self, f: impl FnOnce(&SourceText) -> &str) -> Self {
            // The only strings that `f` can return are those with a lifetime derived from it's
            // input, and `'static` strings. Both are safe to use here.
            self.text = f(&self);
            self
        }

        /// Applies the mapping function to the contained string. Returns `None` if the function
        /// does.
        #[inline]
        #[must_use]
        pub fn try_map_text(mut self, f: impl FnOnce(&SourceText) -> Option<&str>) -> Option<Self> {
            // The only strings that `f` can return are those with a lifetime derived from it's
            // input, and `'static` strings. Both are safe to use here.
            match f(&self) {
                Some(s) => {
                    self.text = s;
                    Some(self)
                },
                None => None,
            }
        }
    }
}
pub use self::source_text::SourceText;
impl SourceText {
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn for_external_range(sm: &SourceMap, range: Range<BytePos>) -> Option<Self> {
        let sfp = sm.lookup_byte_offset(range.start);
        let text = Self::for_external_file(sm, sfp.sf)?;
        let range = RelativeBytePos(sfp.pos.0)..RelativeBytePos(range.end.0.wrapping_sub(text.file().start_pos.0));
        dbg_check_range(sm, &text, None, range.clone());
        text.apply_index(range.into_slice_idx())
    }

    /// Converts this into an owned string.
    #[inline]
    #[must_use]
    pub fn to_owned(&self) -> String {
        self.as_str().to_owned()
    }

    /// Applies an indexing operation to the contained string. Returns `None` if the index is
    /// not valid.
    #[inline]
    #[must_use]
    pub fn apply_index(self, idx: impl SliceIndex<str, Output = str>) -> Option<Self> {
        self.try_map_text(|s| s.get(idx))
    }
}
impl Deref for SourceText {
    type Target = str;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}
impl Borrow<str> for SourceText {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<str> for SourceText {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl<T> Index<T> for SourceText
where
    str: Index<T>,
{
    type Output = <str as Index<T>>::Output;
    #[inline]
    fn index(&self, idx: T) -> &Self::Output {
        &self.as_str()[idx]
    }
}
impl fmt::Display for SourceText {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
impl fmt::Debug for SourceText {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

/// Like `SliceIndex`, but for indexing a file's text rather than any string. This uses
/// `RelativeBytePos` instead of `usize`.
pub trait FileIndex {
    type SliceRange: SliceIndex<str, Output = str>;

    #[must_use]
    fn into_slice_idx(self) -> Self::SliceRange;

    /// Converts this into a bounded range by limiting unbounded ends to the file's bounds.
    #[must_use]
    fn into_file_range(self, scx: &SpanEditCx<'_>) -> FileRange;
}
impl FileIndex for RangeFull {
    type SliceRange = Self;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceRange {
        self
    }
    #[inline]
    fn into_file_range(self, scx: &SpanEditCx<'_>) -> FileRange {
        RelativeBytePos(0)..scx.file().normalized_source_len
    }
}
impl FileIndex for Range<RelativeBytePos> {
    type SliceRange = Range<usize>;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceRange {
        self.start.to_usize()..self.end.to_usize()
    }
    #[inline]
    fn into_file_range(self, _: &SpanEditCx<'_>) -> FileRange {
        self
    }
}
impl FileIndex for RangeTo<RelativeBytePos> {
    type SliceRange = RangeTo<usize>;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceRange {
        ..self.end.to_usize()
    }
    #[inline]
    fn into_file_range(self, _: &SpanEditCx<'_>) -> FileRange {
        RelativeBytePos(0)..self.end
    }
}
impl FileIndex for RangeFrom<RelativeBytePos> {
    type SliceRange = RangeFrom<usize>;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceRange {
        self.start.to_usize()..
    }
    #[inline]
    fn into_file_range(self, scx: &SpanEditCx<'_>) -> FileRange {
        self.start..scx.file().normalized_source_len
    }
}

/// The range type used for specifying a range within a file.
pub type FileRange = Range<RelativeBytePos>;

#[cfg_attr(not(debug_assertions), inline)]
#[cfg_attr(debug_assertions, track_caller)]
fn dbg_check_range(sm: &SourceMap, text: &SourceText, old: Option<FileRange>, new: FileRange) {
    #[cfg(debug_assertions)]
    if text.get(new.clone().into_slice_idx()).is_none() {
        use core::fmt::Write;

        let file = &**text.file();
        let mut msg = String::with_capacity(512);
        let _ = write!(
            msg,
            "error: invalid range `{}..{}`: ",
            // Signed numbers will better show most errors.
            new.start.0.cast_signed(),
            new.end.0.cast_signed(),
        );
        if new.start > file.normalized_source_len || new.end > file.normalized_source_len {
            let _ = write!(
                msg,
                "the bounds are outside the current file (len: {})",
                file.normalized_source_len.0,
            );
        } else if new.start > new.end {
            msg.push_str("the start and end overlap");
        } else {
            msg.push_str("the ends are not on UTF-8 boundaries");
        }

        // Attempt to display the new range bounds as line and column positions.
        let new_start = BytePos(new.start.0.wrapping_add(file.start_pos.0));
        let new_end = BytePos(new.end.0.wrapping_add(file.start_pos.0));
        let files_end = sm.files().last().map(|f| f.start_pos.0 + f.normalized_source_len.0);
        let mut print_loc = |label: &str, pos: BytePos| {
            if files_end.is_some_and(|end| pos.0 <= end) {
                let sfp = sm.lookup_byte_offset(pos);
                let file_name = sfp.sf.name.prefer_local_unconditionally();
                if let Some(text) = SourceText::for_external_file(sm, sfp.sf.clone())
                    && text.get(sfp.pos.to_usize()..).is_some()
                {
                    let (line, col, _) = sfp.sf.lookup_file_pos_with_col_display(pos);
                    let _ = write!(msg, "\n  {label}: {file_name}:{line}:{}", col.to_u32());
                } else {
                    let line = sfp.sf.lookup_line(RelativeBytePos(sfp.pos.0)).unwrap_or(0);
                    let offset = sfp.pos.0 - file.lines()[line].0;
                    let _ = write!(msg, "\n  {label}: {file_name}:{} + {}", line + 1, offset);
                }
            } else {
                let _ = write!(msg, "\n  {label}: not a file");
            }
        };
        if old.as_ref().is_none_or(|old| new.start != old.start) {
            print_loc("new start", new_start);
        }
        if old.as_ref().is_none_or(|old| new.end != old.end) {
            print_loc("new end", new_end);
        }

        // We aren't debug checking the old range, only using it to add additional context.
        if let Some(old) = old
            && let Some(old_text) = text.get(old.clone().into_slice_idx())
        {
            let old_start = BytePos(old.start.0 + file.start_pos.0);
            let old_end = BytePos(old.end.0 + file.start_pos.0);
            let (start_line, start_col, _) = file.lookup_file_pos_with_col_display(old_start);
            let (end_line, end_col, _) = file.lookup_file_pos_with_col_display(old_end);
            let _ = write!(
                msg,
                "\n  current: {}:{}:{}: {}:{}",
                file.name.prefer_local_unconditionally(),
                start_line,
                start_col.to_u32(),
                end_line,
                end_col.to_u32(),
            );
            // Display the old text indented.
            msg.extend(old_text.split('\n').flat_map(|x| ["\n  ", x]));
        } else {
            let _ = write!(msg, "\n  current file: {}", file.name.prefer_local_unconditionally());
        }

        std::panic::panic_any(msg);
    };
}

/// The context used to manipulate source ranges within a single file.
pub struct SpanEditCx<'sm> {
    text: SourceText,
    ctxt: SyntaxContext,
    parent: Option<LocalDefId>,

    // Used only to create debug assertion messages.
    #[cfg(debug_assertions)]
    sm: &'sm SourceMap,
    #[cfg(not(debug_assertions))]
    sm: core::marker::PhantomData<&'sm SourceMap>,
}
impl<'sm> SpanEditCx<'sm> {
    /// Creates a new edit context for a span within a single file. Returns `None` if the source
    /// could not be loaded.
    ///
    /// With debug assertions this will validate that the span:
    /// * Is contained within a single file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn for_external(sm: &'sm SourceMap, data: SpanData) -> Option<(Self, FileRange)> {
        let sfp = sm.lookup_byte_offset(data.lo);
        let end = RelativeBytePos(data.hi.0.wrapping_sub(sfp.sf.start_pos.0));

        let scx = Self {
            text: SourceText::for_external_file(sm, sfp.sf)?,
            ctxt: data.ctxt,
            parent: data.parent,

            #[cfg(debug_assertions)]
            sm,
            #[cfg(not(debug_assertions))]
            sm: core::marker::PhantomData,
        };
        scx.dbg_check_range(None, RelativeBytePos(sfp.pos.0)..end);
        Some((scx, RelativeBytePos(sfp.pos.0)..end))
    }

    /// Creates a new edit context for a span within a crate-local file. Returns `None` if span is
    /// within a non-local file.
    ///
    /// With debug assertions this will validate that the span:
    /// * Is within a single crate-local file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn for_local(sm: &'sm SourceMap, data: SpanData) -> Option<(Self, FileRange)> {
        let sfp = sm.lookup_byte_offset(data.lo);
        let end = RelativeBytePos(data.hi.0.wrapping_sub(sfp.sf.start_pos.0));

        let scx = Self {
            text: SourceText::for_local_file(sfp.sf)?,
            ctxt: data.ctxt,
            parent: data.parent,

            #[cfg(debug_assertions)]
            sm,
            #[cfg(not(debug_assertions))]
            sm: core::marker::PhantomData,
        };
        scx.dbg_check_range(None, RelativeBytePos(sfp.pos.0)..end);
        Some((scx, RelativeBytePos(sfp.pos.0)..end))
    }

    /// Converts this into the inner `SourceText`.
    #[inline]
    #[must_use]
    pub fn into_file_text(self) -> SourceText {
        self.text
    }

    /// Converts this into the inner `SourceText` after slicing it. Returns `None` if the text can't
    /// be indexed by the range.
    #[inline]
    #[must_use]
    pub fn into_sliced_text(self, idx: impl FileIndex) -> Option<SourceText> {
        self.text.apply_index(idx.into_slice_idx())
    }

    /// Gets a reference to the contained source file.
    #[inline]
    #[must_use]
    pub fn file(&self) -> &SourceFile {
        self.text.file()
    }

    /// Gets the text of the whole file.
    #[inline]
    #[must_use]
    pub fn file_text(&self) -> &str {
        self.text.as_str()
    }

    /// Gets a subslice of the file's text. Returns `None` if the range is invalid.
    #[inline]
    #[must_use]
    pub fn get_text(&self, index: impl FileIndex) -> Option<&str> {
        self.text.as_str().get(index.into_slice_idx())
    }

    /// Gets a subslice of the file's text. Returns `None` if the span is invalid.
    ///
    /// With debug assertions this will validate that the span:
    /// * Is from the same syntax context.
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    pub fn get_text_by_span(&self, sp: impl HasSpanData) -> Option<&str> {
        self.get_text(self.span_to_file_range(sp))
    }

    /// Gets a subslice of the file's text. Returns `None` if the range is invalid.
    ///
    /// With debug assertions this will validate that the range:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    pub fn get_text_by_src_range(&self, range: Range<BytePos>) -> Option<&str> {
        self.get_text(self.src_to_file_range(range))
    }

    /// Gets the `SyntaxContext` this was created with.
    #[inline]
    #[must_use]
    pub fn ctxt(&self) -> SyntaxContext {
        self.ctxt
    }

    /// Checks if this file contains the specified `SourceMap` position.
    #[inline]
    #[must_use]
    pub fn contains_pos(&self, pos: BytePos) -> bool {
        let file = self.file();
        pos.0.wrapping_sub(file.start_pos.0) <= file.normalized_source_len.0
    }

    /// Converts the file range into a `SourceMap` range. The previous range can be provided
    /// additional context for the debug assertions.
    ///
    /// With debug assertions this will validate that the range:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn mk_source_range(&self, range: FileRange, old: Option<FileRange>) -> Range<BytePos> {
        self.dbg_check_range(old, range.clone());
        let offset = self.file().start_pos.0;
        BytePos(range.start.0.wrapping_add(offset))..BytePos(range.end.0.wrapping_add(offset))
    }

    /// Converts the file range into a `Span`. The previous range can be provided additional context
    /// for the debug assertions.
    ///
    /// With debug assertions this will validate that the range:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn mk_span(&self, range: FileRange, old: Option<FileRange>) -> Span {
        let range = self.mk_source_range(range, old);
        Span::new(range.start, range.end, self.ctxt, self.parent)
    }

    /// Converts the span into a file range.
    ///
    /// With debug assertions this will validate that the span:
    /// * Is from the same syntax context.
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn span_to_file_range(&self, sp: impl HasSpanData) -> FileRange {
        let data = sp.data();
        debug_assert_eq!(self.ctxt, data.ctxt);
        self.src_to_file_range(data.lo..data.hi)
    }

    /// Converts the `SourceMap` range into a file range.
    ///
    /// With debug assertions this will validate that the range:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn src_to_file_range(&self, range: Range<BytePos>) -> FileRange {
        let offset = self.file().start_pos.0;
        let range =
            RelativeBytePos(range.start.0.wrapping_sub(offset))..RelativeBytePos(range.end.0.wrapping_sub(offset));
        self.dbg_check_range(None, range.clone());
        range
    }

    /// Gets the indent text of the line containing the specified position. Returns `None` if the
    /// position is outside the file's text.
    ///
    /// If the position is inside the line indent only the indent up to the position will be
    /// retrieved.
    #[must_use]
    pub fn get_line_indent_before(&self, pos: RelativeBytePos) -> Option<&str> {
        let file = self.file();
        let lines = file.lines();

        // `lines` either starts with zero or is empty. If it's empty we can use zero as the line
        // start.
        let line = lines.partition_point(|&start| start <= pos);
        let start = lines.get(line.wrapping_sub(1)).map_or(RelativeBytePos(0), |&x| x);
        self.get_text(start..pos)
            .map(|src| &src[..src.len() - src.trim_start().len()])
    }

    /// Runs debug checks on a range, panicking on failure. Does nothing if debug assertions are
    /// disabled.
    ///
    /// A second range can be given as a previous range before a transformation occurred. This will
    /// be displayed as additional context in the panic message, but will not cause additional
    /// validation.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn dbg_check_range(&self, old: Option<FileRange>, new: FileRange) {
        // `cfg` since we only have a source map with debug assertions.
        #[cfg(debug_assertions)]
        dbg_check_range(self.sm, &self.text, old, new);
    }
}

/// A collection of helper functions for adjusting a range within a file.
pub trait FileRangeExt: Sized + FileIndex {
    /// If the range doesn't overlap with the specified span returns the range between the two.
    /// Returns `None` otherwise.
    ///
    /// With debug assertions enabled this will assert that the span:
    /// * Is within the same `SyntaxContext`
    /// * Is within the same file as the current range.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_range_between(self, scx: &SpanEditCx<'_>, other: impl HasSpanData) -> Option<FileRange> {
        <FileRange as FileRangeExt>::get_range_between(self.into_file_range(scx), scx, other)
    }

    /// If the range starts at or after the specified position returns the range from that position
    /// to the end of the range. Returns `None` otherwise.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same `SyntaxContext`
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn extend_start_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        <FileRange as FileRangeExt>::extend_start_to(self.into_file_range(scx), scx, pos)
    }

    /// If the range ends before or at the specified position returns the range from the start of
    /// the range to that position. Returns `None` otherwise.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same `SyntaxContext`
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn extend_end_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        <FileRange as FileRangeExt>::extend_end_to(self.into_file_range(scx), scx, pos)
    }

    /// If the specified position lies within or at the end of range returns the range from that
    /// position to the end of the range. Returns `None` otherwise.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same `SyntaxContext`
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn shrink_start_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        <FileRange as FileRangeExt>::shrink_start_to(self.into_file_range(scx), scx, pos)
    }

    /// If the specified position lies within or at the end of range returns the range from the
    /// start of the range to that position. Returns `None` otherwise.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same `SyntaxContext`
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn shrink_end_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        <FileRange as FileRangeExt>::shrink_end_to(self.into_file_range(scx), scx, pos)
    }

    /// Creates a new file range that represents the result of mapping the text of the specified
    /// range into a substring. Returns `None` if either the mapping function returns `None`, or the
    /// range cannot index the file's text.
    ///
    /// The string returned by the mapping function must be derived from the input string. A
    /// `'static` lifetime string will not work. This case will panic if debug assertions are
    /// enabled.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn map_range_text(self, scx: &SpanEditCx<'_>, f: impl FnOnce(&str) -> Option<&str>) -> Option<FileRange> {
        <FileRange as FileRangeExt>::map_range_text(self.into_file_range(scx), scx, f)
    }

    /// Creates a new array of file ranges that represents the result of mapping the text of the
    /// specified range into an array of substrings. Returns `None` if either the mapping function
    /// returns `None`, or the range cannot index the file's text.
    ///
    /// The strings returned by the mapping function must be derived from the input string.
    /// `'static` lifetime strings will not work. This case will panic if debug assertions are
    /// enabled.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn map_split_range_text<const N: usize>(
        self,
        scx: &SpanEditCx<'_>,
        f: impl FnOnce(&str) -> Option<[&str; N]>,
    ) -> Option<[FileRange; N]> {
        <FileRange as FileRangeExt>::map_split_range_text(self.into_file_range(scx), scx, f)
    }

    /// Extends the range to include all immediately preceding whitespace. Returns `None` if the
    /// range cannot index the file's text.
    ///
    /// The range will not be expanded if it would cross a line boundary, the line the range would
    /// be extended to ends with a line comment and the text after the range contains a
    /// non-whitespace character on the same line. e.g.
    ///
    /// ```ignore
    /// ( // Some comment
    /// foo)
    /// ```
    ///
    /// When the range points to `foo`, suggesting to remove the range after it's been extended will
    /// cause the `)` to be placed inside the line comment as `( // Some comment)`.
    #[inline]
    #[must_use]
    fn with_leading_whitespace(self, scx: &SpanEditCx<'_>) -> Option<FileRange> {
        <FileRange as FileRangeExt>::with_leading_whitespace(self.into_file_range(scx), scx)
    }

    /// Extends the range to include all immediately proceeding whitespace. Returns `None` if the
    /// range cannot index the file's text.
    #[inline]
    #[must_use]
    fn with_trailing_whitespace(self, scx: &SpanEditCx<'_>) -> Option<FileRange> {
        <FileRange as FileRangeExt>::with_trailing_whitespace(self.into_file_range(scx), scx)
    }

    // Extends the range to include the immediately preceding pattern. Returns `None` if the pattern
    // does not immediately precede the range, or if the range cannot index the file's text.
    #[inline]
    #[must_use]
    fn with_leading_match<P>(self, scx: &SpanEditCx<'_>, pat: P) -> Option<FileRange>
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        <FileRange as FileRangeExt>::with_leading_match(self.into_file_range(scx), scx, pat)
    }

    // Extends the range to include the immediately proceeding pattern. Returns `None` if the pattern
    // does not immediately proceed the range, or if the range cannot index the file's text.
    #[inline]
    #[must_use]
    fn with_trailing_match(self, scx: &SpanEditCx<'_>, pat: impl Pattern) -> Option<FileRange> {
        <FileRange as FileRangeExt>::with_trailing_match(self.into_file_range(scx), scx, pat)
    }
}
impl FileRangeExt for FileRange {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_range_between(self, scx: &SpanEditCx<'_>, sp: impl HasSpanData) -> Option<FileRange> {
        #[inline]
        #[cfg_attr(debug_assertions, track_caller)]
        fn f(self_: FileRange, scx: &SpanEditCx<'_>, sp: SpanData) -> Option<FileRange> {
            debug_assert_eq!(scx.ctxt, sp.ctxt);
            let file = scx.file();
            let other = RelativeBytePos(sp.lo.0.wrapping_sub(file.start_pos.0))
                ..RelativeBytePos(sp.hi.0.wrapping_sub(file.start_pos.0));
            scx.dbg_check_range(None, other.clone());
            if self_.end.0 <= other.start.0 {
                Some(self_.end..other.start)
            } else if self_.start.0 >= other.end.0 {
                Some(other.end..self_.start)
            } else {
                None
            }
        }
        f(self, scx, sp.data())
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn extend_start_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (pos <= self.start).then_some(pos..self.end)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn extend_end_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (pos >= self.end).then_some(self.start..pos)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn shrink_start_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (self.start <= pos && pos <= self.end).then_some(pos..self.end)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn shrink_end_to(self, scx: &SpanEditCx<'_>, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (self.start <= pos && pos <= self.end).then_some(self.start..pos)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    #[allow(clippy::manual_map, reason = "track_caller doesn't work through `map`")]
    fn map_range_text(self, scx: &SpanEditCx<'_>, f: impl FnOnce(&str) -> Option<&str>) -> Option<FileRange> {
        let src = scx.text.as_str();
        match src.get(self.start.to_usize()..self.end.to_usize()).and_then(f) {
            Some(s) => {
                let base = src.as_ptr().addr();
                debug_assert!(
                    base <= s.as_ptr().addr() && s.as_ptr().addr() + s.len() <= base + src.len(),
                    "the string is not a valid substring",
                );
                let start = s.as_ptr().addr() - base;
                Some(RelativeBytePos::from_usize(start)..RelativeBytePos::from_usize(start + s.len()))
            },
            None => None,
        }
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    #[allow(clippy::manual_map, reason = "track_caller doesn't work through `map`")]
    fn map_split_range_text<const N: usize>(
        self,
        scx: &SpanEditCx<'_>,
        f: impl FnOnce(&str) -> Option<[&str; N]>,
    ) -> Option<[FileRange; N]> {
        let src = scx.text.as_str();
        match src.get(self.start.to_usize()..self.end.to_usize()).and_then(f) {
            Some(s) => {
                let base = src.as_ptr().addr();
                Some(map_array_inline!(s, s => {
                    debug_assert!(
                        base <= s.as_ptr().addr() && s.as_ptr().addr() + s.len() <= base + src.len(),
                        "the string is not a valid substring",
                    );
                    let start = s.as_ptr().addr() - base;
                    RelativeBytePos::from_usize(start)..RelativeBytePos::from_usize(start + s.len())
                }))
            },
            None => None,
        }
    }

    fn with_leading_whitespace(self, scx: &SpanEditCx<'_>) -> Option<FileRange> {
        let src = scx.file_text();
        let sf = scx.file();

        let mut trimmed_lf = false;
        let text_before = src.get(..self.start.to_usize())?.trim_end_matches(|c: char| {
            trimmed_lf |= c == '\n';
            c.is_whitespace()
        });
        if trimmed_lf
            && let line_starts = sf.lines()
            && let post_search_line = line_starts.partition_point(|&pos| pos.to_usize() <= text_before.len())
            // `get` can fail if `line_starts` is missing the starting zero.
            // Just start the search at the beginning in that case.
            && let search_start = line_starts.get(post_search_line - 1).map_or(0, |&x| x.to_usize())
            && ends_with_line_comment_or_broken(&text_before[search_start..])
            // Is there anything after the range on the same line?
            && !src.get(self.end.to_usize()..)?.chars().take_while(|&c| c != '\n').all(char::is_whitespace)
        {
            Some(self)
        } else {
            Some(RelativeBytePos::from_usize(text_before.len())..self.end)
        }
    }

    fn with_trailing_whitespace(self, scx: &SpanEditCx<'_>) -> Option<FileRange> {
        scx.get_text(self.end..)
            .map(|s| self.start..RelativeBytePos::from_usize(scx.text.len() - s.trim_start().len()))
    }

    fn with_leading_match<P>(self, scx: &SpanEditCx<'_>, pat: P) -> Option<FileRange>
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        scx.get_text(..self.start)
            .and_then(|s| s.strip_suffix(pat))
            .map(|s| RelativeBytePos::from_usize(s.len())..self.end)
    }

    fn with_trailing_match(self, scx: &SpanEditCx<'_>, pat: impl Pattern) -> Option<FileRange> {
        scx.get_text(self.end..)
            .and_then(|s| s.strip_prefix(pat))
            .map(|s| self.start..RelativeBytePos::from_usize(scx.text.len() - s.len()))
    }
}
impl FileRangeExt for RangeFull {}
impl FileRangeExt for RangeTo<RelativeBytePos> {}
impl FileRangeExt for RangeFrom<RelativeBytePos> {}

pub trait StrExt {
    /// Gets the substring which ranges from the start of the first match of the pattern to the end
    /// of the second match. Returns `None` if the pattern doesn't occur twice.
    ///
    /// # Examples
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use clippy_utils::source::StrExt;
    /// let s = "move |arg| arg.foo()";
    /// assert_eq!(s.find_bounded_inclusive('|'), Some("|arg|"));
    ///
    /// let s = "foo | bar";
    /// assert_eq!(s.find_bounded_inclusive('|'), None)
    /// ```
    fn find_bounded_inclusive(&self, pat: impl Pattern) -> Option<&Self>;

    /// Gets the non-overlapping prefix and suffix. Returns `None` if the string doesn't start with
    /// the prefix or end with the suffix.
    ///
    /// The prefix will be taken first, with the suffix taken from the remainder of the string.
    ///
    /// # Examples
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use clippy_utils::source::StrExt;
    /// let s = "[a, b, c]";
    /// assert_eq!(s.get_prefix_suffix('[', ']'), Some(["[", "]"]));
    /// ```
    fn get_prefix_suffix<P>(&self, prefix: impl Pattern, suffix: P) -> Option<[&Self; 2]>
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>;

    /// Splits a string into a prefix and everything proceeding it. Returns `None` if the string
    /// doesn't start with the prefix.
    ///
    /// # Examples
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use clippy_utils::source::StrExt;
    /// let s = "fn foo()";
    /// assert_eq!(s.split_prefix("fn"), Some(["fn", " foo()"]));
    /// ```
    fn split_prefix(&self, pat: impl Pattern) -> Option<[&Self; 2]>;

    /// Splits a string into a suffix and everything preceding it. Returns `None` if the string
    /// doesn't end with the suffix.
    ///
    /// # Examples
    /// ```rust
    /// # #![feature(rustc_private)]
    /// # use clippy_utils::source::StrExt;
    /// let s = "foo.bar()?";
    /// assert_eq!(s.split_suffix('?'), Some(["foo.bar()", "?"]));
    /// ```
    fn split_suffix<P>(&self, pat: P) -> Option<[&Self; 2]>
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>;
}
impl StrExt for str {
    fn find_bounded_inclusive(&self, pat: impl Pattern) -> Option<&Self> {
        let mut iter = self.match_indices(pat);
        if let Some((first_pos, _)) = iter.next()
            && let Some((second_pos, second)) = iter.next()
        {
            Some(&self[first_pos..second_pos + second.len()])
        } else {
            None
        }
    }

    fn get_prefix_suffix<P>(&self, prefix: impl Pattern, suffix: P) -> Option<[&Self; 2]>
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        if let Some([pre, s]) = self.split_prefix(prefix)
            && let Some([_, suf]) = s.split_suffix(suffix)
        {
            Some([pre, suf])
        } else {
            None
        }
    }

    #[inline]
    fn split_prefix(&self, pat: impl Pattern) -> Option<[&Self; 2]> {
        self.strip_prefix(pat)
            .map(|rest| [&self[..self.len() - rest.len()], rest])
    }

    #[inline]
    fn split_suffix<P>(&self, pat: P) -> Option<[&Self; 2]>
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        self.strip_suffix(pat).map(|rest| [rest, &self[rest.len()..]])
    }
}

/// Checks if the last token of the string is either a line comment or an incomplete token.
fn ends_with_line_comment_or_broken(text: &str) -> bool {
    let Some(last) = tokenize(text, FrontmatterAllowed::No).last() else {
        return false;
    };
    match last.kind {
        // Will give the wrong result on text like `" // "` where the first quote ends a string
        // started earlier. The only workaround is to lex the whole file which we don't really want
        // to do.
        TokenKind::LineComment { .. } | TokenKind::BlockComment { terminated: false, .. } => true,
        TokenKind::Literal { kind, .. } => matches!(
            kind,
            LiteralKind::Byte { terminated: false }
                | LiteralKind::ByteStr { terminated: false }
                | LiteralKind::CStr { terminated: false }
                | LiteralKind::Char { terminated: false }
                | LiteralKind::RawByteStr { n_hashes: None }
                | LiteralKind::RawCStr { n_hashes: None }
                | LiteralKind::RawStr { n_hashes: None }
        ),
        _ => false,
    }
}

/// Like `snippet_block`, but add braces if the expr is not an `ExprKind::Block` with no label.
pub fn expr_block<'sm>(
    sm: impl HasSourceMap<'sm>,
    expr: &Expr<'_>,
    outer: SyntaxContext,
    default: &str,
    indent_relative_to: Option<Span>,
    app: &mut Applicability,
) -> String {
    let (code, from_macro) = snippet_block_with_context(sm, expr.span, outer, default, indent_relative_to, app);
    if !from_macro
        && let ExprKind::Block(block, None) = expr.kind
        && block.rules != BlockCheckMode::UnsafeBlock(UnsafeSource::UserProvided)
    {
        code
    } else {
        // FIXME: add extra indent for the unsafe blocks:
        //     original code:   unsafe { ... }
        //     result code:     { unsafe { ... } }
        //     desired code:    {\n  unsafe { ... }\n}
        format!("{{ {code} }}")
    }
}

/// Returns a new Span that extends the original Span to the first non-whitespace char of the first
/// line.
///
/// ```rust,ignore
///     let x = ();
/// //          ^^
/// // will be converted to
///     let x = ();
/// //  ^^^^^^^^^^
/// ```
pub fn first_line_of_span<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> Span {
    first_char_in_first_line(sm, span).map_or(span, |first_char_pos| span.with_lo(first_char_pos))
}

fn first_char_in_first_line<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> Option<BytePos> {
    let line_span = line_span(sm, span);
    snippet_opt(sm, line_span).and_then(|snip| {
        snip.find(|c: char| !c.is_whitespace())
            .map(|pos| line_span.lo() + BytePos::from_usize(pos))
    })
}

/// Extends the span to the beginning of the spans line, incl. whitespaces.
///
/// ```no_run
///        let x = ();
/// //             ^^
/// // will be converted to
///        let x = ();
/// // ^^^^^^^^^^^^^^
/// ```
fn line_span<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> Span {
    let span = original_sp(span, DUMMY_SP);
    let SourceFileAndLine { sf, line } = sm.source_map().lookup_line(span.lo()).unwrap();
    let line_start = sf.lines()[line];
    let line_start = sf.absolute_position(line_start);
    span.with_lo(line_start)
}

/// Returns the indentation of the line of a span
///
/// ```rust,ignore
/// let x = ();
/// //      ^^ -- will return 0
///     let x = ();
/// //          ^^ -- will return 4
/// ```
pub fn indent_of<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> Option<usize> {
    snippet_opt(sm, line_span(sm, span)).and_then(|snip| snip.find(|c: char| !c.is_whitespace()))
}

/// Gets a snippet of the indentation of the line of a span
pub fn snippet_indent<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> Option<String> {
    snippet_opt(sm, line_span(sm, span)).map(|mut s| {
        let len = s.len() - s.trim_start().len();
        s.truncate(len);
        s
    })
}

// If the snippet is empty, it's an attribute that was inserted during macro
// expansion and we want to ignore those, because they could come from external
// sources that the user has no control over.
// For some reason these attributes don't have any expansion info on them, so
// we have to check it this way until there is a better way.
pub fn is_present_in_source<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> bool {
    if let Some(snippet) = snippet_opt(sm, span)
        && snippet.is_empty()
    {
        return false;
    }
    true
}

/// Returns the position just before rarrow
///
/// ```rust,ignore
/// fn into(self) -> () {}
///              ^
/// // in case of unformatted code
/// fn into2(self)-> () {}
///               ^
/// fn into3(self)   -> () {}
///               ^
/// ```
pub fn position_before_rarrow(s: &str) -> Option<usize> {
    s.rfind("->").map(|rpos| {
        let mut rpos = rpos;
        let chars: Vec<char> = s.chars().collect();
        while rpos > 1 {
            if let Some(c) = chars.get(rpos - 1)
                && c.is_whitespace()
            {
                rpos -= 1;
                continue;
            }
            break;
        }
        rpos
    })
}

/// Reindent a multiline string with possibility of ignoring the first line.
pub fn reindent_multiline(s: &str, ignore_first: bool, indent: Option<usize>) -> String {
    let s_space = reindent_multiline_inner(s, ignore_first, indent, ' ');
    let s_tab = reindent_multiline_inner(&s_space, ignore_first, indent, '\t');
    reindent_multiline_inner(&s_tab, ignore_first, indent, ' ')
}

fn reindent_multiline_inner(s: &str, ignore_first: bool, indent: Option<usize>, ch: char) -> String {
    let x = s
        .lines()
        .skip(usize::from(ignore_first))
        .filter_map(|l| {
            if l.is_empty() {
                None
            } else {
                // ignore empty lines
                Some(l.char_indices().find(|&(_, x)| x != ch).unwrap_or((l.len(), ch)).0)
            }
        })
        .min()
        .unwrap_or(0);
    let indent = indent.unwrap_or(0);
    s.lines()
        .enumerate()
        .map(|(i, l)| {
            if (ignore_first && i == 0) || l.is_empty() {
                l.to_owned()
            } else if x > indent {
                l.split_at(x - indent).1.to_owned()
            } else {
                " ".repeat(indent - x) + l
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Converts a span to a code snippet if available, otherwise returns the default.
///
/// This is useful if you want to provide suggestions for your lint or more generally, if you want
/// to convert a given `Span` to a `str`. To create suggestions consider using
/// [`snippet_with_applicability`] to ensure that the applicability stays correct.
///
/// # Example
/// ```rust,ignore
/// // Given two spans one for `value` and one for the `init` expression.
/// let value = Vec::new();
/// //  ^^^^^   ^^^^^^^^^^
/// //  span1   span2
///
/// // The snipped call would return the corresponding code snippet
/// snippet(cx, span1, "..") // -> "value"
/// snippet(cx, span2, "..") // -> "Vec::new()"
/// ```
pub fn snippet<'a, 'sm>(sm: impl HasSourceMap<'sm>, span: Span, default: &'a str) -> Cow<'a, str> {
    snippet_opt(sm, span).map_or_else(|| Cow::Borrowed(default), From::from)
}

/// Same as [`snippet`], but it adapts the applicability level by following rules:
///
/// - Applicability level `Unspecified` will never be changed.
/// - If the span is inside a macro, change the applicability level to `MaybeIncorrect`.
/// - If the default value is used and the applicability level is `MachineApplicable`, change it to
///   `HasPlaceholders`
pub fn snippet_with_applicability<'a, 'sm>(
    sm: impl HasSourceMap<'sm>,
    span: Span,
    default: &'a str,
    applicability: &mut Applicability,
) -> Cow<'a, str> {
    snippet_with_applicability_sm(sm.source_map(), span, default, applicability)
}

fn snippet_with_applicability_sm<'a>(
    sm: &SourceMap,
    span: Span,
    default: &'a str,
    applicability: &mut Applicability,
) -> Cow<'a, str> {
    if *applicability != Applicability::Unspecified && span.from_expansion() {
        *applicability = Applicability::MaybeIncorrect;
    }
    snippet_opt(sm, span).map_or_else(
        || {
            if *applicability == Applicability::MachineApplicable {
                *applicability = Applicability::HasPlaceholders;
            }
            Cow::Borrowed(default)
        },
        From::from,
    )
}

/// Converts a span to a code snippet. Returns `None` if not available.
pub fn snippet_opt<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> Option<String> {
    sm.source_map().span_to_snippet(span).ok()
}

/// Converts a span (from a block) to a code snippet if available, otherwise use default.
///
/// This trims the code of indentation, except for the first line. Use it for blocks or block-like
/// things which need to be printed as such.
///
/// The `indent_relative_to` arg can be used, to provide a span, where the indentation of the
/// resulting snippet of the given span.
///
/// # Example
///
/// ```rust,ignore
/// snippet_block(cx, block.span, "..", None)
/// // where, `block` is the block of the if expr
///     if x {
///         y;
///     }
/// // will return the snippet
/// {
///     y;
/// }
/// ```
///
/// ```rust,ignore
/// snippet_block(cx, block.span, "..", Some(if_expr.span))
/// // where, `block` is the block of the if expr
///     if x {
///         y;
///     }
/// // will return the snippet
/// {
///         y;
///     } // aligned with `if`
/// ```
/// Note that the first line of the snippet always has 0 indentation.
pub fn snippet_block<'sm>(
    sm: impl HasSourceMap<'sm>,
    span: Span,
    default: &str,
    indent_relative_to: Option<Span>,
) -> String {
    let snip = snippet(sm, span, default);
    let indent = indent_relative_to.and_then(|s| indent_of(sm, s));
    reindent_multiline(&snip, true, indent)
}

/// Same as `snippet_block`, but adapts the applicability level by the rules of
/// `snippet_with_applicability`.
pub fn snippet_block_with_applicability<'sm>(
    sm: impl HasSourceMap<'sm>,
    span: Span,
    default: &str,
    indent_relative_to: Option<Span>,
    applicability: &mut Applicability,
) -> String {
    let snip = snippet_with_applicability(sm, span, default, applicability);
    let indent = indent_relative_to.and_then(|s| indent_of(sm, s));
    reindent_multiline(&snip, true, indent)
}

pub fn snippet_block_with_context<'sm>(
    sm: impl HasSourceMap<'sm>,
    span: Span,
    outer: SyntaxContext,
    default: &str,
    indent_relative_to: Option<Span>,
    app: &mut Applicability,
) -> (String, bool) {
    let (snip, from_macro) = snippet_with_context(sm, span, outer, default, app);
    let indent = indent_relative_to.and_then(|s| indent_of(sm, s));
    (reindent_multiline(&snip, true, indent), from_macro)
}

/// Same as `snippet_with_applicability`, but first walks the span up to the given context.
///
/// This will result in the macro call, rather than the expansion, if the span is from a child
/// context. If the span is not from a child context, it will be used directly instead.
///
/// e.g. Given the expression `&vec![]`, getting a snippet from the span for `vec![]` as a HIR node
/// would result in `box []`. If given the context of the address of expression, this function will
/// correctly get a snippet of `vec![]`.
///
/// This will also return whether or not the snippet is a macro call.
pub fn snippet_with_context<'a, 'sm>(
    sm: impl HasSourceMap<'sm>,
    span: Span,
    outer: SyntaxContext,
    default: &'a str,
    applicability: &mut Applicability,
) -> (Cow<'a, str>, bool) {
    snippet_with_context_sm(sm.source_map(), span, outer, default, applicability)
}

fn snippet_with_context_sm<'a>(
    sm: &SourceMap,
    span: Span,
    outer: SyntaxContext,
    default: &'a str,
    applicability: &mut Applicability,
) -> (Cow<'a, str>, bool) {
    // If it is just range desugaring, use the desugaring span since it may include parenthesis.
    if span.desugaring_kind() == Some(DesugaringKind::RangeExpr) && span.parent_callsite().unwrap().ctxt() == outer {
        return (snippet_with_applicability_sm(sm, span, default, applicability), false);
    }

    let (span, is_macro_call) = walk_span_to_context(span, outer).map_or_else(
        || {
            // The span is from a macro argument, and the outer context is the macro using the argument
            if *applicability != Applicability::Unspecified {
                *applicability = Applicability::MaybeIncorrect;
            }
            // TODO: get the argument span.
            (span, false)
        },
        |outer_span| (outer_span, span.ctxt() != outer),
    );

    (
        snippet_with_applicability_sm(sm, span, default, applicability),
        is_macro_call,
    )
}

/// Walks the span up to the target context, thereby returning the macro call site if the span is
/// inside a macro expansion, or the original span if it is not.
///
/// Note this will return `None` in the case of the span being in a macro expansion, but the target
/// context is from expanding a macro argument.
///
/// Given the following
///
/// ```rust,ignore
/// macro_rules! m { ($e:expr) => { f($e) }; }
/// g(m!(0))
/// ```
///
/// If called with a span of the call to `f` and a context of the call to `g` this will return a
/// span containing `m!(0)`. However, if called with a span of the literal `0` this will give a span
/// containing `0` as the context is the same as the outer context.
///
/// This will traverse through multiple macro calls. Given the following:
///
/// ```rust,ignore
/// macro_rules! m { ($e:expr) => { n!($e, 0) }; }
/// macro_rules! n { ($e:expr, $f:expr) => { f($e, $f) }; }
/// g(m!(0))
/// ```
///
/// If called with a span of the call to `f` and a context of the call to `g` this will return a
/// span containing `m!(0)`.
pub fn walk_span_to_context(span: Span, outer: SyntaxContext) -> Option<Span> {
    let outer_span = hygiene::walk_chain(span, outer);
    (outer_span.ctxt() == outer).then_some(outer_span)
}

/// Trims the whitespace from the start and the end of the span.
pub fn trim_span(sm: &SourceMap, span: Span) -> Span {
    let data = span.data();
    let sf: &_ = &sm.lookup_source_file(data.lo);
    let Some(src) = sf.src.as_deref() else {
        return span;
    };
    let Some(snip) = &src.get((data.lo - sf.start_pos).to_usize()..(data.hi - sf.start_pos).to_usize()) else {
        return span;
    };
    let trim_start = snip.len() - snip.trim_start().len();
    let trim_end = snip.len() - snip.trim_end().len();
    SpanData {
        lo: data.lo + BytePos::from_usize(trim_start),
        hi: data.hi - BytePos::from_usize(trim_end),
        ctxt: data.ctxt,
        parent: data.parent,
    }
    .span()
}

/// Expand a span to include a preceding comma
/// ```rust,ignore
/// writeln!(o, "")   ->   writeln!(o, "")
///             ^^                   ^^^^
/// ```
pub fn expand_past_previous_comma<'sm>(sm: impl HasSourceMap<'sm>, span: Span) -> Span {
    let extended = sm.source_map().span_extend_to_prev_char(span, ',', true);
    extended.with_lo(extended.lo() - BytePos(1))
}

/// Converts `expr` to a `char` literal if it's a `str` literal containing a single
/// character (or a single byte with `ascii_only`)
pub fn str_literal_to_char_literal<'sm>(
    sm: impl HasSourceMap<'sm>,
    expr: &Expr<'_>,
    applicability: &mut Applicability,
    ascii_only: bool,
) -> Option<String> {
    if let ExprKind::Lit(lit) = &expr.kind
        && let LitKind::Str(r, style) = lit.node
        && let string = r.as_str()
        && let len = if ascii_only {
            string.len()
        } else {
            string.chars().count()
        }
        && len == 1
    {
        let snip = snippet_with_applicability(sm, expr.span, string, applicability);
        let ch = if let StrStyle::Raw(nhash) = style {
            let nhash = nhash as usize;
            // for raw string: r##"a"##
            &snip[(nhash + 2)..(snip.len() - 1 - nhash)]
        } else {
            // for regular string: "a"
            &snip[1..(snip.len() - 1)]
        };

        let hint = format!(
            "'{}'",
            match ch {
                "'" => "\\'",
                r"\" => "\\\\",
                "\\\"" => "\"", // no need to escape `"` in `'"'`
                _ => ch,
            }
        );

        Some(hint)
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::reindent_multiline;

    #[test]
    fn test_reindent_multiline_single_line() {
        assert_eq!("", reindent_multiline("", false, None));
        assert_eq!("...", reindent_multiline("...", false, None));
        assert_eq!("...", reindent_multiline("    ...", false, None));
        assert_eq!("...", reindent_multiline("\t...", false, None));
        assert_eq!("...", reindent_multiline("\t\t...", false, None));
    }

    #[test]
    #[rustfmt::skip]
    fn test_reindent_multiline_block() {
        assert_eq!("\
    if x {
        y
    } else {
        z
    }", reindent_multiline("    if x {
            y
        } else {
            z
        }", false, None));
        assert_eq!("\
    if x {
    \ty
    } else {
    \tz
    }", reindent_multiline("    if x {
        \ty
        } else {
        \tz
        }", false, None));
    }

    #[test]
    #[rustfmt::skip]
    fn test_reindent_multiline_empty_line() {
        assert_eq!("\
    if x {
        y

    } else {
        z
    }", reindent_multiline("    if x {
            y

        } else {
            z
        }", false, None));
    }

    #[test]
    #[rustfmt::skip]
    fn test_reindent_multiline_lines_deeper() {
        assert_eq!("\
        if x {
            y
        } else {
            z
        }", reindent_multiline("\
    if x {
        y
    } else {
        z
    }", true, Some(8)));
    }
}
