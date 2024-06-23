//! Utils for extracting, inspecting or transforming source code

#![allow(clippy::module_name_repetitions)]

use rustc_ast::{LitKind, StrStyle};
use rustc_errors::Applicability;
use rustc_hir::{BlockCheckMode, Expr, ExprKind, UnsafeSource};
use rustc_lexer::{LiteralKind, TokenKind, tokenize};
use rustc_lint::{EarlyContext, LateContext};
use rustc_middle::ty::TyCtxt;
use rustc_session::Session;
use rustc_span::source_map::{SourceMap, original_sp};
use rustc_span::{
    BytePos, DUMMY_SP, FileNameDisplayPreference, Pos, RelativeBytePos, SourceFile, SourceFileAndLine, Span, SpanData,
    SyntaxContext, hygiene,
};
use std::borrow::{Borrow, Cow};
use std::fmt;
use std::ops::{Deref, Index, Range};
use std::sync::Arc;

pub trait HasSourceMap {
    #[must_use]
    fn source_map(&self) -> &SourceMap;
}
impl HasSourceMap for SourceMap {
    #[inline]
    fn source_map(&self) -> &SourceMap {
        self
    }
}
impl HasSourceMap for Session {
    #[inline]
    fn source_map(&self) -> &SourceMap {
        self.source_map()
    }
}
impl HasSourceMap for TyCtxt<'_> {
    #[inline]
    fn source_map(&self) -> &SourceMap {
        self.sess.source_map()
    }
}
impl HasSourceMap for EarlyContext<'_> {
    #[inline]
    fn source_map(&self) -> &SourceMap {
        ::rustc_lint::LintContext::sess(self).source_map()
    }
}
impl HasSourceMap for LateContext<'_> {
    #[inline]
    fn source_map(&self) -> &SourceMap {
        self.tcx.sess.source_map()
    }
}

/// A type which can be treated as a span for the purpose of retrieving the source text.
pub trait SpanLike: Sized {
    #[must_use]
    fn into_range(self) -> Range<BytePos>;
    #[must_use]
    fn into_span_data(self) -> SpanData;
    #[must_use]
    fn from_span_data(data: SpanData) -> Self;
}
impl SpanLike for Span {
    #[inline]
    fn into_range(self) -> Range<BytePos> {
        let data = self.data();
        data.lo..data.hi
    }
    #[inline]
    fn into_span_data(self) -> SpanData {
        self.data()
    }
    #[inline]
    fn from_span_data(data: SpanData) -> Self {
        Span::new(data.lo, data.hi, data.ctxt, data.parent)
    }
}
impl SpanLike for SpanData {
    #[inline]
    fn into_range(self) -> Range<BytePos> {
        self.lo..self.hi
    }
    #[inline]
    fn into_span_data(self) -> SpanData {
        self
    }
    #[inline]
    fn from_span_data(data: SpanData) -> Self {
        data
    }
}
impl SpanLike for Range<BytePos> {
    #[inline]
    fn into_range(self) -> Range<BytePos> {
        self
    }
    #[inline]
    fn into_span_data(self) -> SpanData {
        SpanData {
            lo: self.start,
            hi: self.end,
            ctxt: SyntaxContext::root(),
            parent: None,
        }
    }
    #[inline]
    fn from_span_data(data: SpanData) -> Self {
        data.lo..data.hi
    }
}

