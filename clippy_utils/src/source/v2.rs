//! Utilities for interacting with the source text and manipulating spans.
//!
//! The main entry points for working with the source text are on the [`SpanExt`] trait. This trait
//! is implemented on a few types and exists as a bridge between a [`Span`] and the source text
//! backing it. The following are the main functions:
//!
//! * [`SpanExt::mk_file_cx`]: This is the most general method interacting with the source text and the other methods
//!   should be preferred when possible. This is useful for handling multiple sub-spans (e.g. multiple items in a list),
//!   or when checking a predicate on the text surrounding a span.
//! * [`SpanExt::get_text`]: Gets a `SourceText` representing the text the span refers to. It works very similarly to an
//!   `Arc<str>`. This is normally used when building a suggestion.
//! * [`SpanExt::get_text_within_other`]: Like `get_text`, but first adjusts the context to match another span. This is
//!   needed when the node this span is from might be created by a macro.
//! * [`SpanExt::check_text`]: A slightly simpler way to check a predicate on the text than using `get_text`. Prefer
//!   this if possible.
//! * [`SpanExt::map_range`]: The main way to adjust the range portion of a span. See [`SpanFileCx`] for range
//!   adjustment utilities.
//! * [`SpanExt::map_split_range`]: Similar to the previous, but for when you need to create multiple spans from a
//!   single span.
//!
//! # Working With Macros
//!
//! All parts of the linting infrastructure work after macro expansion meaning that there are no
//! nodes in the AST or HIR representing a macro call (AST nodes have a variant for these, but it's
//! only used before expansion). Because of this there's no direct way of knowing where a macro call
//! occurred. Instead the `Span`, or more specifically the [`SyntaxContext`], notes which expansion
//! created the node.
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
//! can only be detected by checking the source text. With this the source text of all AST/HIR
//! items can be almost anything. In short, validate all range adjustments against the source text.

use super::{HasSourceMap, ends_with_line_comment_or_broken};
use core::borrow::Borrow;
use core::fmt;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::{Deref, Index, Range, RangeFrom, RangeFull, RangeTo};
use core::slice::SliceIndex;
use core::str::pattern::{Pattern, ReverseSearcher};
use rustc_hir::def_id::LocalDefId;
use rustc_lexer::is_whitespace;
use rustc_middle::ty::tls;
use rustc_span::source_map::SourceMap;
use rustc_span::{
    BytePos, DesugaringKind, ExpnKind, Pos as _, RelativeBytePos, SourceFile, Span, SpanData, SyntaxContext, hygiene,
    with_metavar_spans,
};

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
        let src: ManuallyDrop<[_; _]> = ManuallyDrop::new($a);
        let mut dst = mk_uninit(&src);
        for i in 0..dst.len() {
            // SAFETY:
            // `src` was fully initialized at the start of the loop.
            // Each loop iteration is accessing a different item in the array.
            // The array is never dropped after moving from it.
            let $p = unsafe { (&raw const src[i]).read() };
            dst[i].write($e);
        }
        // SAFETY:
        // `mk_uninit` makes the `src`` and `dst`` arrays the same length.
        // The loop has moved every item from `src` into `dst`.
        unsafe { MaybeUninit::array_assume_init(dst) }
    }};
}

pub trait ToSpanData: Copy {
    #[must_use]
    fn data(self) -> SpanData;
}
impl ToSpanData for SpanData {
    #[inline]
    fn data(self) -> SpanData {
        self
    }
}
impl ToSpanData for &SpanData {
    #[inline]
    fn data(self) -> SpanData {
        *self
    }
}
impl ToSpanData for Span {
    #[inline]
    fn data(self) -> SpanData {
        self.data()
    }
}

