# Modifying Clippy behavior with attributes

In some cases it is possible extend Clippy coverage to include 3rd party libraries.
At this moment, only one such modification is possible: adding a
`#[clippy::format_args]` attribute to a macro that supports `format!`-like syntax.

## `#[clippy::format_args]`

This attribute can be added to a macro that supports `format!`-like syntax.
It tells Clippy that the macro is a formatting macro, and that the arguments to the macro
should be linted as if they were arguments to `format!`. Any lint that would apply to a
`format!` call will also apply to the macro call. The macro may have additional arguments
before the format string, and these will be ignored.

### Example

Note that the `#[clippy::format_args]` is only available in v1.84, and will
cause an `error: usage of unknown attribute` when running older `clippy`.
To avoid this, you can use the [rustversion](https://github.com/dtolnay/rustversion)
crate to apply the attribute conditionally.

```rust
/// A macro that prints a message if a condition is true
#[macro_export]
#[rustversion::attr(since(1.84), clippy::format_args)]
macro_rules! print_if {
    ($condition:expr, $($args:tt)+) => {{
        if $condition {
            println!($($args)+)
        }
    }};
}
```