/// Helper functions to interact with the source text of a span.
pub trait SpanExt: SpanLike {
    /// Attempts to get a handle to the source file and the text range within that file. Returns
    /// `None` if the source text is not available.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_source_range(self, sm: &impl HasSourceMap) -> Option<SourceFileRange<'_>> {
        SourceFileRange::new(sm.source_map(), self.into_range())
    }

    /// Attempts to get a handle to the source text. Returns `None` if the source text could not be
    /// accessed for any reason.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn get_source_text(self, sm: &impl HasSourceMap) -> Option<SourceText> {
        SourceFileRange::new(sm.source_map(), self.into_range()).and_then(SourceFileRange::into_text)
    }

    /// Checks if the referenced source text satisfies the given predicate. Returns `false` if the
    /// source text could not be accessed for any reason.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of a single source file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    fn check_source_text(self, sm: &impl HasSourceMap, pred: impl FnOnce(&str) -> bool) -> bool {
        SourceFileRange::new(sm.source_map(), self.into_range()).is_some_and(|x| x.current_text().is_some_and(pred))
    }

    /// Maps the range of the current span. Returns `None` if either the given function returns
    /// `None`, or the source text could not be accessed for any reason.
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
        sm: &'sm impl HasSourceMap,
        f: impl for<'a> FnOnce(&'a mut SourceFileRange<'sm>) -> Option<&'a mut SourceFileRange<'sm>>,
    ) -> Option<Self> {
        let data = self.into_span_data();
        if let Some(mut range) = SourceFileRange::new(sm.source_map(), data.lo..data.hi)
            && let Some(range) = f(&mut range)
            // Make sure the span is valid. Always passes with debug assertions as
            // `SourceFileRange` panics whenever a bad range is set.
            && range.as_text().is_some()
        {
            let range = range.source_range();
            Some(Self::from_span_data(SpanData {
                lo: range.start,
                hi: range.end,
                ..data
            }))
        } else {
            None
        }
    }
}
impl<T: SpanLike> SpanExt for T {}

mod source_text {
    use core::slice::SliceIndex;
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
        /// Gets the text of the given file. Returns `None` if the file's text could not be loaded.
        #[must_use]
        pub fn new(sm: &SourceMap, file: Arc<SourceFile>) -> Option<Self> {
            if !sm.ensure_source_file_source_present(&file) {
                return None;
            }
            let text: *const str = if let Some(text) = &file.src {
                &raw const ***text
            } else if let Some(src) = file.external_src.get()
                // `get` or `freeze` must be used to access the lock.
                // Since `ensure_source_file_source_present` calls `freeze` when loading the source
                // we use `get` to avoid the extra load.
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

        /// Applies an indexing operation to the contained string. Returns `None` if the index is
        /// not valid.
        #[inline]
        #[must_use]
        pub fn with_index(mut self, idx: impl SliceIndex<str, Output = str>) -> Option<Self> {
            self.text = self.as_str().get(idx)?;
            Some(self)
        }

        /// Gets the source file containing the text.
        #[inline]
        #[must_use]
        pub fn file(&self) -> &Arc<SourceFile> {
            &self.file
        }
    }
}
pub use self::source_text::SourceText;
impl SourceText {
    /// Converts this into an owned string.
    #[inline]
    pub fn to_owned(&self) -> String {
        self.as_str().to_owned()
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

#[cfg(not(debug_assertions))]
fn dbg_check_range(_: &SourceFileRange<'_>, _: Range<RelativeBytePos>) {}

#[cfg(debug_assertions)]
#[track_caller]
fn dbg_check_range(current: &SourceFileRange<'_>, new: Range<RelativeBytePos>) {
    use core::fmt::Write;

    if new.start > new.end
        || new.end > current.file().source_len
        || current
            .file_text()
            .get(new.start.to_usize()..new.end.to_usize())
            .is_none()
    {
        let mut msg = String::with_capacity(512);

        let file = current.file();
        let new_start = BytePos(new.start.0.wrapping_add(file.start_pos.0));
        let new_end = BytePos(new.end.0.wrapping_add(file.start_pos.0));

        let _ = write!(
            msg,
            "error setting the requested range `{:?}: ",
            // signed numbers will better show most errors.
            new.start.0.cast_signed()..new.end.0.cast_signed(),
        );
        if new_start > new_end {
            msg.push_str("the start and end overlap");
        } else if new_start < file.start_pos || file.start_pos.0 + file.source_len.0 < new_end.0 {
            let _ = write!(
                msg,
                "the bounds are outside the current file (len: {})",
                file.source_len.to_u32(),
            );
        } else {
            msg.push_str("the ends are not on UTF-8 boundaries");
        }

        let start = BytePos::from_u32(current.range.start.to_u32()) + file.start_pos;
        let end = BytePos::from_u32(current.range.end.to_u32()) + file.start_pos;
        let (start_line, start_col, _) = file.lookup_file_pos_with_col_display(start);
        let (end_line, end_col, _) = file.lookup_file_pos_with_col_display(end);
        let _ = write!(
            msg,
            "\n  current: {}:{}:{}: {}:{}",
            file.name.display(FileNameDisplayPreference::Local),
            start_line,
            start_col.to_u32(),
            end_line,
            end_col.to_u32(),
        );

        let files = current.sm.files();
        let mut print_loc = |label: &str, pos: BytePos| {
            if files
                .last()
                .is_some_and(|f| pos <= f.start_pos + BytePos::from_u32(f.source_len.to_u32()))
            {
                let file = current.sm.lookup_source_file(pos);
                let file_name = file.name.display(FileNameDisplayPreference::Local);
                let rel_pos = RelativeBytePos::from_u32((pos - file.start_pos).to_u32());
                if let Some(text) = SourceText::new(current.sm, file.clone())
                    && text.get(rel_pos.to_usize()..).is_some()
                {
                    let (line, col, _) = file.lookup_file_pos_with_col_display(pos);
                    let _ = write!(msg, "\n  {label}: {file_name}:{line}:{}", col.to_u32());
                } else {
                    let line = file.lookup_line(rel_pos).unwrap_or(0);
                    let offset = rel_pos - file.lines()[line];
                    let _ = write!(msg, "\n  {label}: {file_name}:{} + {}", line + 1, offset.to_u32());
                }
            } else {
                let _ = write!(msg, "\n  {label}: not a file");
            }
        };
        if new.start != current.range.start {
            print_loc("new start", new_start);
        }
        if new.end != current.range.end {
            print_loc("new end", new_end);
        }

        std::panic::panic_any(msg);
    }
}

/// Handle to a source file's text and a range within that file.
///
/// With debug assertions the range is checked to be a valid substring of the source text. Without
/// assertions `None` will be returned from various functions when accessing the substring of the
/// source text fails.
#[derive(Clone)]
pub struct SourceFileRange<'sm> {
    file: SourceText,
    range: Range<RelativeBytePos>,
    #[cfg(debug_assertions)]
    sm: &'sm SourceMap,
    #[cfg(not(debug_assertions))]
    sm: core::marker::PhantomData<&'sm SourceMap>,
}
impl<'sm> SourceFileRange<'sm> {
    /// Attempts to get a handle to the source file and the text range within that file. Returns
    /// `None` if the source text is not available.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not cross multiple files.
    /// * Does not exceed the bounds of the source map.
    /// * Lies on a UTF-8 boundary.
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn new(sm: &'sm SourceMap, range: Range<BytePos>) -> Option<Self> {
        let start = sm.lookup_byte_offset(range.start);
        let end = RelativeBytePos::from_u32(range.end.to_u32() - start.sf.start_pos.to_u32());
        let mut res = Self {
            file: SourceText::new(sm, start.sf)?,
            range: RelativeBytePos::from_u32(0)..RelativeBytePos::from_u32(0),
            #[cfg(debug_assertions)]
            sm,
            #[cfg(not(debug_assertions))]
            sm: core::marker::PhantomData,
        };
        res.set_range(RelativeBytePos::from_u32(start.pos.to_u32())..end);
        Some(res)
    }

    /// Gets a reference to the containing source file.
    #[inline]
    #[must_use]
    pub fn file(&self) -> &SourceFile {
        self.file.file()
    }

    /// Checks if this range is part of the same file as another range.
    #[inline]
    #[must_use]
    pub fn is_same_file_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.file.file(), other.file.file())
    }

