# Testing

Developing lints for Clippy is a Test-Driven Development (TDD)
process because our first task before implementing any logic for
a new lint is to write some test cases.

Writing tests first help Clippy developers to find a balance for the
first iteration of and further enhancements for a lint.
With test cases, we will not have to worry about over-engineering a lint
on its first version nor missing out some obvious edge cases of the lint.

## Clippy UI Tests

In Clippy, we use **UI tests** for testing lint behaviors.
These UI tests check that the output of Clippy is exactly as we expect it to be.
Each test is just a plain Rust file that contains the code we want to check.

The output of Clippy is compared against a `.stderr` file.
Note that you don't have to create this file yourself.
We'll get to generating the `.stderr` files with the command `cargo dev bless` later on.

### Write Test Cases

For a `foo_functions` lint that detects functions with `foo` as their name,
we start by opening the test file `tests/ui/foo_functions.rs` that was created by
the `cargo dev new_lint` command for adding a new lint.

Update the file with some positive and negative examples to get started:

```rust
#![allow(unused)]
#![warn(clippy::foo_functions)]

// Impl methods
struct A;
impl A {
    pub fn fo(&self) {}
    pub fn foo(&self) {} // Should lint
    pub fn food(&self) {}
}

// Default trait methods
trait B {
    fn fo(&self) {}
    fn foo(&self) {} // Should lint
    fn food(&self) {}
}

// Plain functions
fn fo() {}
fn foo() {} // Should lint
fn food() {}

fn main() {
    foo();
    let a = A;
    a.foo();
}
```

Without actual lint logic to emit the lint when we see a `foo` function name,
these tests are still quite meaningless.
However, we can now run the test with the following command:

```sh
$ TESTNAME=foo_functions cargo uitest
```

Clippy will compile and it will conclude with an `ok` for the tests:

```
...Clippy warnings and test outputs...

test compile_test ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
```

This is normal. After all, we have not implemented any logic for Clippy to
detect `foo` functions and emit a lint.

As we gradually implement our lint logic, we will keep running and improving UI tests
untilthe output turns into what we want it to be.

> _Note:_ You can run multiple test files by specifying a comma separated list:
> `TESTNAME=foo_functions,bar_methods,baz_structs`.

### `cargo dev bless`

Once we are satisfied with the lint output, we run the following command to generate
or update the `.stderr` file for our lint:

```sh
$ TESTNAME=foo_functions cargo uitest
$ cargo dev bless
```

This will format the `.stderr` file to include the emitted lint suggestions and
fixes to the test file, with the reason for the lint, suggested fixes, and
line numbers, etc.

> _Note:_ we should run `TESTNAME=foo_functions cargo uitest` every time before we run
> `cargo dev bless`.

Running `TESTNAME=foo_functions cargo uitest` should pass then. When we
commit our lint, we need to commit the generated `.stderr` files, too.

In general, you should only commit files changed by `cargo dev bless` for the
specific lint you are creating/editing.

> _Note:_ If the generated `.stderr`, `.txt` and `.fixed` files are empty,
> they should be removed.

## Cargo Lints

The process of testing is different for Cargo lints in that now we are
interested in the `Cargo.toml` manifest file.
In this case, we also need a minimal crate associated with that manifest.

For an imaginary new lint named `foo_categories`, we can run:

```sh
$ cargo dev new_lint --name=foo_categories --pass=late --category=cargo
```

After running `cargo dev new_lint` we will find by default two new crates,
each with its manifest file:

* `tests/ui-cargo/foo_categories/fail/Cargo.toml`: this file should cause the
  new lint to raise an error.
* `tests/ui-cargo/foo_categories/pass/Cargo.toml`: this file should not trigger
  the lint.

If you need more cases, you can copy one of those crates (under `foo_categories`) and rename it.

The process of generating the `.stderr` file is the same as for other lints 
and prepending the `TESTNAME` variable to `cargo uitest` works for Cargo lints too.

Overall, you should see the following changes when you generate a new Cargo lint:

```sh
$ git status
On branch foo_categories
Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
	modified:   CHANGELOG.md
	modified:   clippy_lints/src/cargo/mod.rs
	modified:   clippy_lints/src/lib.register_cargo.rs
	modified:   clippy_lints/src/lib.register_lints.rs
	modified:   src/docs.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
	clippy_lints/src/cargo/foo_categories.rs
	src/docs/foo_categories.txt
	tests/ui-cargo/foo_categories/
```

## Rustfix Tests

If the lint you are working on is making use of structured suggestions, the test
file should include a `// run-rustfix` comment at the top.

What are structured suggestions? They are suggestions that tell a user how to
fix or re-write certain code that has been linted.

The `// run-rustfix` comment will additionally run [rustfix] for our test.
Rustfix will apply the suggestions from the lint to the code of the test file and
compare that to the contents of a `.fixed` file.

Use `cargo dev bless` to automatically generate the `.fixed` file after running the tests.

## Testing Manually

Manually testing against an example file can be useful if you have added some
`println!`s and the test suite output becomes unreadable.

To try Clippy with your local modifications, run from the working copy root.

```sh
$ cargo dev lint input.rs
```

[rustfix]: https://github.com/rust-lang/rustfix
