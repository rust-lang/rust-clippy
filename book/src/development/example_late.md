# Example with `LateLintPass`

Let us create a Clippy lint that implements `LateLintPass`
so that we can put the knowledge from all previous chapters into practice.

> Note: This lint is actually implemented in Clippy.
> If you are curious, feel free to check out the actual implementation
> of this lint example here: [trim_split_whitespace].

- [Example with `LateLintPass`](#example-with-latelintpass)
  - [The Problem: Unnecessary Whitespace Trimming](#the-problem-unnecessary-whitespace-trimming)
  - [Define the Lint](#define-the-lint)
    - [Diagnostic Items](#diagnostic-items)
    - [Generate New Lint Files with `LateLintPass`](#generate-new-lint-files-with-latelintpass)
  - [Understand HIR with `#[clippy::dump]`](#understand-hir-with-clippydump)
  - [Write UI Tests](#write-ui-tests)
    - [Match for Method Calls](#match-for-method-calls)
    - [Edge Cases for UI Tests](#edge-cases-for-ui-tests)

## The Problem: Unnecessary Whitespace Trimming

Have you met a cautious Rustacean who are obsessed with leading
and trailing whitespaces in their `String`/`str` that they would
perform [trim]/[trim_start]/[trim_end] before [split_whitespace]:

```rust
" A B C ".trim().split_whitespace();
```

No judgement! It is okay to be extra careful. But as it turns out,
`split_whitespace` would already ignore leading and trailing whitespaces.
So our careful Rustacean could have written instead the following code
(go to this [playground] for an example):

```rust
" A B C ".split_whitespace();
```

This presents a perfect opportunity for a lint that could inform
other cautious Rustaceans that they only need to perform one method
call if that call is `split_whitespace`.

Let us get started!

## Define the Lint

As the reader might intuitively sense, we are not just interacting
with Rust "grammar" if we want to implement this lint.

### Diagnostic Items

We need to somehow ascertain that we are sequentially calling the
`trim`/`trim_start`/`trim_end` and `split_whitespace` methods and
this would require us to access [diagnostic items][diagnostic_items].

Diagnostic items are our way to check for specific types, traits and,
in our present case, functions/methods.

> Note: Take a quick look at [Using Diagnostic Items][using_diagnostic_items]
> if you want to have an overview of how they work.

### Generate New Lint Files with `LateLintPass`

Because of this, we are going to use [LateLintPass] for this new lint.
Since this lint will examine chained method calls, it is quite convenient
to implement the [check_expr] method for `LateLintPass` when working on
this new lint.

Additionally, we could name our lint `trim_split_whitespace`, which is
an accurate description of the unnecessary method call.

Moreover, since this lint is more related to writing idiomatic Rust,
we can consider putting it in the `clippy::style` group.

Let us run our beloved `cargo dev new_lint` command to generate some files:

```sh
$ git checkout -b new_lint_trim_split_whitespace

$ cargo dev new_lint --name=trim_split_whitespace --pass=late --category=style
    Finished dev [unoptimized + debuginfo] target(s) in 0.07s
     Running `target/debug/clippy_dev new_lint --name=trim_split_whitespace --pass=late --category=style`
Generated lint file: `clippy_lints/src/trim_split_whitespace.rs`
Generated test file: `tests/ui/trim_split_whitespace.rs`
```

Examine the changes with `git status` carefully, we see that most of the important
files have been created by the command so that we can focus on developing the
new lint:

```sh
$ git status
On branch new_lint_trim_split_whitespace

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
	modified:   CHANGELOG.md
	modified:   clippy_lints/src/lib.register_all.rs
	modified:   clippy_lints/src/lib.register_lints.rs
	modified:   clippy_lints/src/lib.register_style.rs
	modified:   clippy_lints/src/lib.rs
	modified:   src/docs.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
	clippy_lints/src/trim_split_whitespace.rs
	src/docs/trim_split_whitespace.txt
	tests/ui/trim_split_whitespace.rs

no changes added to commit (use "git add" and/or "git commit -a")
```

Feel free to experiment with the command and check what the `cargo dev new_lint`
command did for us. Most importantly, it has done quite a bit of heavy-lifting
and we have a `clippy_lints/src/trim_split_whitespace.rs` file to implement the lint logic as well as a `tests/ui/trim_split_whitespace.rs` file to implement the UI tests in.

## Understand HIR with `#[clippy::dump]`

Before we move further with our UI test cases as well as the lint implementation,
it is important to note that it is helpful to first understand the internal
representation that rustc uses. Clippy has the `#[clippy::dump]` attribute that
prints the [_High-Level Intermediate Representation (HIR)_] of the item,
statement, or expression that the attribute is attached to.

> Note: If you have not read other chapters of the Clippy book and wonder
> what HIR is in the context of `rustc`, feel free to read [_Rustc Overview_]
> as well as [_High-Level Intermediate Representation (HIR)_] for a quick review.

To attach the attribute to expressions you often need to enable
`#![feature(stmt_expr_attributes)]`.

[Here][print_hir_example] you can find an example. To see how an expression like
`" A ".trim().split_whitespace()` looks like in HIR,
select _TOOLS_ from the upper right corner of Rust Playground and click _Clippy_.
Overall, you should see a structure that resembles the following:

```rust
Expr {
    hir_id: HirId { // fields },
    kind: MethodCall(
        PathSegment {
            ident: split_whitespace#0,
            hir_id: HirId { // fields },
            // fields
        },
        Expr {
            hir_id: HirId { // fields },
            kind: MethodCall(
                PathSegment {
                    ident: trim#0,
                    hir_id: HirId { // fields },
                    // fields
                },
                Expr {
                    hir_id: HirId { // fields },
                    kind: Lit(
                        Spanned {
                            node: Str(
                                " A ",
                                Cooked,
                            ),
                            span: src/main.rs:5:13: 5:18 (#0),
                        },
                    ),
                    span: src/main.rs:5:13: 5:18 (#0),
                },
                [],
                src/main.rs:5:19: 5:25 (#0),
            ),
            span: src/main.rs:5:13: 5:25 (#0),
        },
        [],
        src/main.rs:5:26: 5:44 (#0),
    ),
    span: src/main.rs:5:13: 5:44 (#0),
}
```

It might take some getting used to but, if we observe carefully, we
see that there are 2 [MethodCall]s, one for `split_whitespace`, where
the `PathSegment.ident` is `split_whitespace#0`, and one for `trim`,
where the `PathSegment.ident` is `trim#0`.
We might also notice an expression `Expr` whose `kind` is [Lit] with
a `Spanned { node: Str(" A "), ..}`.

In essence, when an expression has chained method calls like:

```rust
" A ".trim().split_whitespace()
```

Its HIR representation would look like the following, with the last
method `split_whitespace` being the most outer element, the method
`trim` being the second most outer element, and the method call receiver
`" A "` being the most inner element.

```rust
// Note that this is an extremely simplified pseudo-HIR structure
Expr {
    kind: MethodCall(
        split_whitespace,             // This is the last method call
        Expr {
            kind: MethodCall(
                trim,                 // This is the first method call
                Expr {
                    kind: Lit(" A ")  // This is the receiver of the first method call
                }
            )
        }
    )
}
```

Let us keep this information in our head as we move on to the next steps
in writing a new Clippy lint.

## Write UI Tests

With some knowledge of the kind of expression we are working with,
let us write down some cases which we expect the lint to warn.
The following could be a reasonable starting point for us:

```rust
// tests/ui/trim_split_whitespace.rs

#![warn(clippy::trim_split_whitespace)]

fn main() {
    // &str
    let _ = " A B C ".trim().split_whitespace(); // should trigger lint
    let _ = " A B C ".trim_start().split_whitespace(); // should trigger lint
    let _ = " A B C ".trim_end().split_whitespace(); // should trigger lint

    // String
    let _ = (" A B C ").to_string().trim().split_whitespace(); // should trigger lint
    let _ = (" A B C ").to_string().trim_start().split_whitespace(); // should trigger lint
    let _ = (" A B C ").to_string().trim_end().split_whitespace(); // should trigger lint

}
```

### Match for Method Calls

Assuming that we will implement [check_expr]
for `LateLintPass` when implementing this lint and assuming that
we will examine an expression `expr` for `" A ".trim().split_whitespace()`,
we can check for the usage of `trim` and `split_whitespace` like the following:

```rust
// We first match for 
if let ExprKind::MethodCall(path, [split_recv], split_ws_span) = expr.kind
    && path.ident.name == sym!(split_whitespace)
    && let ExprKind::MethodCall(path, [_trim_recv], trim_span) = split_recv.kind
    && let trim_fn_name @ ("trim" | "trim_start" | "trim_end") = path.ident.name.as_str()
    && let Some(trim_def_id) = tyckres.type_dependent_def_id(split_recv.hir_id) {
        span_lint_and_sugg(
            cx,
            TRIM_SPLIT_WHITESPACES,
            trim_span.with_hi(split_ws_span.lo()),
            &format!("found call to `str::{}` before `str::split_whitespace`", trim_fn_name),
            &format!("remove `{}()`", trim_fn_name),
            String::new(),
            Applicability::MachineApplicable,
        );
}
```

### Edge Cases for UI Tests

[check_expr]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html#method.check_expr
[diagnostic_items]: https://rustc-dev-guide.rust-lang.org/diagnostics/diagnostic-items.html
[_Rustc Overview_]: https://rustc-dev-guide.rust-lang.org/overview.html
[_High-Level Intermediate Representation (HIR)_]: https://rustc-dev-guide.rust-lang.org/hir.html
[LateLintPass]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html
[Lit]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_hir/hir/enum.ExprKind.html#variant.Lit
[MethodCall]: https://doc.rust-lang.org/stable/nightly-rustc/rustc_hir/hir/enum.ExprKind.html#variant.MethodCall
[playground]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=62c07194aada06aecb7c600123fd5786
[print_hir_example]: https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=a32d1fa3bfc35607db75e226e5fe475b
[split_whitespace]: https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace
[trim]: https://doc.rust-lang.org/std/primitive.str.html#method.trim
[trim_end]: https://doc.rust-lang.org/std/primitive.str.html#method.trim_end
[trim_start]: https://doc.rust-lang.org/std/primitive.str.html#method.trim_start
[trim_split_whitespace]: https://github.com/rust-lang/rust-clippy/blob/9a6eca5f852830cb5e9a520f79ce02e6aae9a1b1/clippy_lints/src/strings.rs#L464-L517
[using_diagnostic_items]: https://rustc-dev-guide.rust-lang.org/diagnostics/diagnostic-items.html#using-diagnostic-items
