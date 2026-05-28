# Attributes for Crate Authors

In some cases it is possible to extend Clippy coverage to 3rd party libraries.
To do this, Clippy provides attributes that can be applied to items in the 3rd party crate.

## `#[clippy::format_args]`

_Available since Clippy v1.85_

This attribute can be added to a macro that supports `format!`, `println!`, or similar syntax.
It tells Clippy that the macro is a formatting macro, and that the arguments to the macro
should be linted as if they were arguments to `format!`. Any lint that would apply to a
`format!` call will also apply to the macro call. The macro may have additional arguments
before the format string, and these will be ignored.

### Example

```rust
/// A macro that prints a message if a condition is true.
#[macro_export]
#[clippy::format_args]
macro_rules! print_if {
    ($condition:expr, $($args:tt)+) => {{
        if $condition {
            println!($($args)+)
        }
    }};
}
```

## `#[clippy::has_significant_drop]`

_Available since Clippy v1.60_

The `clippy::has_significant_drop` attribute can be added to types whose Drop impls have an important side effect,
such as unlocking a mutex, making it important for users to be able to accurately understand their lifetimes.
When a temporary is returned in a function call in a match scrutinee, its lifetime lasts until the end of the match
block, which may be surprising.

### Example

```rust
#[clippy::has_significant_drop]
struct CounterWrapper<'a> {
    counter: &'a Counter,
}

impl<'a> Drop for CounterWrapper<'a> {
    fn drop(&mut self) {
        self.counter.i.fetch_sub(1, Ordering::Relaxed);
    }
}
```

## `#[clippy::avoid_eager_arguments]`

_Available since Clippy v1.XX_

The `clippy::avoid_eager_arguments` attribute can be added to functions to discourage users from passing in arguments
that are evaluated eagerly (via a function call or closure). This is useful in cases where a function may exist in
different "variants" that accept either eager or lazy arguments, like `Option`'s `.unwrap_or` vs `.unwrap_or_else`.

Library authors should provide a hint as part of the attribute, to inform users of the recommended solution.

### Example

```rust
#[clippy::avoid_eager_arguments = "Prefer using `lazy_foo` instead of eagerly evaluating `argument`."]
fn foo(argument: String) {
    // Perform some logic that only rarely involves the use of `argument`
}

fn lazy_foo<F>(argument: F)
where
    F: FnOnce() -> String
{
    /// Perform some logic and evaluate `argument` on the fly, only if it is needed.
}
```
