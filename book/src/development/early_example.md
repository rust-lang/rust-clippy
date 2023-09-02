# Example with `EarlyLintPass`

Let us create a Clippy lint that implements `EarlyLintPass`
so that we can put the knowledge from all previous chapters into practice. We'll implement this
example in an **early** lint, but nowadays, late lints are the standard and the recommended for
new lints.

> Note: This lint is actually implemented in Clippy.
> If you are curious, feel free to check out the actual implementation
> of this lint example here: [`empty_structs_with_brackets`].

- [Example with `EarlyLintPass`](#example-with-earlylintpass)
  - [The Problem: Empty structs with brackets](#the-problem-empty-structs-with-brackets)
  - [Define the Lint](#define-the-lint)
  - [Write UI Tests](#write-ui-tests)
  - [Implement the Lint](#implement-the-lint)
    - [Implement `check_item` Method](#implement-check_item-method)
    - [Emit the Lint](#emit-the-lint)
    - [Run `cargo bless` to generate `.stderr` file](#run-cargo-bless-to-generate-stderr-file)
  - [Document the New Lint](#document-the-new-lint)

## The Problem: Empty structs with brackets

The usage of `struct <Name> {};` is very common in Rust projects.
However, these brackets (`{}`) don't serve any function if the struct is
empty, so, for legibility’s sake, we could remove them.

So, we're going to create this lint that warns you about empty structs with brackets.

## Define the Lint

Since we do not need any type information (just access to the
[AST][ast]), we will implement `EarlyLintPass` for this lint.

Let's name it `empty_structs_with_brackets`, which suggests
what this lint is aimed to do. Check the
[lint naming conventions][naming_conventions] for more info about naming.

Additionally, since this lint warns about useless syntax that should not be there in any case, we'll choose the
`style` lint group. (Not the group choosen for this lint, but still).

With these decisions at hand, we could generate the new lint by running
the following command inside our Clippy directory:

```sh
$ git checkout -b new_lint_empty_struct_with_brackets

$ cargo dev new_lint --name=empty_struct_with_brackets --pass=early --category=style

    Finished dev [unoptimized + debuginfo] target(s) in 0.11s
     Running `target/debug/clippy_dev new_lint --name=pub_use --pass=early --category=restriction`
Generated lint file: `clippy_lints/src/empty_struct_with_brackets.rs`
Generated test file: `tests/ui/empty_struct_with_brackets.rs`

NOTE: Use a late pass unless you need something specific from an early pass, as they lack many features and utilities
```

Let's take a look at all the changes that are happening:

```sh
$ git status
On branch new_lint_pub_use

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
	modified:   CHANGELOG.md
	modified:   clippy_lints/src/declared_lints.rs
	modified:   clippy_lints/src/lib.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
	clippy_lints/src/empty_struct_with_brackets.rs
	tests/ui/empty_struct_with_brackets.rs

no changes added to commit (use "git add" and/or "git commit -a")
```

Feel free to experiment with the command and check what the `cargo dev new_lint`
command did for us. Most importantly, it has done
quite a bit of the heavy-lifting, and we have a
`clippy_lints/src/empty_struct_with_brackets.rs` file to implement
the lint logic as well as a `tests/ui/empty_struct_with_brackets.rs`
file to implement the (UI) tests in.

## Write UI Tests

Let's safely remove the stub content of the newly created UI test file:

```rust
// tests/ui/empty_struct_with_brackets.rs

#![warn(clippy::empty_struct_with_brackets)]

fn main() {
    // test code goes here
}

```

Since this is a relatively simple and straightforward lint, let us
simply put some undesirable code that we want to lint and some code
we don't want linted inside this file so that we have some
positive and negative test cases:

```rust
// tests/ui/empty_struct_with_brackets.rs

#![warn(clippy::empty_struct_with_brackets)]
#![allow(unused_imports)]
#![no_main]

// This should lint
struct MyBadStruct {};

// This should not lint
struct MyGoodStruct;
struct MyGoodStruct2 { a_field: usize };

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
    pub EMPTY_STRUCT_WITH_BRACKETS,
    style,
    "default lint description"
}
declare_lint_pass!(PubUse => [PUB_USE]);

impl EarlyLintPass for PubUse {}
```

### Implement `check_item` Method

Since we want to check the usage of `struct <Name> {};`, we could utilize
`EarlyLintPass`'s [check_item] method, which gives us an item of type [Item]:

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
which is an enum that includes a variant `Struct`:

```rust
pub enum ItemKind {
    // [...]
    Enum(EnumDef, Generics),
    Struct(VariantData, Generics),
    Union(VariantData, Generics),
    // Other item kind variants...
}
```

Let's write some code down:

```rust
// ...Code above ignored

impl EarlyLintPass for PubUse {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        // We only check for an item if its kind is `ItemKind::Struct`
        // and it has no fields.
        if let ItemKind::Struct(variant_data, _) = item.kind &&
            !matches!(variant_data, VariantData::Unit(_)) && // Has brackets
            variant_data.fields().is_empty() // Doesn't have any fields
            {
                // Let's just print the line out
                println!("We found a struct with brackets but without any fields!");
        }
    }
}
```

At this point, if we run the UI test, Clippy will compile and throw
us some test failures:

```sh
$ TESTNAME=empty_struct_with_brackets cargo uitest

running 1 test
test [ui] ui/empty_struct_with_brackets.rs ... FAILED

...error messages...

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s

...error messages...

failures:

---- compile_test stdout ----
normalized stdout:
We found a struct with brackets but without any fields!

...error messages...

error: test failed, to rerun pass '--test compile-test'
```

But we see that our `println!` message is in the `stdout`!

### Emit the Lint

To emit a meaningful lint, we could use `span_lint_and_help`:

```rust
// ...Code above ignored

impl EarlyLintPass for PubUse {
    fn check_item(&mut self, cx: &EarlyContext<'_>, item: &Item) {
        // We only check for an item if its kind is `ItemKind::Struct`
        // and it has no fields.
        if let ItemKind::Struct(variant_data, _) = item.kind &&
            !matches!(variant_data, VariantData::Unit(_)) && // Has brackets
            variant_data.fields().is_empty() // Doesn't have any fields
            {
                // Let's just print the line out
                span_lint_and_help(
                    cx,
                    EMPTY_STRUCTS_WITH_BRACKETS,
                    item.span,
                    "this struct has no fields but brackets",
                    None,
                    "omit these brackets"
                )
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
error: found empty brackets on struct declaration
  --> $DIR/empty_structs_with_brackets.rs:4:25
   |
LL | pub struct MyEmptyStruct {} // should trigger lint
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `-D clippy::empty-structs-with-brackets` implied by `-D warnings`
   = help: remove the brackets
error: aborting due to previous error

...error messages...
```

### Run `cargo bless` to generate `.stderr` file

This looks very much like how we would want the lint to behave!
So we can run `cargo bless`:

```sh
$ cargo bless
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/clippy_dev bless`
updating tests/ui/empty_structs_with_brackets.stderr
```

Peeking into this newly generated `stderr` file we will see:

```txt
error: found empty brackets on struct declaration
  --> $DIR/empty_structs_with_brackets.rs:4:25
   |
LL | pub struct MyEmptyStruct {} // should trigger lint
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `-D clippy::empty-structs-with-brackets` implied by `-D warnings`
   = help: remove the brackets
error: aborting due to previous error
```

And running `TESTNAME=empty_structs_with_brackets cargo uitest` command again will give us
some `ok` messages:

```sh
$ TESTNAME=pub_use cargo uitest
    Finished test [unoptimized + debuginfo] target(s) in 0.06s
     Running tests/compile-test.rs (target/debug/deps/compile_test-d827993ccb35780b)

running 3 tests
test rustfix_coverage_known_exceptions_accuracy ... ok
test ui_cargo_toml_metadata ... ok

running 1 test
test [ui] ui/empty_structs_with_brackets.rs ... ok

...Lots of ok messages...
```

## Document the New Lint

Don't forget that we should document this new
`empty_structs_with_brackets` lint so that other Rustaceans
can easily understand what it does and if it fits their needs:

```rust
// clippy_lints/src/empty_structs_with_brackets.rs

// ...Code above ignored

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Finds structs without fields (a so-called
    /// "empty struct") that are declared with brackets.
    ///
    /// ### Why is this bad?
    ///
    /// Empty brackets after a struct declaration
    /// can be omitted.
    ///
    /// ### Example
    /// ```rust
    /// struct Cookie {}
    /// ```
    /// Use instead:
    /// ```rust
    /// struct Cookie;
    /// ```
    #[clippy::version = "1.62.0"] // Put the next version to your current one here
    pub EMPTY_STRUCTS_WITH_BRACKETS,
    style,
    "finds struct declarations with empty brackets"
}

// Code below ignored...
```

This looks about right! Now we can commit the code and push to our Github
branch and create a pull request for Clippy's `master` branch.

[check_item]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.EarlyLintPass.html#method.check_item
[Item]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/struct.Item.html
[ItemKind]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/enum.ItemKind.html
[ast]: https://rustc-dev-guide.rust-lang.org/syntax-intro.html
[naming_conventions]: https://rust-lang.github.io/rfcs/0344-conventions-galore.html#lints
