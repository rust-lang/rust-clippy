mod crosspointer_transmute;
mod transmute_float_to_int;
mod transmute_int_to_bool;
mod transmute_int_to_char;
mod transmute_int_to_float;
mod transmute_ptr_to_ptr;
mod transmute_ptr_to_ref;
mod transmute_ref_to_ref;
mod transmutes_expressible_as_ptr_casts;
mod unsound_collection_transmute;
mod useless_transmute;
mod utils;
mod wrong_transmute;

use clippy_utils::in_constant;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes that can't ever be correct on any
    /// architecture.
    ///
    /// ### Why is this bad?
    /// It's basically guaranteed to be undefined behaviour.
    ///
    /// ### Known problems
    /// When accessing C, users might want to store pointer
    /// sized objects in `extradata` arguments to save an allocation.
    ///
    /// ### Example
    /// ```ignore
    /// let ptr: *const T = core::intrinsics::transmute('x')
    /// ```
    pub WRONG_TRANSMUTE,
    correctness,
    "transmutes that are confusing at best, undefined behaviour at worst and always useless"
}

// FIXME: Move this to `complexity` again, after #5343 is fixed
declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes to the original type of the object
    /// and transmutes that could be a cast.
    ///
    /// ### Why is this bad?
    /// Readability. The code tricks people into thinking that
    /// something complex is going on.
    ///
    /// ### Example
    /// ```rust,ignore
    /// core::intrinsics::transmute(t); // where the result type is the same as `t`'s
    /// ```
    pub USELESS_TRANSMUTE,
    nursery,
    "transmutes that have the same to and from types or could be a cast/coercion"
}