    /// Gets the whole source text of the file.
    #[inline]
    #[must_use]
    pub fn file_text(&self) -> &str {
        self.file.as_str()
    }

    /// Gets the source text contained within the current range. Returns `None` if the current range
    /// is not valid.
    #[inline]
    #[must_use]
    pub fn current_text(&self) -> Option<&str> {
        // The range will have already been validated if debug assertions are enabled.
        self.file_text()
            .get(self.range.start.to_usize()..self.range.end.to_usize())
    }

    /// Gets the current range in the file.
    #[inline]
    #[must_use]
    pub fn range(&self) -> Range<RelativeBytePos> {
        self.range.clone()
    }

    /// Gets the current range as a range in the source map.
    #[inline]
    #[must_use]
    pub fn source_range(&self) -> Range<BytePos> {
        BytePos(self.range.start.0 + self.file().start_pos.0)..BytePos(self.range.end.0 + self.file().start_pos.0)
    }

    /// Converts this into handle which acts as a `&str`. Returns `None` if the current range is
    /// ill-formed.
    #[inline]
    #[must_use]
    pub fn into_text(self) -> Option<SourceText> {
        self.file
            .with_index(self.range.start.to_usize()..self.range.end.to_usize())
    }

    /// Converts this into handle which acts as a `&str`. Returns `None` if the current range is
    /// ill-formed.
    #[inline]
    #[must_use]
    pub fn as_text(&self) -> Option<SourceText> {
        self.file
            .clone()
            .with_index(self.range.start.to_usize()..self.range.end.to_usize())
    }