/// Marker trait for `Span`.
///
/// Only exists so all functions can exist on the `SpanExt` trait.
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
        Self: ToSpanData,
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
        Self: ToSpanData,
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
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_text<'sm>(self, sm: impl HasSourceMap<'sm>) -> Option<SourceText> {
        SourceText::for_local_range(sm.source_map(), self.span_range())
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
        self.get_text(sm).as_deref().is_some_and(pred)
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
            if expn.call_site.ctxt() != target {
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
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn mk_file_cx<'sm>(self, sm: impl HasSourceMap<'sm>) -> Option<(SpanFileCx, FileRange)>
    where
        Self: ToSpanData,
    {
        SpanFileCx::for_local(sm.source_map(), self.data())
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
        f: impl FnOnce(&SpanFileCx, FileRange) -> Option<FileRange>,
    ) -> Option<Span>
    where
        Self: ToSpanData,
    {
        if let Some((scx, range)) = self.mk_file_cx(sm)
            && let Some(mapped) = f(&scx, range.clone())
        {
            Some(scx.mk_transformed_span(mapped, range))
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
        f: impl FnOnce(&SpanFileCx, FileRange) -> Option<[FileRange; N]>,
    ) -> Option<[Span; N]>
    where
        Self: ToSpanData,
    {
        if let Some((scx, range)) = self.mk_file_cx(sm)
            && let Some(mapped) = f(&scx, range.clone())
        {
            Some(map_array_inline!(mapped, r => scx.mk_transformed_span(r, range.clone())))
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
        #[inline]
        #[must_use]
        pub fn for_local_file(file: Arc<SourceFile>) -> Option<Self> {
            if let Some(text) = &file.src {
                let text: *const str = &raw const ***text;
                Some(Self { file, text })
            } else {
                None
            }
        }

        /// Gets the source text.
        #[inline]
        #[must_use]
        pub fn as_str(&self) -> &str {
            // SAFETY: `text` is owned by `file` and comes from either an `Option<Arc<String>>`, or a
            // frozen `FeezeLock<ExternalSrc>` (which ultimately contains an `Arc<String>`). Neither
            // of these can change so long as we own `file`.
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
            self.text = f(&self)?;
            Some(self)
        }
    }
}
pub use self::source_text::SourceText;
impl SourceText {
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn for_local_range(sm: &SourceMap, range: Range<BytePos>) -> Option<Self> {
        let sfp = sm.lookup_byte_offset(range.start);
        let text = Self::for_local_file(sfp.sf)?;
        let range = RelativeBytePos(sfp.pos.0)..RelativeBytePos(range.end.0.wrapping_sub(text.file().start_pos.0));
        dbg_check_range(&text, None, range.clone());
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
    type SliceIndex: SliceIndex<str, Output = str>;

    #[must_use]
    fn into_slice_idx(self) -> Self::SliceIndex;

    /// Converts this into a bounded range by limiting unbounded ends to the file's bounds.
    #[must_use]
    fn into_file_range(self, scx: &SpanFileCx) -> FileRange;
}
impl FileIndex for RangeFull {
    type SliceIndex = Self;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceIndex {
        self
    }
    #[inline]
    fn into_file_range(self, scx: &SpanFileCx) -> FileRange {
        RelativeBytePos(0)..scx.file().normalized_source_len
    }
}
impl FileIndex for Range<RelativeBytePos> {
    type SliceIndex = Range<usize>;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceIndex {
        self.start.to_usize()..self.end.to_usize()
    }
    #[inline]
    fn into_file_range(self, _: &SpanFileCx) -> FileRange {
        self
    }
}
impl FileIndex for RangeTo<RelativeBytePos> {
    type SliceIndex = RangeTo<usize>;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceIndex {
        ..self.end.to_usize()
    }
    #[inline]
    fn into_file_range(self, _: &SpanFileCx) -> FileRange {
        RelativeBytePos(0)..self.end
    }
}
impl FileIndex for RangeFrom<RelativeBytePos> {
    type SliceIndex = RangeFrom<usize>;
    #[inline]
    fn into_slice_idx(self) -> Self::SliceIndex {
        self.start.to_usize()..
    }
    #[inline]
    fn into_file_range(self, scx: &SpanFileCx) -> FileRange {
        self.start..scx.file().normalized_source_len
    }
}

/// The range type used for specifying a range within a file.
pub type FileRange = Range<RelativeBytePos>;

#[cfg_attr(not(debug_assertions), inline)]
#[cfg_attr(debug_assertions, track_caller)]
fn dbg_check_range(text: &SourceText, old: Option<FileRange>, new: FileRange) {
    if cfg!(debug_assertions) && text.get(new.clone().into_slice_idx()).is_none() {
        tls::with(
            #[track_caller]
            |tcx| {
                let file = &**text.file();
                let dcx = tcx.dcx();
                let mut diag = dcx.struct_bug(format!(
                    "invalid range `{}..{}` within file `{}`",
                    // Signed numbers will better show most errors.
                    new.start.0.cast_signed(),
                    new.end.0.cast_signed(),
                    file.name.prefer_local_unconditionally(),
                ));
                if new.start <= file.normalized_source_len {
                    let (sp, msg) = if text.is_char_boundary(new.start.to_usize()) {
                        let pos = BytePos(new.start.0 + file.start_pos.0);
                        (
                            Span::new(pos, pos, SyntaxContext::root(), None),
                            "the range starts here",
                        )
                    } else {
                        #[expect(clippy::cast_possible_truncation)]
                        let lo = BytePos(text.floor_char_boundary(new.start.to_usize()) as u32 + file.start_pos.0);
                        #[expect(clippy::cast_possible_truncation)]
                        let hi = BytePos(text.ceil_char_boundary(new.start.to_usize()) as u32 + file.start_pos.0);
                        (
                            Span::new(lo, hi, SyntaxContext::root(), None),
                            "the range starts within this character",
                        )
                    };
                    diag.span_note(sp, msg);
                } else {
                    diag.note("the range starts outside this file");
                }
                if new.end <= file.normalized_source_len {
                    let (sp, msg) = if text.is_char_boundary(new.end.to_usize()) {
                        let pos = BytePos(new.end.0 + file.start_pos.0);
                        (Span::new(pos, pos, SyntaxContext::root(), None), "the range ends here")
                    } else {
                        #[expect(clippy::cast_possible_truncation)]
                        let lo = BytePos(text.floor_char_boundary(new.end.to_usize()) as u32 + file.start_pos.0);
                        #[expect(clippy::cast_possible_truncation)]
                        let hi = BytePos(text.ceil_char_boundary(new.end.to_usize()) as u32 + file.start_pos.0);
                        (
                            Span::new(lo, hi, SyntaxContext::root(), None),
                            "the range ends within this character",
                        )
                    };
                    diag.span_note(sp, msg);
                } else {
                    diag.note("the range ends outside this file");
                }
                if let Some(old) = old
                    && text.get(old.clone().into_slice_idx()).is_some()
                {
                    let lo = BytePos(old.start.0 + file.start_pos.0);
                    let hi = BytePos(old.end.0 + file.start_pos.0);
                    diag.span_note(
                        Span::new(lo, hi, SyntaxContext::root(), None),
                        "the range was transformed from this span",
                    );
                }
                diag.emit();
            },
        );
    }
}

/// The context used to manipulate source ranges within a single file.
pub struct SpanFileCx {
    text: SourceText,
    ctxt: SyntaxContext,
    parent: Option<LocalDefId>,
}
impl SpanFileCx {
    /// Creates a new edit context for a span within a crate-local file. Returns `None` if span is
    /// within a non-local file.
    ///
    /// With debug assertions this will validate that the span:
    /// * The start and end do not overlap.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on UTF-8 boundaries.
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn for_local(sm: &SourceMap, data: SpanData) -> Option<(Self, FileRange)> {
        let sfp = sm.lookup_byte_offset(data.lo);
        let range = RelativeBytePos(sfp.pos.0)..RelativeBytePos(data.hi.0.wrapping_sub(sfp.sf.start_pos.0));

        let scx = Self {
            text: SourceText::for_local_file(sfp.sf)?,
            ctxt: data.ctxt,
            parent: data.parent,
        };
        scx.dbg_check_range(None, range.clone());
        Some((scx, range))
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
    pub fn get_text_by_span(&self, sp: impl ToSpanData) -> Option<&str> {
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
        self.file().contains(pos)
    }

    /// Converts the file range into a `SourceMap` range.
    ///
    /// With debug assertions this will validate that the range:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn mk_src_range(&self, range: FileRange, old: Option<FileRange>) -> Range<BytePos> {
        self.dbg_check_range(old, range.clone());
        let offset = self.file().start_pos.0;
        BytePos(range.start.0.wrapping_add(offset))..BytePos(range.end.0.wrapping_add(offset))
    }

    /// Converts the file range into a `Span`.
    ///
    /// With debug assertions this will validate that the range:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn mk_span(&self, range: FileRange) -> Span {
        self.dbg_check_range(None, range.clone());
        let offset = self.file().start_pos.0;
        Span::new(
            BytePos(range.start.0.wrapping_add(offset)),
            BytePos(range.end.0.wrapping_add(offset)),
            self.ctxt,
            self.parent,
        )
    }

    /// Converts the file range into a `Span`.
    ///
    /// Does the same as `mk_span` except this takes a second range as context for
    /// assertion failures.
    ///
    /// With debug assertions this will validate that the range:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn mk_transformed_span(&self, range: FileRange, prev: FileRange) -> Span {
        self.dbg_check_range(Some(prev), range.clone());
        let offset = self.file().start_pos.0;
        Span::new(
            BytePos(range.start.0.wrapping_add(offset)),
            BytePos(range.end.0.wrapping_add(offset)),
            self.ctxt,
            self.parent,
        )
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
    pub fn span_to_file_range(&self, sp: impl ToSpanData) -> FileRange {
        let data = sp.data();
        debug_assert_eq!(self.ctxt, data.ctxt);
        self.src_to_file_range(data.lo..data.hi)
    }

    /// Converts the span into a file range if the span is in the current syntax context.
    ///
    /// With debug assertions this will validate that the span:
    /// * Is contained within the current file.
    /// * The start and end do not overlap.
    /// * Lies on UTF-8 boundaries.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn span_to_file_range_check_ctxt(&self, sp: impl ToSpanData) -> Option<FileRange> {
        let data = sp.data();
        (data.ctxt == self.ctxt).then(|| self.src_to_file_range(data.lo..data.hi))
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
    pub fn get_line_indent_before(&self, pos: RelativeBytePos) -> &str {
        let lines = self.file().lines();
        let src = self.file_text();

        // `lines` either starts with zero or is empty. If it's empty we can use zero as the line
        // start.
        let line = lines.partition_point(|&start| start <= pos);
        let start = lines.get(line.wrapping_sub(1)).map_or(RelativeBytePos(0), |&x| x);
        let src = &src[start.to_usize()..];
        let end = src
            .trim_start_matches(|c| is_whitespace(c) && c != '\n')
            .len()
            .min(pos.to_usize() - start.to_usize());
        &src[..end]
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
        dbg_check_range(&self.text, old, new);
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
    fn get_range_between(self, scx: &SpanFileCx, other: impl ToSpanData) -> Option<FileRange> {
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
    fn extend_start_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
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
    fn extend_end_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
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
    fn shrink_start_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
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
    fn shrink_end_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
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
    fn map_range_text(self, scx: &SpanFileCx, f: impl FnOnce(&str) -> Option<&str>) -> Option<FileRange> {
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
        scx: &SpanFileCx,
        f: impl FnOnce(&str) -> Option<[&str; N]>,
    ) -> Option<[FileRange; N]> {
        <FileRange as FileRangeExt>::map_split_range_text(self.into_file_range(scx), scx, f)
    }

    /// Extends the range to include all immediately preceding whitespace.
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
    fn with_leading_whitespace(self, scx: &SpanFileCx) -> FileRange {
        <FileRange as FileRangeExt>::with_leading_whitespace(self.into_file_range(scx), scx)
    }

    /// Extends the range to include all immediately proceeding whitespace.
    #[inline]
    #[must_use]
    fn with_trailing_whitespace(self, scx: &SpanFileCx) -> FileRange {
        <FileRange as FileRangeExt>::with_trailing_matches(self.into_file_range(scx), scx, is_whitespace)
    }

    // Extends the range to include the immediately preceding pattern. Returns `None` if the pattern
    // does not immediately precede the range, or if the range cannot index the file's text.
    #[inline]
    #[must_use]
    fn with_leading_match<P>(self, scx: &SpanFileCx, pat: P) -> Option<FileRange>
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
    fn with_trailing_match(self, scx: &SpanFileCx, pat: impl Pattern) -> Option<FileRange> {
        <FileRange as FileRangeExt>::with_trailing_match(self.into_file_range(scx), scx, pat)
    }

    // Extends the range to include all instances of the immediately preceding pattern.
    #[inline]
    #[must_use]
    fn with_leading_matches<P>(self, scx: &SpanFileCx, pat: P) -> FileRange
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        <FileRange as FileRangeExt>::with_leading_matches(self.into_file_range(scx), scx, pat)
    }

    // Extends the range to include all instances of the immediately proceeding pattern.
    #[inline]
    #[must_use]
    fn with_trailing_matches(self, scx: &SpanFileCx, pat: impl Pattern) -> FileRange {
        <FileRange as FileRangeExt>::with_trailing_matches(self.into_file_range(scx), scx, pat)
    }

    /// Extends the range to include the remainder of it's first and last line if the
    /// remaining characters on those lines are only whitespace.
    ///
    /// If the range is extended the line terminator will also be included if it exists.
    /// This function is not idempotent and multiple calls may extend the range to include
    /// multiple subsequent lines.
    #[inline]
    #[must_use]
    fn extend_to_line_if_ws(self, scx: &SpanFileCx) -> FileRange {
        <FileRange as FileRangeExt>::extend_to_line_if_ws(self.into_file_range(scx), scx)
    }
}
impl FileRangeExt for FileRange {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_range_between(self, scx: &SpanFileCx, sp: impl ToSpanData) -> Option<FileRange> {
        #[inline]
        #[cfg_attr(debug_assertions, track_caller)]
        fn f(self_: FileRange, scx: &SpanFileCx, sp: SpanData) -> Option<FileRange> {
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
    fn extend_start_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (pos <= self.start).then_some(pos..self.end)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn extend_end_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (pos >= self.end).then_some(self.start..pos)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn shrink_start_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (self.start <= pos && pos <= self.end).then_some(pos..self.end)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn shrink_end_to(self, scx: &SpanFileCx, pos: PosWithCtxt) -> Option<FileRange> {
        debug_assert_eq!(scx.ctxt, pos.ctxt);
        let file = scx.file();
        let pos = RelativeBytePos(pos.pos.0.wrapping_sub(file.start_pos.0));
        scx.dbg_check_range(None, pos..pos);
        (self.start <= pos && pos <= self.end).then_some(self.start..pos)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    #[allow(clippy::manual_map, reason = "track_caller doesn't work through `map`")]
    fn map_range_text(self, scx: &SpanFileCx, f: impl FnOnce(&str) -> Option<&str>) -> Option<FileRange> {
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
        scx: &SpanFileCx,
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

    fn with_leading_whitespace(self, scx: &SpanFileCx) -> FileRange {
        let src = scx.file_text();
        if src.get(self.clone().into_slice_idx()).is_none() {
            return self;
        }
        let sf = scx.file();

        let mut trimmed_lf = false;
        let text_before = src[..self.start.to_usize()].trim_end_matches(|c: char| {
            trimmed_lf |= c == '\n';
            is_whitespace(c)
        });
        if trimmed_lf
            && let line_starts = sf.lines()
            && let post_search_line = line_starts.partition_point(|&pos| pos.to_usize() <= text_before.len())
            // `get` can fail if `line_starts` is missing the starting zero.
            // Just start the search at the beginning in that case.
            && let search_start = line_starts.get(post_search_line - 1).map_or(0, |&x| x.to_usize())
            && ends_with_line_comment_or_broken(&text_before[search_start..])
            // Is there anything after the range on the same line?
            && !src[self.end.to_usize()..].chars().take_while(|&c| c != '\n').all(is_whitespace)
        {
            self
        } else {
            RelativeBytePos::from_usize(text_before.len())..self.end
        }
    }

    fn with_leading_match<P>(self, scx: &SpanFileCx, pat: P) -> Option<FileRange>
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        scx.get_text(..self.start)
            .and_then(|s| s.strip_suffix(pat))
            .map(|s| RelativeBytePos::from_usize(s.len())..self.end)
    }

    fn with_trailing_match(self, scx: &SpanFileCx, pat: impl Pattern) -> Option<FileRange> {
        scx.get_text(self.end..)
            .and_then(|s| s.strip_prefix(pat))
            .map(|s| self.start..RelativeBytePos::from_usize(scx.text.len() - s.len()))
    }

    fn with_leading_matches<P>(self, scx: &SpanFileCx, pat: P) -> FileRange
    where
        P: Pattern,
        for<'a> P::Searcher<'a>: ReverseSearcher<'a>,
    {
        scx.get_text(..self.start).map_or(self.clone(), |s| {
            RelativeBytePos::from_usize(s.trim_end_matches(pat).len())..self.end
        })
    }

    fn with_trailing_matches(self, scx: &SpanFileCx, pat: impl Pattern) -> FileRange {
        scx.get_text(self.end..).map_or(self.clone(), |s| {
            self.start..RelativeBytePos::from_usize(scx.text.len() - s.trim_start_matches(pat).len())
        })
    }

    fn extend_to_line_if_ws(self, scx: &SpanFileCx) -> FileRange {
        let text = scx.file_text();
        if text.get(self.clone().into_slice_idx()).is_some()
            && let pre = text[..self.start.to_usize()].trim_end_matches(|c| is_whitespace(c) && c != '\n')
            && (pre.is_empty() || pre.ends_with('\n'))
            && let post = text[self.end.to_usize()..].trim_start_matches(|c| is_whitespace(c) && c != '\n')
            && let Some(post) = post.strip_suffix('\n').or(post.is_empty().then_some(post))
        {
            RelativeBytePos::from_usize(pre.len())..RelativeBytePos::from_usize(text.len() - post.len())
        } else {
            self
        }
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
    /// # use clippy_utils::source::v2::StrExt as _;
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
    /// # use clippy_utils::source::v2::StrExt as _;
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
    /// # use clippy_utils::source::v2::StrExt as _;
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
    /// # use clippy_utils::source::v2::StrExt as _;
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