// FIXME: Merge this lint with USELESS_TRANSMUTE once that is out of the nursery.
declare_clippy_lint! {
    /// ### What it does
    ///Checks for transmutes that could be a pointer cast.
    ///
    /// ### Why is this bad?
    /// Readability. The code tricks people into thinking that
    /// something complex is going on.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # let p: *const [i32] = &[];
    /// unsafe { std::mem::transmute::<*const [i32], *const [u16]>(p) };
    /// ```
    /// Use instead:
    /// ```rust
    /// # let p: *const [i32] = &[];
    /// p as *const [u16];
    /// ```
    pub TRANSMUTES_EXPRESSIBLE_AS_PTR_CASTS,
    complexity,
    "transmutes that could be a pointer cast"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes between a type `T` and `*T`.
    ///
    /// ### Why is this bad?
    /// It's easy to mistakenly transmute between a type and a
    /// pointer to that type.
    ///
    /// ### Example
    /// ```rust,ignore
    /// core::intrinsics::transmute(t) // where the result type is the same as
    ///                                // `*t` or `&t`'s
    /// ```
    pub CROSSPOINTER_TRANSMUTE,
    complexity,
    "transmutes that have to or from types that are a pointer to the other"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes from a pointer to a reference.
    ///
    /// ### Why is this bad?
    /// This can always be rewritten with `&` and `*`.
    ///
    /// ### Known problems
    /// - `mem::transmute` in statics and constants is stable from Rust 1.46.0,
    /// while dereferencing raw pointer is not stable yet.
    /// If you need to do this in those places,
    /// you would have to use `transmute` instead.
    ///
    /// ### Example
    /// ```rust,ignore
    /// unsafe {
    ///     let _: &T = std::mem::transmute(p); // where p: *const T
    /// }
    ///
    /// // can be written:
    /// let _: &T = &*p;
    /// ```
    pub TRANSMUTE_PTR_TO_REF,
    complexity,
    "transmutes from a pointer to a reference type"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes from an integer to a `char`.
    ///
    /// ### Why is this bad?
    /// Not every integer is a Unicode scalar value.
    ///
    /// ### Known problems
    /// - [`from_u32`] which this lint suggests using is slower than `transmute`
    /// as it needs to validate the input.
    /// If you are certain that the input is always a valid Unicode scalar value,
    /// use [`from_u32_unchecked`] which is as fast as `transmute`
    /// but has a semantically meaningful name.
    /// - You might want to handle `None` returned from [`from_u32`] instead of calling `unwrap`.
    ///
    /// [`from_u32`]: https://doc.rust-lang.org/std/char/fn.from_u32.html
    /// [`from_u32_unchecked`]: https://doc.rust-lang.org/std/char/fn.from_u32_unchecked.html
    ///
    /// ### Example
    /// ```rust
    /// let x = 1_u32;
    /// unsafe {
    ///     let _: char = std::mem::transmute(x); // where x: u32
    /// }
    ///
    /// // should be:
    /// let _ = std::char::from_u32(x).unwrap();
    /// ```
    pub TRANSMUTE_INT_TO_CHAR,
    complexity,
    "transmutes from an integer to a `char`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes from a `&[u8]` to a `&str`.
    ///
    /// ### Why is this bad?
    /// Not every byte slice is a valid UTF-8 string.
    ///
    /// ### Known problems
    /// - [`from_utf8`] which this lint suggests using is slower than `transmute`
    /// as it needs to validate the input.
    /// If you are certain that the input is always a valid UTF-8,
    /// use [`from_utf8_unchecked`] which is as fast as `transmute`
    /// but has a semantically meaningful name.
    /// - You might want to handle errors returned from [`from_utf8`] instead of calling `unwrap`.
    ///
    /// [`from_utf8`]: https://doc.rust-lang.org/std/str/fn.from_utf8.html
    /// [`from_utf8_unchecked`]: https://doc.rust-lang.org/std/str/fn.from_utf8_unchecked.html
    ///
    /// ### Example
    /// ```rust
    /// let b: &[u8] = &[1_u8, 2_u8];
    /// unsafe {
    ///     let _: &str = std::mem::transmute(b); // where b: &[u8]
    /// }
    ///
    /// // should be:
    /// let _ = std::str::from_utf8(b).unwrap();
    /// ```
    pub TRANSMUTE_BYTES_TO_STR,
    complexity,
    "transmutes from a `&[u8]` to a `&str`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes from an integer to a `bool`.
    ///
    /// ### Why is this bad?
    /// This might result in an invalid in-memory representation of a `bool`.
    ///
    /// ### Example
    /// ```rust
    /// let x = 1_u8;
    /// unsafe {
    ///     let _: bool = std::mem::transmute(x); // where x: u8
    /// }
    ///
    /// // should be:
    /// let _: bool = x != 0;
    /// ```
    pub TRANSMUTE_INT_TO_BOOL,
    complexity,
    "transmutes from an integer to a `bool`"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes from an integer to a float.
    ///
    /// ### Why is this bad?
    /// Transmutes are dangerous and error-prone, whereas `from_bits` is intuitive
    /// and safe.
    ///
    /// ### Example
    /// ```rust
    /// unsafe {
    ///     let _: f32 = std::mem::transmute(1_u32); // where x: u32
    /// }
    ///
    /// // should be:
    /// let _: f32 = f32::from_bits(1_u32);
    /// ```
    pub TRANSMUTE_INT_TO_FLOAT,
    complexity,
    "transmutes from an integer to a float"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes from a float to an integer.
    ///
    /// ### Why is this bad?
    /// Transmutes are dangerous and error-prone, whereas `to_bits` is intuitive
    /// and safe.
    ///
    /// ### Example
    /// ```rust
    /// unsafe {
    ///     let _: u32 = std::mem::transmute(1f32);
    /// }
    ///
    /// // should be:
    /// let _: u32 = 1f32.to_bits();
    /// ```
    pub TRANSMUTE_FLOAT_TO_INT,
    complexity,
    "transmutes from a float to an integer"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes from a pointer to a pointer, or
    /// from a reference to a reference.
    ///
    /// ### Why is this bad?
    /// Transmutes are dangerous, and these can instead be
    /// written as casts.
    ///
    /// ### Example
    /// ```rust
    /// let ptr = &1u32 as *const u32;
    /// unsafe {
    ///     // pointer-to-pointer transmute
    ///     let _: *const f32 = std::mem::transmute(ptr);
    ///     // ref-ref transmute
    ///     let _: &f32 = std::mem::transmute(&1u32);
    /// }
    /// // These can be respectively written:
    /// let _ = ptr as *const f32;
    /// let _ = unsafe{ &*(&1u32 as *const u32 as *const f32) };
    /// ```
    pub TRANSMUTE_PTR_TO_PTR,
    pedantic,
    "transmutes from a pointer to a pointer / a reference to a reference"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for transmutes between collections whose
    /// types have different ABI, size or alignment.
    ///
    /// ### Why is this bad?
    /// This is undefined behavior.
    ///
    /// ### Known problems
    /// Currently, we cannot know whether a type is a
    /// collection, so we just lint the ones that come with `std`.
    ///
    /// ### Example
    /// ```rust
    /// // different size, therefore likely out-of-bounds memory access
    /// // You absolutely do not want this in your code!
    /// unsafe {
    ///     std::mem::transmute::<_, Vec<u32>>(vec![2_u16])
    /// };
    /// ```
    ///
    /// You must always iterate, map and collect the values:
    ///
    /// ```rust
    /// vec![2_u16].into_iter().map(u32::from).collect::<Vec<_>>();
    /// ```
    pub UNSOUND_COLLECTION_TRANSMUTE,
    correctness,
    "transmute between collections of layout-incompatible types"
}