    /// Sets the current range in the file.
    ///
    /// With debug assertions this will assert that the range:
    /// * Does not start after it's end.
    /// * Does not exceed the bounds of the file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set_range(&mut self, range: Range<RelativeBytePos>) -> &mut Self {
        dbg_check_range(self, range.clone());
        self.range = range;
        self
    }

    /// Sets the current range to the range between the current range and the given range.
    /// Does nothing and returns `None` if the ranges overlap.
    ///
    /// With debug assertions enabled this will assert that the given range:
    /// * Is within the same file as the current range.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set_range_between_other(&mut self, other: impl SpanLike) -> Option<&mut Self> {
        fn f<'a, 'sm>(
            self_: &'a mut SourceFileRange<'sm>,
            other: Range<BytePos>,
        ) -> Option<&'a mut SourceFileRange<'sm>> {
            let file = self_.file();
            let other = other.into_range();
            let other = RelativeBytePos(other.start.0.wrapping_sub(file.start_pos.0))
                ..RelativeBytePos(other.end.0.wrapping_sub(file.start_pos.0));
            dbg_check_range(self_, other.clone());
            if self_.range.end.0.cast_signed() < other.start.0.cast_signed() {
                self_.range.start = self_.range.end;
                self_.range.end = other.start;
                Some(self_)
            } else if self_.range.start.0.cast_signed() > other.end.0.cast_signed() {
                self_.range.end = self_.range.start;
                self_.range.start = other.end;
                Some(self_)
            } else {
                None
            }
        }
        f(self, other.into_range())
    }

    /// Sets the start of this range to the given source map position if it's at or before
    /// the current range.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set_start_if_before(&mut self, pos: BytePos) -> Option<&mut Self> {
        let rel_pos = RelativeBytePos(pos.0.wrapping_sub(self.file().start_pos.0));
        dbg_check_range(self, RelativeBytePos(0)..rel_pos);

        // Signed comparison will catch some cross file positions.
        if rel_pos.0.cast_signed() <= self.range.start.0.cast_signed() {
            self.range.start = rel_pos;
            Some(self)
        } else {
            None
        }
    }

    /// Sets the start of this range to the given source map position if it's within the
    /// current range.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set_start_if_within(&mut self, pos: BytePos) -> Option<&mut Self> {
        let rel_pos = RelativeBytePos(pos.0.wrapping_sub(self.file().start_pos.0));
        dbg_check_range(self, RelativeBytePos(0)..rel_pos);
        if self.range.start <= rel_pos && rel_pos <= self.range.end {
            self.range.start = rel_pos;
            Some(self)
        } else {
            None
        }
    }

    /// Sets the end of this range to the given source map position if it's at or after
    /// the current range.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set_end_if_after(&mut self, pos: BytePos) -> Option<&mut Self> {
        let rel_pos = RelativeBytePos(pos.0.wrapping_sub(self.file().start_pos.0));
        dbg_check_range(self, RelativeBytePos(0)..rel_pos);

        // Signed comparison will catch some cross file positions.
        if self.range.end.0.cast_signed() <= rel_pos.0.cast_signed() {
            self.range.end = rel_pos;
            Some(self)
        } else {
            None
        }
    }

    /// Sets the end of this range to the given source map position if it's within the
    /// current range.
    ///
    /// With debug assertions enabled this will assert that the position:
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set_end_if_within(&mut self, pos: BytePos) -> Option<&mut Self> {
        let rel_pos = RelativeBytePos(pos.0.wrapping_sub(self.file().start_pos.0));
        dbg_check_range(self, RelativeBytePos(0)..rel_pos);
        if self.range.start <= rel_pos && rel_pos <= self.range.end {
            self.range.end = rel_pos;
            Some(self)
        } else {
            None
        }
    }

    /// Maps the current range using the given function. Return `None` if the function returns
    /// `None`, or the current range is ill-formed.
    ///
    /// With debug assertions this will assert that the mapped range:
    /// * Does not start after it's end.
    /// * Is within the same file.
    /// * Lies on a UTF-8 boundary.
    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn edit_range(&mut self, f: impl FnOnce(&str, Range<usize>) -> Option<Range<usize>>) -> Option<&mut Self> {
        let range = f(self.file_text(), self.range.start.to_usize()..self.range.end.to_usize());
        let range = range?;
        self.set_range(RelativeBytePos::from_usize(range.start)..RelativeBytePos::from_usize(range.end));
        Some(self)
    }

    /// Trims the whitespace from the start of the range. Returns `None` if the current range is
    /// ill-formed.
    #[must_use]
    pub fn remove_leading_whitespace(&mut self) -> Option<&mut Self> {
        self.edit_range(|src, range| {
            let src = src.get(range.clone())?;
            Some(range.start + (src.len() - src.trim_start().len())..range.end)
        })
    }

    /// Extends the range to include all preceding whitespace characters.
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
    #[must_use]
    pub fn add_leading_whitespace(&mut self) -> Option<&mut Self> {
        let text_before = self.file_text().get(..self.range.start.to_usize())?.trim_end();
        let range = self.range.clone();
        let file = self.file();
        let lines = self.file().lines();

        // First check if extending backwards crosses lines into a comment line.
        let post_search_line = lines.partition_point(|&pos| pos.to_usize() <= text_before.len());
        if let Some(&search_line_end) = lines.get(post_search_line)
            // Did we extend pass a line boundary?
            && search_line_end <= range.start
            && let search_start = lines.get(post_search_line - 1).map_or(0, |&x| x.to_usize())
            && ends_with_line_comment_or_broken(&text_before[search_start..])
            // Next check if there's anything after the current range on the same line.
            && let next_line = lines.partition_point(|&pos| pos < range.end)
            && let next_start = lines.get(next_line).map_or(file.source_len, |&x| x)
            && tokenize(self.file_text().get(range.end.to_usize()..next_start.to_usize())?)
                .any(|t| !matches!(t.kind, TokenKind::Whitespace))
        {
            // Do nothing; removing whitespace would move code into a comment.
        } else {
            self.set_range(RelativeBytePos::from_usize(text_before.len())..range.end);
        }
        Some(self)
    }

    /// Extends the range to include all trailing whitespace.
    #[must_use]
    pub fn add_trailing_whitespace(&mut self) -> Option<&mut Self> {
        self.edit_range(|src, range: Range<usize>| {
            let tail = src.get(range.end..)?;
            Some(range.start..range.end + (tail.len() - tail.trim_end().len()))
        })
    }

    /// Sets the range to that of it's first character if the predicate matches.
    ///
    /// If the predicate returns false or the range is empty then the range will be left unchanged
    /// and `None` will be returned.
    #[must_use]
    pub fn set_to_first_char_if(&mut self, pred: impl FnOnce(char) -> bool) -> Option<&mut Self> {
        self.edit_range(|src, range| {
            let c = src.get(range.clone())?.chars().next()?;
            pred(c).then_some(range.start..range.start + c.len_utf8())
        })
    }

    /// Sets the range to that of it's last character if the predicate matches.
    ///
    /// If the predicate returns false or the range is empty then the range will be left unchanged
    /// and `None` will be returned.
    #[must_use]
    pub fn set_to_last_char_if(&mut self, pred: impl FnOnce(char) -> bool) -> Option<&mut Self> {
        self.edit_range(|src, range| {
            let c = src.get(range.clone())?.chars().next_back()?;
            pred(c).then_some(range.end - c.len_utf8()..range.end)
        })
    }

    /// Gets the indent of the line this range starts on.
    #[must_use]
    pub fn get_line_indent(&self) -> &str {
        let file = self.file.file();
        if let Some(line) = file.lookup_line(self.range.start)
            && let Some(start) = file.lines().get(line)
        {
            let end = file.lines().get(line + 1).map_or(file.source_len, |&x| x);
            let line = &self.file_text()[start.to_usize()..end.to_usize()];
            &line[..line.len() - line.trim_start().len()]
        } else {
            ""
        }
    }
}
impl fmt::Debug for SourceFileRange<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file.file();
        if self.as_text().is_some() {
            let start = BytePos::from_u32(self.range.start.to_u32()) + file.start_pos;
            let end = BytePos::from_u32(self.range.end.to_u32()) + file.start_pos;
            let (start_line, start_col, _) = file.lookup_file_pos_with_col_display(start);
            let (end_line, end_col, _) = file.lookup_file_pos_with_col_display(end);
            write!(
                f,
                "{}:{}:{}: {}:{}",
                file.name.display(FileNameDisplayPreference::Local),
                start_line,
                start_col.to_u32(),
                end_line,
                end_col.to_u32(),
            )
        } else {
            let start_line = file.lookup_line(self.range.start).unwrap_or(0);
            let start_offset = self.range.start - file.lines()[start_line];
            let end_line = file.lookup_line(self.range.end).unwrap_or(0);
            let end_offset = self.range.end - file.lines()[end_line];
            write!(
                f,
                "{}:{} + {}: {} + {}",
                file.name.display(FileNameDisplayPreference::Local),
                start_line,
                start_offset.0,
                end_line,
                end_offset.0,
            )
        }
    }
}

