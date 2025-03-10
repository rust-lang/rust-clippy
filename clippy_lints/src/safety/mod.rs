mod proper_safety_comment;

use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// It requires proper safety comments at the barrier of [Unsafety](https://doc.rust-lang.org/reference/unsafety.html).
    /// This includes any part of the [code that needs to satisfy extra safety conditions](https://doc.rust-lang.org/reference/unsafe-keyword.html):
    ///
    /// - unsafe blocks (`unsafe {}`)
    /// - unsafe trait implementations (`unsafe impl`)
    /// - unsafe external blocks (`unsafe extern`)
    /// - unsafe attributes (`#[unsafe(attr)]`)
    ///
    /// Safety comments are [non-doc line comments](https://doc.rust-lang.org/reference/comments.html) starting with `SAFETY:`:
    ///
    /// ```no_run
    /// // SAFETY: A safety comment
    /// // that can cover
    /// // multiple lines.
    /// ```
    ///
    /// Furthermore, it detects unnecessary safety comments for non-critical blocks, trait implementations and attributes. However, there can be false negatives.
    ///
    /// [Code that defines extra safety conditions](https://doc.rust-lang.org/reference/unsafe-keyword.html) is covered by [`clippy::missing_safety_doc`](https://rust-lang.github.io/rust-clippy/master/index.html#missing_safety_doc) and [`clippy::unnecessary_safety_doc`](https://rust-lang.github.io/rust-clippy/master/index.html#unnecessary_safety_doc)
    ///
    /// ### Why restrict this?
    ///
    /// Breaking the safety barrier should not be done carelessly.
    /// Proper documentation should be provided as to why each unsafe operation does not introduce [undefined behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html).
    /// Thinking about these safety requirements and writing them down can prevent incorrect implementations.
    /// On the other hand, unnecessary safety comments are confusing and should not exist.
    ///
    /// ### Example
    ///
    /// ```no_run
    /// unsafe fn f1() {}
    /// fn f2() {
    ///     unsafe { f1() }
    /// }
    ///
    /// unsafe trait A {}
    /// unsafe impl A for () {}
    ///
    /// unsafe extern {
    ///     pub fn g1();
    ///     pub unsafe fn g2();
    ///     pub safe fn g3();
    /// }
    ///
    /// #[unsafe(no_mangle)]
    /// fn h() {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```no_run
    /// unsafe fn f1() {}
    /// fn f2() {
    ///     unsafe {
    ///         // SAFETY: ...
    ///         f1()
    ///     }
    /// }
    ///
    /// unsafe trait A {}
    /// // SAFETY: ...
    /// unsafe impl A for () {}
    ///
    /// // SAFETY: ...
    /// unsafe extern {
    ///     // SAFETY: ...
    ///     pub fn g1();
    ///     // SAFETY: ...
    ///     pub unsafe fn g2();
    ///     // SAFETY: ...
    ///     pub safe fn g3();
    /// }
    ///
    /// // SAFETY: ...
    /// #[unsafe(no_mangle)]
    /// fn h() {}
    /// ```
    #[clippy::version = "1.85.0"]
    pub PROPER_SAFETY_COMMENT,
    restriction,
    "requires proper safety comments at the barrier of unsafety"
}

pub struct Safety;

impl_lint_pass!(Safety => [
    PROPER_SAFETY_COMMENT,
]);

impl EarlyLintPass for Safety {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &rustc_ast::Attribute) {
        proper_safety_comment::check_attribute(cx, attr);
    }

    fn check_block(&mut self, cx: &EarlyContext<'_>, block: &rustc_ast::Block) {
        proper_safety_comment::check_block(cx, block);
    }

    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &rustc_ast::Item) {
        proper_safety_comment::check_item(cx, item);
    }
}
