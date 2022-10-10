# Example with `EarlyLintPass`

Let us create a Clippy lint that implements `EarlyLintPass`
so that we can put the knowledge from all previous chapters into practice.

> Note: This lint is actually implemented in Clippy.
> If you are curious, feel free to check out the actual implementation
> of this lint example here: [pub_use].

- [Example with `EarlyLintPass`](#example-with-earlylintpass)
  - [The Problem: Pub Use](#the-problem-pub-use)
  - [Define the Lint](#define-the-lint)
  - [Write UI Tests](#write-ui-tests)
  - [Implement the Lint](#implement-the-lint)
    - [Implement `check_item` Method](#implement-check_item-method)
    - [Emit the Lint](#emit-the-lint)
    - [Run `cargo dev bless` to Generate `.stderr` file](#run-cargo-dev-bless-to-generate-stderr-file)
  - [Document the New Lint](#document-the-new-lint)

## The Problem: Pub Use

The usage of `pub use ...` is extremely common in Rust projects.
However, for certain projects we might want to restrict writing
`pub use ...` to prevent unintentional exports or to encourage
the developers to place exported items explicitly in public modules.

For instance, we might often write the following module export:

```rust
pub mod outer {
    mod inner {
        pub struct Test {}
    }
    pub use inner::Test;  // <- `pub use` instance
}

use outer::Test;
```

Instead, for some very specific projects, we want to encourage
the following approach instead:

```rust
pub mod outer {
    pub struct Test {}
}

use outer::Test;
```

## Define the Lint

Since we only need to detect the usage of `pub use` and do not need
any type information, we could implement `EarlyLintPass` for this lint.

Let us name it `pub_use`, which suggests explicitly what this lint
is aimed to do.

Additionally, since restricting `pub use ...` is for very specific cases,
we probably want to create a lint but put it into `clippy::restriction`
group, which contains lints that, when enabled, disallow Rustaceans
from writing certain code.

With these decisions at hand, we could generate the new lint by running
the following command inside our Clippy directory:

```sh
$ git checkout -b new_lint_pub_use

$ cargo dev new_lint --name=pub_use --pass=early --category=restriction

    Finished dev [unoptimized + debuginfo] target(s) in 0.11s
     Running `target/debug/clippy_dev new_lint --name=pub_use --pass=early --category=restriction`
Generated lint file: `clippy_lints/src/pub_use.rs`
Generated test file: `tests/ui/pub_use.rs`
```

Let us take a look at all the changes that are happening:

```sh
$ git status
On branch new_lint_pub_use

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
	modified:   CHANGELOG.md
	modified:   clippy_lints/src/lib.register_lints.rs
	modified:   clippy_lints/src/lib.register_restriction.rs
	modified:   clippy_lints/src/lib.rs
	modified:   src/docs.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
	clippy_lints/src/pub_use.rs
	src/docs/pub_use.txt
	tests/ui/pub_use.rs

no changes added to commit (use "git add" and/or "git commit -a")
```

Feel free to experiment with the command and check what the `cargo dev new_lint`
command did for us. Most importantly, it has done quite a bit of heavy-lifting
and we have a `clippy_lints/src/pub_use.rs` file to implement the lint logic
as well as a `tests/ui/pub_use.rs` file to implement the UI tests in.

## Write UI Tests

Let us safely remove the stub content of the newly created UI test file:

```rust
// tests/ui/pub_use.rs

#![warn(clippy::pub_use)]

fn main() {
    // test code goes here
}

```

Since this is a relatively simple and straightforward lint, let us
simply put some undesirable code that we want to lint and some code
which we do not want to lint inside this file so that we have some
positive and negative test cases:

```rust
// tests/ui/pub_use.rs

#![warn(clippy::pub_use)]
#![allow(unused_imports)]
#![no_main]

pub mod outer {
    mod inner {
        pub struct Test {}
    }
    // should be linted
    pub use inner::Test;
}

// should not be linted
use std::fmt;
```

## Implement the Lint

We also have a stub lint file:

```rust
use rustc_ast::ast::*;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```rust
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```rust
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.65.0"]
    pub PUB_USE,
    restriction,
    "default lint description"
}
declare_lint_pass!(PubUse => [PUB_USE]);

impl EarlyLintPass for PubUse {}
```

### Implement `check_item` Method

Since we want to check the usage of `pub use`, we could utilize
`EarlyLintPass`'s [check_item] method, which gives us an item of [Item]:

```rust
pub struct Item<K = ItemKind> {
    pub attrs: AttrVec,
    pub id: NodeId,
    pub span: Span,
    pub vis: Visibility,
    pub ident: Ident,
    pub kind: K,
    pub tokens: Option<LazyAttrTokenStream>,
}
```

Note that this `Item` contains information on the item's [ItemKind],
which is an enum that includes a variant `Use`:

```rust
pub enum ItemKind {
    ExternCrate(Option<Symbol>),
    Use(UseTree),
    // Other item kind variants...
}
```

Moreover, the `Item` struct includes a `vis` field of [Visibility]
which contains `kind` for [VisibilityKind], which includes the
`VisibilityKind::Public` variant for indicating the usage of `pub use`.

Let us write some code down:

```rust
// ...Code above ignored

impl EarlyLintPass for PubUse {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        // We only check for an item if its kind is `ItemKind::Use`
        // and if its visibility is `VisibilityKind::Public`
        if let ItemKind::Use(_) = item.kind &&
            let VisibilityKind::Public = item.vis.kind {
                // Let's just print the line out
                println!("We found a `pub use` item!");
            }
    }
}
```

At this point, if we run the UI test, Clippy will compile and throw
us some test failures:

```sh
$ TESTNAME=pub_use cargo uitest

running 1 test
test [ui] ui/pub_use.rs ... FAILED

...error messages...

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s

...error messages...

failures:

---- compile_test stdout ----
normalized stdout:
We found a `pub use` item!

...error messages...

error: test failed, to rerun pass '--test compile-test'
```

But we see that our `println!` message is in the `stdout`!

### Emit the Lint

To emit a meaningful lint, we could use use `span_lint_and_help`:

```rust
// ...Code above ignored

impl EarlyLintPass for PubUse {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        if let ItemKind::Use(_) = item.kind &&
            let VisibilityKind::Public = item.vis.kind {
                // Emit the lint with `span_lint_and_help`
                span_lint_and_help(
                    cx,
                    PUB_USE,
                    item.span,
                    "using `pub use`",
                    None,
                    "move the exported item to a public module instead",
                );
            }
    }
}
```

If we run the UI test again, we will observe some output like:

```sh
...error messages...

failures:

---- compile_test stdout ----
normalized stderr:
error: using `pub use`
  --> $DIR/pub_use.rs:10:5
   |
LL |     pub use inner::Test;
   |     ^^^^^^^^^^^^^^^^^^^^
   |
   = note: `-D clippy::pub-use` implied by `-D warnings`
   = help: move the exported item to a public module instead

error: aborting due to previous error

...error messages...
```

### Run `cargo dev bless` to Generate `.stderr` file

This looks very much like how we would want the lint to behave!
So we can run `cargo dev bless`:

```sh
$ cargo dev bless
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/clippy_dev bless`
updating tests/ui/pub_use.stderr
```

Peeking into this newly generated `stderr` file we will see:

```txt
error: using `pub use`
  --> $DIR/pub_use.rs:10:5
   |
LL |     pub use inner::Test;
   |     ^^^^^^^^^^^^^^^^^^^^
   |
   = note: `-D clippy::pub-use` implied by `-D warnings`
   = help: move the exported item to a public module instead

error: aborting due to previous error
```

And running `TESTNAME=pub_use cargo uitest` command again will give us
some `ok` messages:

```sh
$ TESTNAME=pub_use cargo uitest
    Finished test [unoptimized + debuginfo] target(s) in 0.06s
     Running tests/compile-test.rs (target/debug/deps/compile_test-d827993ccb35780b)

running 3 tests
test rustfix_coverage_known_exceptions_accuracy ... ok
test ui_cargo_toml_metadata ... ok

running 1 test
test [ui] ui/pub_use.rs ... ok

...Lots of ok messages...
```

## Document the New Lint

Don't forget that we should document this new `pub_use` lint
so that other Rustaceans can easily understand what it does and if
it fits their needs:

```rust
// clippy_lints/src/pub_use.rs

// ...Code above ignored

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Restricts the usage of `pub use ...`
    ///
    /// ### Why is this bad?
    ///
    /// `pub use` is usually fine, but a project may wish to limit `pub use` instances to prevent
    /// unintentional exports or to encourage placing exported items directly in public modules
    ///
    /// ### Example
    /// ```rust
    /// pub mod outer {
    ///     mod inner {
    ///         pub struct Test {}
    ///     }
    ///     pub use inner::Test;
    /// }
    ///
    /// use outer::Test;
    /// ```
    /// Use instead:
    /// ```rust
    /// pub mod outer {
    ///     pub struct Test {}
    /// }
    ///
    /// use outer::Test;
    /// ```
    #[clippy::version = "1.62.0"]
    pub PUB_USE,
    restriction,
    "restricts the usage of `pub use`"
}

// Code below ignored...
```

This looks about right! Now we can commit the code and push to our Github
branch and create a pull request for Clippy's `master` branch.

[check_item]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.EarlyLintPass.html#method.check_item
[Item]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/struct.Item.html
[ItemKind]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/enum.ItemKind.html
[pub_use]: https://github.com/rust-lang/rust-clippy/blob/cf72565a12c982f577ca4394c3b80edb89f6c6d3/clippy_lints/src/pub_use.rs
[Visibility]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/struct.Visibility.html
[VisibilityKind]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/struct.Visibility.html