/// Checks if the last token of the string is either a line comment or an incomplete token.
fn ends_with_line_comment_or_broken(text: &str) -> bool {
    let Some(last) = tokenize(text).last() else {
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
pub fn expr_block(
    sm: &impl HasSourceMap,
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
pub fn first_line_of_span(sm: &impl HasSourceMap, span: Span) -> Span {
    first_char_in_first_line(sm, span).map_or(span, |first_char_pos| span.with_lo(first_char_pos))
}

fn first_char_in_first_line(sm: &impl HasSourceMap, span: Span) -> Option<BytePos> {
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
fn line_span(sm: &impl HasSourceMap, span: Span) -> Span {
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
pub fn indent_of(sm: &impl HasSourceMap, span: Span) -> Option<usize> {
    snippet_opt(sm, line_span(sm, span)).and_then(|snip| snip.find(|c: char| !c.is_whitespace()))
}

/// Gets a snippet of the indentation of the line of a span
pub fn snippet_indent(sm: &impl HasSourceMap, span: Span) -> Option<String> {
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
pub fn is_present_in_source(sm: &impl HasSourceMap, span: Span) -> bool {
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
pub fn snippet<'a>(sm: &impl HasSourceMap, span: Span, default: &'a str) -> Cow<'a, str> {
    snippet_opt(sm, span).map_or_else(|| Cow::Borrowed(default), From::from)
}

/// Same as [`snippet`], but it adapts the applicability level by following rules:
///
/// - Applicability level `Unspecified` will never be changed.
/// - If the span is inside a macro, change the applicability level to `MaybeIncorrect`.
/// - If the default value is used and the applicability level is `MachineApplicable`, change it to
///   `HasPlaceholders`
pub fn snippet_with_applicability<'a>(
    sm: &impl HasSourceMap,
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
pub fn snippet_opt(sm: &impl HasSourceMap, span: Span) -> Option<String> {
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
pub fn snippet_block(sm: &impl HasSourceMap, span: Span, default: &str, indent_relative_to: Option<Span>) -> String {
    let snip = snippet(sm, span, default);
    let indent = indent_relative_to.and_then(|s| indent_of(sm, s));
    reindent_multiline(&snip, true, indent)
}

/// Same as `snippet_block`, but adapts the applicability level by the rules of
/// `snippet_with_applicability`.
pub fn snippet_block_with_applicability(
    sm: &impl HasSourceMap,
    span: Span,
    default: &str,
    indent_relative_to: Option<Span>,
    applicability: &mut Applicability,
) -> String {
    let snip = snippet_with_applicability(sm, span, default, applicability);
    let indent = indent_relative_to.and_then(|s| indent_of(sm, s));
    reindent_multiline(&snip, true, indent)
}

pub fn snippet_block_with_context(
    sm: &impl HasSourceMap,
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
pub fn snippet_with_context<'a>(
    sm: &impl HasSourceMap,
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
pub fn expand_past_previous_comma(sm: &impl HasSourceMap, span: Span) -> Span {
    let extended = sm.source_map().span_extend_to_prev_char(span, ',', true);
    extended.with_lo(extended.lo() - BytePos(1))
}

/// Converts `expr` to a `char` literal if it's a `str` literal containing a single
/// character (or a single byte with `ascii_only`)
pub fn str_literal_to_char_literal(
    sm: &impl HasSourceMap,
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
