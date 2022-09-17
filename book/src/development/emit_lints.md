# Emitting a lint

Once we have [defined a lint](define_lints.md), written [UI tests](write_tests.md)
and chosen [the lint pass](lint_passes.md) for the lint, we can begin the
implementation of the lint logic so that we can emit the lint and gradually work
towards a lint that behaves as expected.

We will look into how we can emit alint for both `LateLintPass` and `EarlyLintPass`.
Note that we will not go into concrete implementation of a lint logic in this
chapter. We will go into details in later chapters as well as in two examples
of real Clippy lints.

## `LateLintPass`

To emit a lint with `LateLintPass`, we must implement it for the lint that we have
declared. Take a look at the [LateLintPass][late_lint_pass] documentation, which
provides an abundance of methods that we can implement for our lint.

```rust
pub trait LateLintPass<'tcx>: LintPass {
    // Trait methods
}
```

By far the most common method used for Clippy lints is [`check_expr` method][late_check_expr],
this is likely because Rust is an expression language and, more often than not,
the lint we want to work on must examine expressions.

> _Note:_ If you don't fully understand what expressions are in Rust,
> take a look at the official documentation on [expressions][rust_expressions]

Other common ones include [`check_fn` method][late_check_fn] and
[`check_item` method][late_check_item]. We choose to implement whichever trait
method based on what we need for the lint at hand.

### Implement Trait Method in `LateLintPass`

Assume that we have added and defined a `BarExpressions` lint, we could write
down a skeleton for this lint as the following:

```rust
impl<'tcx> LateLintPass<'tcx> for BarExpressions {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>)  {}
}
```

### Emitting Lint in `LateLintPass`

Inside the trait method that we implement, we can write down the lint logic
and the emit the lint with suggestions.

Clippy's [diagnostics] provides quite a few diagnositc functions that we can
use to emit lints. Take a look at the documentation to pick one that suits
your lint's needs the best. Some common ones you will encounter in the Clippy
repository includes:

- [span_lint_and_help]: emits lint and provides a helpful message
- [span_lint_and_sugg]: emits lint and provides a suggestion to fix the code

```rust
impl<'tcx> LateLintPass<'tcx> for BarExpressions {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>)  {
        // Imagine a `some_bar_expr_logic` that checks for requirements for emitting the lint
        if some_bar_expr_logic(expr) {
            span_lint_and_help(
                cx,
                FOO_FUNCTIONS,
                expr.span,
                "message on why the lint is emitted",
                None,
                "message that provides a helpful suggestion",
            );
        }
    }
}
```

> Note: According to [the rustc-dev-guide], the message should be matter of fact and avoid
> capitalization and periods, unless multiple sentences are needed. When code or
> an identifier must appear in a message or label, it should be surrounded with
> single grave accents \`.

## `EarlyLintPass`

Emitting a lint with `EarlyLintPass` follows the same logic like `LateLintPass`.
Take a look at the [EarlyLintPass][early_lint_pass] documentation, which
provides an abundance of methods that we can implement for our lint.

```rust
pub trait EarlyLintPass: LintPass {
    // Trait methods
}
```

### Implement Trait Method in `EarlyLintPass`

Similar to `LateLintPass`, we pick a trait method to implement if we choose
`EarlyLintPass` for a specific lint we want to design. Using `FooFunctions`
as an example, we will use [`check_fn`][early_check_fn] since it gives us
access to various information about the function that is currently being checked.

```rust
impl EarlyLintPass for FooFunctions {
    fn check_fn(&mut self, cx: &EarlyContext<'_>, fn_kind: FnKind<'_>, span: Span, _: NodeId) {
        // TODO: Emit lint here
    }
}
```

### Emitting Lint in `EarlyLintPass`

To emit the lint with `EarlyLintPass`, use `span_lint_and_help`
to provide an extra help message. This is how it looks:

```rust
impl EarlyLintPass for FooFunctions {
    fn check_fn(&mut self, cx: &EarlyContext<'_>, fn_kind: FnKind<'_>, span: Span, _: NodeId) {
        // Imagine a `is_foo_function` that checks for functions named `foo`
        if is_foo_function(fn_kind) {
            span_lint_and_help(
                cx,
                FOO_FUNCTIONS,
                span,
                "function named `foo`",
                None,
                "consider using a more meaningful name"
            );
        }
    }
}
```

## Run UI Tests to Emit the Lint

Now, if we run our [UI test](write_tests.md), we should see that the compiler now
produce output that contains the lint message we designed.

The next step is to implement the logic properly, which is a detail that we will
cover in the next chapters.

[diagnostics]: https://doc.rust-lang.org/nightly/nightly-rustc/clippy_utils/diagnostics/index.html
[early_check_fn]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.EarlyLintPass.html#method.check_fn
[early_lint_pass]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.EarlyLintPass.html
[late_check_expr]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LateLintPass.html#method.check_expr
[late_check_fn]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LateLintPass.html#method.check_fn
[late_check_item]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LateLintPass.html#method.check_item
[late_lint_pass]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LateLintPass.html
[rust_expressions]: https://doc.rust-lang.org/reference/expressions.html
[span_lint_and_help]: https://doc.rust-lang.org/nightly/nightly-rustc/clippy_utils/diagnostics/fn.span_lint_and_help.html
[span_lint_and_sugg]: https://doc.rust-lang.org/nightly/nightly-rustc/clippy_utils/diagnostics/fn.span_lint_and_sugg.html
[the rustc-dev-guide]: https://rustc-dev-guide.rust-lang.org/diagnostics.html
