# Define New Lints

The first step in the journey of a new lint is to define the definition
and registration of the lint in Clippy's codebase.
We can use the Clippy dev tools to handle this step since setting up the 
lint involves some boilerplate code.

## Name the Lint

A wise software engineer Phil Karlton once said:
> There are only two hard things in Computer Science: cache invalidation and naming things.

Naming a lint is no exception.
Therefore, in case of uncertainty about if the name you chose fits the lint,
please do the following:

1. Consult our [lint naming guidelines][lint_naming]
2. Ping a [Clippy team member][clippy_team_members] in our [Zulip] chat
3. Comment on the corresponding Github issue (less preferrable as comments might be overlooked)

For now, let's imagine that your fellow developers at work tend to define some of their
functions with the name `foo` and forget to re-name it to a more meaningful name
when they submit a pull request.

`foo` is a highly non-descriptive name for a function, so we want to detect this
bad naming and fix it early on in the development process.

For this, we will create a lint that detects these `foo` functions and
help our fellow developers correct this bad practice. Note that in Clippy,
lints are generally written in snake cases.
We can name this new lint `foo_functions`.

## Add and Register the Lint

Now that a name is chosen, we shall register `foo_functions` as a lint to the codebase.
There are two ways to register a lint.

### Standalone

If you believe that this new lint is a standalone lint, you can run the following
command in your Clippy project:

```sh
$ cargo dev new_lint --name=foo_functions --pass=late --category=pedantic
```

There are two things to note here:

1. We set `--pass=late` in this command to do a late lint pass. The alternative
is an `early` lint pass. We will discuss this difference in a later chapter.
2. If not provided, the `category` of this new lint will default to `nursery`.
See Clippy's [lint groups](../lints.md) for more information on categories.

The `cargo dev new_lint` command will create a new file: `clippy_lints/src/foo_functions.rs`
as well as [register the lint](#lint-registration).

Overall, you should notice that the following files are modified or created:

```sh
$ git status
On branch foo_functions
Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
	modified:   CHANGELOG.md
	modified:   clippy_lints/src/lib.register_lints.rs
	modified:   clippy_lints/src/lib.register_pedantic.rs
	modified:   clippy_lints/src/lib.rs
	modified:   src/docs.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
	clippy_lints/src/foo_functions.rs
	src/docs/foo_functions.txt
	tests/ui/foo_functions.rs
```

### Specific Type

If you believe that this new lint belong to a specific type of lints,
you can run `cargo dev new_lint` with a `--type` option.

Since our `foo_functions` lint is related to function calls, one could
argue that we should put it into a group of lints that detect some behaviors
of functions, we can put it in the `functions` group.

Let's run the following command in your Clippy project:

```sh
$ cargo dev new_lint --name=foo_functions --type=functions --category=pedantic
```

This command will create, among other things, a new file:
`clippy_lints/src/{type}/foo_functions.rs`.
In our case, the path will be `clippy_lints/src/functions/foo_functions.rs`.

Notice how this command has a `--type` flag instead of `--pass`. Unlike a standalone
definition, this lint won't be registered in the traditional sense. Instead, you will
call your lint from within the type's lint pass, found in `clippy_lints/src/{type}/mod.rs`.

A _type_ is just the name of a directory in `clippy_lints/src`, like `functions` in
the example command. Clippy groups together some lints that share common behaviors,
so if your lint falls into one, it would be best to add it to that type.
Read more about [lint groups](#lint-groups) below.

Overall, you should notice that the following files are modified or created:

```sh
$ git status
On branch foo_functions
Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
	modified:   CHANGELOG.md
	modified:   clippy_lints/src/functions/mod.rs
	modified:   clippy_lints/src/lib.register_lints.rs
	modified:   clippy_lints/src/lib.register_pedantic.rs
	modified:   src/docs.rs

Untracked files:
  (use "git add <file>..." to include in what will be committed)
	clippy_lints/src/functions/foo_functions.rs
	src/docs/foo_functions.txt
	tests/ui/foo_functions.rs
```

## Lint registration

If we run the `cargo dev new_lint` command for a new lint,
the lint will be automatically registered and there is nothing more to do.

However, sometimes we might want to declare a new lint by hand.
In this case, we'd use `cargo dev update_lints` command.

When `cargo dev update_lints` is used, we might need to register the lint pass
manually in the `register_plugins` function in `clippy_lints/src/lib.rs`:

```rust
store.register_early_pass(|| Box::new(foo_functions::FooFunctions));
```

As you might have guessed, where there's something early, there is something late:
in Clippy there is a `register_late_pass` method as well.
More on early vs. late passes in a later chapter.

Without a call to one of `register_early_pass` or `register_late_pass`,
the lint pass in question will not be run.

One reason that `cargo dev update_lints` does not automate this step is that
multiple lints might use the same lint pass, so registering the lint pass may
have already been done when adding a new lint.

Another reason for not automating this step is that the order
that the passes are registered determines the order the passes actually run,
which in turn affects the order that any emitted lints are output in.

## Lint groups

As of the writing of this documentation update, there are 11 categories (a.k.a. _types_)
of lints besides the numerous standalone lints living under `clippy_lints/src/`:

- `cargo`
- `casts`
- `functions`
- `loops`
- `matches`
- `methods`
- `misc_early`
- `operators`
- `transmute`
- `types`
- `unit_types`

These categories group together lints that share some common behaviors.
For instance, as we have mentioned earlier, `functions` groups together lints
that deal with some aspects of function calls in Rust.

Some other common categories are `loops` and `methods`. `loops` group is for
lints that involve `for` loops, `while` loops, `range` loops, etc.
`methods` group is for lints that target method calls.

For more information, feel free to compare the lint files under any category
with [All Clippy lints][all_lints] or
ask one of the maintainers.

[all_lints]: https://rust-lang.github.io/rust-clippy/master/
[lint_naming]: https://rust-lang.github.io/rfcs/0344-conventions-galore.html#lints
[clippy_team_members]: https://www.rust-lang.org/governance/teams/dev-tools#Clippy%20team
[Zulip]: https://rust-lang.zulipchat.com/#narrow/stream/257328-clippy
