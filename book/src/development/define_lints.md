# Define New Lints

The first step in the journey of a new lint is to define the definition
and registration of the lint in Clippy's codebase.
We can use the Clippy dev tools to handle this step since setting up the
lint involves some boilerplate code.

- [Define New Lints](#define-new-lints)
	- [Lint Name](#lint-name)
	- [Add and Register Lint](#add-and-register-lint)
		- [Automated Command](#automated-command)
		- [Specific Lint Types](#specific-lint-types)
		- [Lint Registration](#lint-registration)
	- [Checklist](#checklist)
	- [Lint Types](#lint-types)

## Lint Name

A lint that checks function names and will trigger on all functions
named `foo` should be named as `foo_functions`.

As the [lint naming guidelines][lint_naming] suggests: If a lint applies
to a specific grammatical class, mention that class and use the plural form.
Therefore, `foo_function` would not be chosen.

Consult our [lint naming guidelines][lint_naming] for naming lints.
During the code review process, [Clippy team members][clippy_team_members]
will suggest a new name if the existing lint name is not a good fit.

## Add and Register Lint

Use `cargo dev new_lint` command to register a lint, e.g. `foo_functions`,
to the codebase.

### Automated Command

If you believe that this new lint is a standalone lint, you can run the following
command in your Clippy project:

```sh
cargo dev new_lint --name=foo_functions --pass=late --category=pedantic
```

There are two things to note here:

1. We set `--pass=late` in this command to do a late lint pass. The alternative
is an `early` lint pass. We will discuss this difference in [lint passes chapter](./lint_passes.md).
2. If not provided, the `category` of this new lint will default to `nursery`.
See Clippy's [lint groups](../lints.md) for more information on categories.

The `cargo dev new_lint` command will create a new file: `clippy_lints/src/foo_functions.rs`.

### Specific Lint Types

If you believe that this new lint belong to a specific type of lints,
you can run `cargo dev new_lint` with a `--type` option.

For instance, `foo_functions` lint is related to function calls,
so we can put it in the `functions` group by running:

```sh
cargo dev new_lint --name=foo_functions --type=functions --category=pedantic
```

This command will create, among other things, a new file:
`clippy_lints/src/{type}/foo_functions.rs`.
In our case, the path will be `clippy_lints/src/functions/foo_functions.rs`.

This way, a lint will be registered within the type's lint pass,
found in `clippy_lints/src/{type}/mod.rs`.

A _type_ is just the name of a directory in `clippy_lints/src`, like `functions` in
the example command. Clippy organizes some lints that share common behaviors under
the same directory, read more in the [Lint Types](#lint-types) section.

### Lint Registration

If we run the `cargo dev new_lint` command for a new lint,
the lint will be automatically registered and there is nothing more to do.

However, sometimes we might want to declare a new lint by hand.
In this case, we'd use `cargo dev update_lints` command.

When `cargo dev update_lints` is used, we might need to register the lint pass
manually in the `register_plugins` function in `clippy_lints/src/lib.rs`:

```rust
store.register_early_pass(|| Box::new(foo_functions::FooFunctions));
```

As you might have guessed: In Clippy there is also a `register_late_pass` method.
More on early vs. late passes in [lint passes chapter](./lint_passes.md).

Without a call to one of `register_early_pass` or `register_late_pass`,
the lint pass in question will not be run.

One reason that `cargo dev update_lints` does not automate this step is that
multiple lints might use the same lint pass, so registering the lint pass may
have already been done when adding a new lint.

Another reason for not automating this step is that the order
that the passes are registered determines the order the passes actually run,
which in turn affects the order that any emitted lints are output in.

## Checklist

All steps to define and register lints include:

- Choose a lint name
- Check if the lint is standalone or can be added to a type of lints
- Determine the category for the lint, e.g. `style`, `pedantic`, etc.
- Determine whether to use `EarlyLintPass` or `LateLintPass` (see [Lint Passes](./lint_passes.md))
- Run `cargo dev new_lint` to automate lint registration
- Add documentation (see [Add Documentation](./add_documentation.md))

## Lint Types

As of the writing of this documentation update, there are 11 categories (a.k.a. _types_)
of lints besides the numerous standalone lints living under `clippy_lints/src/`:

- `cargo`: Lints related specifically to `cargo`
- `casts`: Lints related to type conversion
- `functions`: Lints related to functions calls
- `loops`: Lints related to loops and for loops
- `matches`: Lints related to match expressions and `if let` expressions
- `methods`: Lints related to method calls
- `misc_early`: Miscellaneous lints related to `EarlyLintPass`
- `operators`: Lints related to operators such as `+`, `-`, `*`, etc.
- `transmute`: Lints related to transmute
- `types`: Lints related to type definitions in code
- `unit_types`: Lints related to unit values and unit types

For more information, feel free to compare the lint files under any category
with [All Clippy lints][all_lints] or ask one of the maintainers.

[all_lints]: https://rust-lang.github.io/rust-clippy/master/
[lint_naming]: https://rust-lang.github.io/rfcs/0344-conventions-galore.html#lints
[clippy_team_members]: https://www.rust-lang.org/governance/teams/dev-tools#Clippy%20team