declare_lint_pass!(Transmute => [
    CROSSPOINTER_TRANSMUTE,
    TRANSMUTE_PTR_TO_REF,
    TRANSMUTE_PTR_TO_PTR,
    USELESS_TRANSMUTE,
    WRONG_TRANSMUTE,
    TRANSMUTE_INT_TO_CHAR,
    TRANSMUTE_BYTES_TO_STR,
    TRANSMUTE_INT_TO_BOOL,
    TRANSMUTE_INT_TO_FLOAT,
    TRANSMUTE_FLOAT_TO_INT,
    UNSOUND_COLLECTION_TRANSMUTE,
    TRANSMUTES_EXPRESSIBLE_AS_PTR_CASTS,
]);

impl<'tcx> LateLintPass<'tcx> for Transmute {
    #[allow(clippy::similar_names, clippy::too_many_lines)]
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if_chain! {
            if let ExprKind::Call(path_expr, args) = e.kind;
            if let ExprKind::Path(ref qpath) = path_expr.kind;
            if let Some(def_id) = cx.qpath_res(qpath, path_expr.hir_id).opt_def_id();
            if cx.tcx.is_diagnostic_item(sym::transmute, def_id);
            then {
                // Avoid suggesting from/to bits and dereferencing raw pointers in const contexts.
                // See https://github.com/rust-lang/rust/issues/73736 for progress on making them `const fn`.
                // And see https://github.com/rust-lang/rust/issues/51911 for dereferencing raw pointers.
                let const_context = in_constant(cx, e.hir_id);

                let from_ty = cx.typeck_results().expr_ty(&args[0]);
                let to_ty = cx.typeck_results().expr_ty(e);

                // If useless_transmute is triggered, the other lints can be skipped.
                if useless_transmute::check(cx, e, from_ty, to_ty, args) {
                    return;
                }

                let mut linted = wrong_transmute::check(cx, e, from_ty, to_ty);
                linted |= crosspointer_transmute::check(cx, e, from_ty, to_ty);
                linted |= transmute_ptr_to_ref::check(cx, e, from_ty, to_ty, args, qpath);
                linted |= transmute_int_to_char::check(cx, e, from_ty, to_ty, args);
                linted |= transmute_ref_to_ref::check(cx, e, from_ty, to_ty, args, const_context);
                linted |= transmute_ptr_to_ptr::check(cx, e, from_ty, to_ty, args);
                linted |= transmute_int_to_bool::check(cx, e, from_ty, to_ty, args);
                linted |= transmute_int_to_float::check(cx, e, from_ty, to_ty, args, const_context);
                linted |= transmute_float_to_int::check(cx, e, from_ty, to_ty, args, const_context);
                linted |= unsound_collection_transmute::check(cx, e, from_ty, to_ty);

                if !linted {
                    transmutes_expressible_as_ptr_casts::check(cx, e, from_ty, to_ty, args);
                }
            }
        }
    }
}
